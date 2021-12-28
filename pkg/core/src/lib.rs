use std::collections::HashSet;
use generic_loki_client::{LokiError, Response, LokiClient, VectorOrStream, Data, ResultType, LabelResponse};
use http_loki_client::HttpLokiClient;
use tokio::task;
use futures::future::join_all;
use futures::{stream, StreamExt};
use std::sync::Arc;
use std::pin::Pin;
use anyhow::Error;
use crate::aggregate::aggregate;
use serde::{Deserialize, Serialize};

mod aggregate;

#[derive(Debug, Clone)]
pub struct FederatedLoki {
    backends: Vec<String>,
}

#[derive(Deserialize, Debug)]
pub struct ServerConfig {
    pub port: u16,
    pub bind_address: String,
}

#[derive(Deserialize, Debug)]
pub struct Datasources {
    pub name: String,
    pub urls: Option<Vec<String>>
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub server: ServerConfig,
    pub datasources: Datasources,
}

#[derive(Deserialize, Serialize, Debug, Copy, Clone)]
pub enum Direction {
    #[serde(alias = "forward")]
    #[serde(alias = "FORWARD")]
    Forward,
    #[serde(alias = "backward")]
    #[serde(alias = "BACKWARD")]
    Backward
}

impl Direction {
    pub fn to_string(&self) -> String {
        match self {
            Direction::Forward => "forward".to_string(),
            Direction::Backward => "backward".to_string(),
        }
    }
    pub fn to_generic_loki_direction(&self) -> generic_loki_client::Direction {
        match self {
            Direction::Forward => generic_loki_client::Direction::Forward,
            Direction::Backward => generic_loki_client::Direction::Backward,
        }
    }
}


const MAX_CONCURRENT_REQUESTS: usize = 8;

impl FederatedLoki {
    pub fn new(backends: Vec<String>) -> Self {
        FederatedLoki {
            backends,
        }
    }

    pub async fn query(&self, query: String, limit: Option<i32>, time: Option<i64>, direction: Option<Direction>) -> Result<Response, LokiError> {
        let buffered_jobs = stream::iter(self.backends.clone())
            .map(|backend| {
                //TODO: find a way to inject HttpLokiClient and rely on GenericLokiClient instead
                let client = HttpLokiClient::new(backend.clone());
                let query = &query;
                async move {
                    let result = client.query(query.to_string(), limit, time, Some(direction.map_or(generic_loki_client::Direction::Backward, |direction| direction.to_generic_loki_direction()))).await;
                    result
                }
            }).buffer_unordered(MAX_CONCURRENT_REQUESTS).collect::<Vec<Result<Response, LokiError>>>();;

        let direction = direction.unwrap_or(Direction::Backward);

        let responses = buffered_jobs.await;

        let aggregatedResponse = Self::aggregate_responses(direction, responses);

        Ok(aggregatedResponse)
    }

    pub async fn query_range(&self, query: String, start: i64, end: i64, limit: Option<i32>, direction: Option<Direction>, step: Option<String>, interval: Option<String>) -> Result<Response, LokiError> {
        let buffered_jobs = stream::iter(self.backends.clone())
            .map(|backend| {
                //TODO: find a way to inject HttpLokiClient and rely on GenericLokiClient instead
                let client = HttpLokiClient::new(backend.clone());
                let query = &query;
                let step = step.clone();
                let interval = interval.clone();
                async move {
                    let result = client.query_range(query.to_string(), start, end, limit, Some(direction.map_or(generic_loki_client::Direction::Backward, |direction| direction.to_generic_loki_direction())), step, interval).await;
                    result
                }
            }).buffer_unordered(MAX_CONCURRENT_REQUESTS).collect::<Vec<Result<Response, LokiError>>>();

        let direction = direction.unwrap_or(Direction::Backward);

        let responses = buffered_jobs.await;

        let aggregatedResponse = Self::aggregate_responses(direction, responses);

        Ok(aggregatedResponse)
    }

