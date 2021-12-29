use std::collections::{HashMap, HashSet};
use generic_loki_client::{LokiError, Response, LokiClient, VectorOrStream, Data, ResultType, LabelResponse, SerieResponse};
use http_loki_client::HttpLokiClient;
use futures::{stream, StreamExt};
use anyhow::Error;
use log::{error, info, warn};
use crate::aggregate::aggregate;
use serde::{Deserialize, Serialize};

mod aggregate;

#[derive(Deserialize, Debug)]
pub struct ServerConfig {
    pub port: u16,
    pub bind_address: String,
}

#[derive(Deserialize, Debug, Clone)]
pub struct Datasources {
    pub name: String,
    pub urls: Option<Vec<String>>
}

#[derive(Deserialize, Debug)]
pub struct DebugConfig {
    pub log_level: String,
}

#[derive(Deserialize, Debug)]
pub struct Config {
    pub server: ServerConfig,
    pub datasources: Datasources,
    pub debug: DebugConfig,
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

#[derive(Debug, Clone)]
pub struct HttpDataSource {
    url: String,
}

#[derive(Debug, Clone)]
pub struct GrpcDataSource {
    //todo, not yet supported
}

#[derive(Debug, Clone)]
pub enum DataSource {
    HttpDataSource(HttpDataSource),
    GrpcDataSource(GrpcDataSource),
}

#[derive(Debug, Clone)]
pub struct DataSourceInstance {
    data_source: DataSource,
}

impl DataSourceInstance {
    pub fn new(data_source: DataSource) -> Self {
        Self {
            data_source
        }
    }
    pub fn get_client(&self) -> Result<impl LokiClient, LokiError> {
        match self.data_source {
            DataSource::HttpDataSource(ref http_data_source) => {
                let client = HttpLokiClient::new(http_data_source.url.clone());
                Ok(client)
            }
            DataSource::GrpcDataSource(ref _grpc_data_source) => {
                //todo, not yet supported
                Err(LokiError::Other(Error::msg("grpcDataSource not yet supported")))
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct DataSourcesProvider {
    data_sources_config: Datasources
}

impl DataSourcesProvider {
    pub fn new(data_sources_config: Datasources) -> Self {
        Self {
            data_sources_config
        }
    }

    pub fn get_data_sources(&self) -> Result<Vec<DataSourceInstance>, LokiError> {
        match self.data_sources_config.name.as_str() {
            "static-http" => {
                let urls_option = self.data_sources_config.urls.clone();
                if urls_option.is_none() {
                    return Err(LokiError::Other(Error::msg("static-http requires urls")));
                }

                let urls = urls_option.unwrap();

                info!("Using static urls {}", urls.join(", "));
                return Ok(urls.iter().map(|url| {
                    DataSourceInstance::new(DataSource::HttpDataSource(HttpDataSource {
                        url: url.clone(),
                    }))
                }).collect());
            }
            _ => {
                error!("Unsupported datasource {}", self.data_sources_config.name);
                return Err(LokiError::Other(Error::msg("Unsupported datasource")));
            }
        }
    }
}

const MAX_CONCURRENT_REQUESTS: usize = 8;

#[derive(Debug, Clone)]
pub struct FederatedLoki {
    data_sources_provider: DataSourcesProvider,
}

impl FederatedLoki {
    pub fn new(data_sources_provider: DataSourcesProvider) -> Self {
        FederatedLoki {
            data_sources_provider,
        }
    }

    pub async fn query(&self, query: String, limit: Option<i32>, time: Option<i64>, direction: Option<Direction>) -> Result<Response, LokiError> {
        let data_sources_result = self.data_sources_provider.get_data_sources();
        if let Err(loki_error) = data_sources_result {
            return Err(loki_error);
        }

        let data_sources = data_sources_result.unwrap();

        let buffered_jobs = stream::iter(data_sources)
            .map(|data_source| {
                let client_result = data_source.get_client();

                let query = &query;
                async move {
                    if let Err(loki_error) = client_result {
                        return Err(loki_error);
                    }
                    let client = client_result.unwrap();
                    let result = client.query(query.to_string(), limit, time, Some(direction.map_or(generic_loki_client::Direction::Backward, |direction| direction.to_generic_loki_direction()))).await;
                    result
                }
            }).buffer_unordered(MAX_CONCURRENT_REQUESTS).collect::<Vec<Result<Response, LokiError>>>();

        let direction = direction.unwrap_or(Direction::Backward);

        let responses = buffered_jobs.await;

        let aggregated_response = Self::aggregate_responses(direction, responses);

        Ok(aggregated_response)
    }

    pub async fn query_range(&self, query: String, start: i64, end: i64, limit: Option<i32>, direction: Option<Direction>, step: Option<String>, interval: Option<String>) -> Result<Response, LokiError> {
        let data_sources_result = self.data_sources_provider.get_data_sources();
        if let Err(loki_error) = data_sources_result {
            return Err(loki_error);
        }

        let data_sources = data_sources_result.unwrap();

        let buffered_jobs = stream::iter(data_sources)
            .map(|data_source| {
                let client_result = data_source.get_client();

                let query = &query;
                let step = step.clone();
                let interval = interval.clone();
                async move {
                    if let Err(loki_error) = client_result {
                        return Err(loki_error);
                    }
                    let client = client_result.unwrap();
                    let result = client.query_range(query.to_string(), start, end, limit, Some(direction.map_or(generic_loki_client::Direction::Backward, |direction| direction.to_generic_loki_direction())), step, interval).await;
                    result
                }
            }).buffer_unordered(MAX_CONCURRENT_REQUESTS).collect::<Vec<Result<Response, LokiError>>>();

        let direction = direction.unwrap_or(Direction::Backward);

        let responses = buffered_jobs.await;

        let aggregated_response = Self::aggregate_responses(direction, responses);

        Ok(aggregated_response)
    }

    pub async fn labels(&self, start: Option<i64>, end: Option<i64>) -> Result<LabelResponse, LokiError> {
        let data_sources_result = self.data_sources_provider.get_data_sources();
        if let Err(loki_error) = data_sources_result {
            return Err(loki_error);
        }

        let data_sources = data_sources_result.unwrap();

        let buffered_jobs = stream::iter(data_sources)
            .map(|data_source| {
                let client_result = data_source.get_client();

                async move {
                    if let Err(loki_error) = client_result {
                        return Err(loki_error);
                    }
                    let client = client_result.unwrap();
                    let result = client.labels(start, end).await;
                    result
                }
            }).buffer_unordered(MAX_CONCURRENT_REQUESTS).collect::<Vec<Result<LabelResponse, LokiError>>>();

        let responses: Vec<Result<LabelResponse, LokiError>> = buffered_jobs.await;

        let aggregated_label_response = Self::merge_label_responses(responses);

        Ok(aggregated_label_response)
    }

    pub async fn label_values(&self, label: String, start: Option<i64>, end: Option<i64>) -> Result<LabelResponse, LokiError> {
        let data_sources_result = self.data_sources_provider.get_data_sources();
        if let Err(loki_error) = data_sources_result {
            return Err(loki_error);
        }

        let data_sources = data_sources_result.unwrap();

        let buffered_jobs = stream::iter(data_sources)
            .map(|data_source| {
                let client_result = data_source.get_client();

                let label = &label;
                async move {
                    if let Err(loki_error) = client_result {
                        return Err(loki_error);
                    }
                    let client = client_result.unwrap();
                    let result = client.label_values(label.to_string(), start, end).await;
                    result
                }
            }).buffer_unordered(MAX_CONCURRENT_REQUESTS).collect::<Vec<Result<LabelResponse, LokiError>>>();

        let responses: Vec<Result<LabelResponse, LokiError>> = buffered_jobs.await;

        let aggregated_label_response = Self::merge_label_responses(responses);

        Ok(aggregated_label_response)
    }

    pub async fn series(&self, matches: Option<Vec<String>>, start: Option<i64>, end: Option<i64>) -> Result<SerieResponse, LokiError> {
        let data_sources_result = self.data_sources_provider.get_data_sources();
        if let Err(loki_error) = data_sources_result {
            return Err(loki_error);
        }

        let data_sources = data_sources_result.unwrap();

        let buffered_jobs = stream::iter(data_sources)
            .map(|data_source| {
                let client_result = data_source.get_client();

                let matches = matches.clone();
                async move {
                    if let Err(loki_error) = client_result {
                        return Err(loki_error);
                    }
                    let client = client_result.unwrap();
                    let result = client.series(matches, start, end).await;
                    result
                }
            }).buffer_unordered(MAX_CONCURRENT_REQUESTS).collect::<Vec<Result<SerieResponse, LokiError>>>();

        let responses: Vec<Result<SerieResponse, LokiError>> = buffered_jobs.await;

        let aggregated_serie_response = Self::merge_serie_responses(responses);

        Ok(aggregated_serie_response)
    }

    fn merge_serie_responses(responses: Vec<Result<SerieResponse, LokiError>>) -> SerieResponse {
        let mut series_data: Vec<HashMap<String, String>> = Vec::new();

        for response in responses {
            match response {
                Ok(response) => {
                    response.data.iter().for_each(|serie| {
                        series_data.push(serie.clone());
                    });
                },
                Err(error) => {
                    warn!("One of the result contains an error: {}", error);
                }
            }
        }

        //remove duplicates in series data
        let mut already_seen_series = Vec::new();
        series_data.retain(|item| match already_seen_series.contains(item) {
            true => false,
            _ => {
                already_seen_series.push(item.clone());
                true
            }
        });

        SerieResponse {
            data: already_seen_series,
            status: "success".to_string(),
        }
    }

    fn merge_label_responses(responses: Vec<Result<LabelResponse, LokiError>>) -> LabelResponse {
        responses.into_iter().fold(LabelResponse { status: "success".to_string(), data: None }, |mut acc, response| {
            match response {
                Ok(response) => {
                    match response.data {
                        Some(data) => {
                            let some_acc_data = acc.data.unwrap_or(Vec::new());
                            let merged_data = some_acc_data.into_iter().chain(data.into_iter()).collect::<HashSet<String>>();
                            acc.data = Some(merged_data.into_iter().collect::<Vec<String>>());
                        }
                        None => {}
                    }
                    acc
                },
                Err(error) => {
                    warn!("One of the result contains an error: {}", error);
                    acc
                }
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
        let mut aggregated_response: Response = Response {
            data: Data {
                result_type: ResultType::Streams,
                result: vec![],
            },
            status: "success".to_string(),
        };

        responses.iter().for_each(|result| {
            match result {
                Ok(response) => {
                    response.data.result.iter().for_each(|stream| {
                        let aggregated_stream = aggregated_response.data.result.iter_mut().find(|s| s.stream == stream.stream);//TODO: add replica-labels
                        match aggregated_stream {
                            Some(mut aggregated_stream) => {
                                let aggregated_data = Self::get_stream_data(&aggregated_stream).unwrap();
                                let current_data = Self::get_stream_data(stream).unwrap();
                                let merged = aggregate(aggregated_data, current_data, direction);
                                Self::replace_stream_data(&mut aggregated_stream, merged);
                            },
                            None => {
                                aggregated_response.data.result.push(stream.clone());
                            }
                        }
                    });
                },
                Err(error) => {
                    warn!("One of the result contains an error: {}", error);
                },
            }
        });
        aggregated_response
    }
}