    pub async fn labels(&self, start: Option<i64>, end: Option<i64>) -> Result<LabelResponse, LokiError> {
        let buffered_jobs = stream::iter(self.backends.clone())
            .map(|backend| {
                let client = HttpLokiClient::new(backend.clone());
                async move {
                    let result = client.labels(start, end).await;
                    result
                }
            }).buffer_unordered(MAX_CONCURRENT_REQUESTS).collect::<Vec<Result<LabelResponse, LokiError>>>();

        let responses: Vec<Result<LabelResponse, LokiError>> = buffered_jobs.await;

        let aggregatedLabelResponse = Self::merge_label_responses(responses);

        Ok(aggregatedLabelResponse)
    }

    pub async fn label_values(&self, label: String, start: Option<i64>, end: Option<i64>) -> Result<LabelResponse, LokiError> {
        let buffered_jobs = stream::iter(self.backends.clone())
            .map(|backend| {
                let client = HttpLokiClient::new(backend.clone());
                let label = &label;
                async move {
                    let result = client.label_values(label.to_string(), start, end).await;
                    result
                }
            }).buffer_unordered(MAX_CONCURRENT_REQUESTS).collect::<Vec<Result<LabelResponse, LokiError>>>();

        let responses: Vec<Result<LabelResponse, LokiError>> = buffered_jobs.await;

        let aggregatedLabelResponse = Self::merge_label_responses(responses);

        Ok(aggregatedLabelResponse)
    }

    fn merge_label_responses(responses: Vec<Result<LabelResponse, LokiError>>) -> LabelResponse {
        responses.into_iter().fold(LabelResponse { status: "success".to_string(), data: None }, |mut acc, response| {
            match response {
                Ok(response) => {
                    match response.data {
                        Some(data) => {
                            let some_acc_data = acc.data.unwrap_or(Vec::new());
                            let mut merged_data = some_acc_data.into_iter().chain(data.into_iter()).collect::<HashSet<String>>();
                            acc.data = Some(merged_data.into_iter().collect::<Vec<String>>());
                        }
                        None => {}
                    }
                    acc
                },
                Err(err) => acc // todo: log error
            }
        })
    }

    fn get_stream_data(stream: &VectorOrStream) -> Result<Vec<(i64, String)>, LokiError> {
        match &stream.values {
            Some(values) => {
                let mut result = Vec::new();
                for value in values {
                    let timestamp = value.0.clone();
                    let value = value.1.clone();
                    match timestamp.parse::<i64>() {
                        Ok(ts) => result.push((ts, value)),
                        Err(e) => return Err(LokiError::Other(Error::new(e)))
                    }
                }
                Ok(result)
            }
            None => Err(LokiError::NoData),//TODO: better error handling
        }
    }

    fn replace_stream_data(stream: &mut VectorOrStream, data: Vec<(i64, String)>) -> &mut VectorOrStream {
        stream.values = Some(data.iter().map(|(timestamp, value)| {
            (timestamp.to_string(), value.to_string())
        }).collect::<Vec<(String, String)>>());
        stream
    }

    fn aggregate_responses(direction: Direction, responses: Vec<Result<Response, LokiError>>) -> Response {
        let mut aggregatedResponse: Response = Response {
            data: Data {
                resultType: ResultType::streams,
                result: vec![],
            },
            status: "success".to_string(),
        };

        responses.iter().for_each(|result| {
            match result {
                Ok(response) => {
                    response.data.result.iter().for_each(|stream| {
                        let mut aggregatedStream = aggregatedResponse.data.result.iter_mut().find(|s| s.stream == stream.stream);//TODO: add replica-labels
                        match aggregatedStream {
                            Some(mut aggregatedStream) => {
                                let aggregatedData = Self::get_stream_data(&aggregatedStream).unwrap();
                                let currentData = Self::get_stream_data(stream).unwrap();
                                let mut merged = aggregate(aggregatedData, currentData, direction);
                                Self::replace_stream_data(&mut aggregatedStream, merged);
                            },
                            None => {
                                aggregatedResponse.data.result.push(stream.clone());
                            }
                        }
                    });
                },
                Err(e) => {},
            }
        });
        aggregatedResponse
    }
}

