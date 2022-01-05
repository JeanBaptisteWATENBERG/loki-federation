use std::collections::HashMap;
use anyhow::anyhow;
use generic_loki_client::{Data, Direction, LabelResponse, LokiClient, LokiError, Response, ResultType, SerieResponse, VectorOrStream};
use async_trait::async_trait;
use log::{error, info};
use prost_types::Timestamp;
use prometheus_labels_parser::{parse_labels, parse_labels_into_map};

pub mod grpc_loki_client {
    tonic::include_proto!("logproto");
}

pub struct GrpcLokiClient {
    url: String,
}

impl GrpcLokiClient {
    pub fn new(url: String) -> Self {
        GrpcLokiClient { url }
    }
}


pub fn from_unix_nano_timestamp(timestamp: i64) -> Timestamp {
    let mut ts = Timestamp {
        seconds: timestamp / 1_000_000_000,
        nanos: (timestamp % 1_000_000_000) as i32,
    };
    ts
}

#[async_trait]
impl LokiClient for GrpcLokiClient {
    //Query loki grpc api using tonic asynchronously
    async fn query(&self, query: String, limit: Option<i32>, time: Option<i64>, direction: Option<Direction>) -> Result<Response, LokiError> {
        info!("Query: {}", query);
        let mut client = grpc_loki_client::querier_client::QuerierClient::connect(self.url.clone()).await;
        match client {
            Ok(mut client) => {
                let request = grpc_loki_client::QueryRequest {//QuerySampleRequest doesn't support limit and direction, we then have to use QueryRequest instead with a 30 seconds timeframe
                    selector: query,
                    limit: limit.unwrap_or(100) as u32,//Todo the actual parameter could be a u32
                    start: if let Some(start) = time { Some(from_unix_nano_timestamp(start - 30_000_000_000)) } else { None },
                    end: if let Some(end) = time { Some(from_unix_nano_timestamp(end)) } else { None },
                    direction: match direction {
                        Some(Direction::Forward) => grpc_loki_client::Direction::Forward as i32,
                        Some(Direction::Backward) => grpc_loki_client::Direction::Backward as i32,
                        None => grpc_loki_client::Direction::Backward as i32,
                    },
                    shards: ::prost::alloc::vec::Vec::new(),
                };

                info!("Request, {:?}", request);

                let response: Result<tonic::Response<tonic::Streaming<grpc_loki_client::QueryResponse>>, tonic::Status> = client.query(request).await;

                match response {
                    Ok(response) => {
                        info!("Response received, {:?}", response);
                        let mut streaming_response: tonic::Streaming<grpc_loki_client::QueryResponse> = response.into_inner();
                        let message_result = streaming_response.message().await;

                        let result = match message_result {
                            Ok(message_option) => {
                                info!("Message option, {:?}", message_option);
                                match message_option {
                                    Some(message) => {
                                        let streams: ::prost::alloc::vec::Vec<grpc_loki_client::StreamAdapter> = message.streams;
                                        let mut result: Vec<VectorOrStream> = Vec::new();

                                        for stream in streams {
                                            let labels: ::prost::alloc::string::String = stream.labels;
                                            let entries: ::prost::alloc::vec::Vec<grpc_loki_client::EntryAdapter> = stream.entries;
                                            let mut vectors: Vec<(String, String)> = Vec::new();

                                            for entry in entries {
                                                let timestamp: Option<Timestamp> = entry.timestamp;
                                                let line: ::prost::alloc::string::String = entry.line;

                                                if timestamp.is_none() {
                                                    return Err(LokiError::Other(anyhow!("Timestamp is missing in the response")));
                                                }

                                                let nanos = format!("{:0>9}", &timestamp.as_ref().unwrap().nanos);

                                                vectors.push((timestamp.unwrap().seconds.to_string() + &nanos, line));
                                            }

                                            let parse_label_result = parse_labels_into_map(labels.to_string());

                                            match parse_label_result {
                                                Ok(labels) => {
                                                    result.push(VectorOrStream {
                                                        stream: Some(labels),
                                                        values: Some(vectors),
                                                        value: None,
                                                        metric: None,
                                                    })
                                                },
                                                Err(err) => {
                                                    return Err(LokiError::Other(anyhow!("Error while parsing labels: {}", err)));
                                                }
                                            }
                                        }
                                        Ok(result)
                                    }
                                    None => {
                                        return Err(LokiError::Other(anyhow!("No message in the response")));
                                    }
                                }
                            }
                            Err(error) => {
                                return Err(LokiError::Other(anyhow!("Error while receiving the response: {}", error)));
                            }
                        };

                        if let Err(error) = result {
                            return error;
                        }

                        Ok(Response {
                            status: "success".to_string(),
                            data: Data {
                                result_type: ResultType::Streams,
                                result: result.unwrap(),
                            },
                        })
                    }
                    Err(err) => Err(LokiError::Other(anyhow!(err))),
                }
            }
            Err(e) => {
                error!("Error while sending the request: {}", e);
                Err(LokiError::Other(anyhow!("Error while sending the request: {}", e)))
            },
        }
    }

    async fn query_range(&self, query: String, start: i64, end: i64, limit: Option<i32>, direction: Option<Direction>, step: Option<String>, interval: Option<String>) -> Result<Response, LokiError> {
        Err(LokiError::NotImplemented)
    }
    async fn labels(&self, start: Option<i64>, end: Option<i64>) -> Result<LabelResponse, LokiError> {
        Err(LokiError::NotImplemented)
    }
    async fn label_values(&self, label: String, start: Option<i64>, end: Option<i64>) -> Result<LabelResponse, LokiError> {
        Err(LokiError::NotImplemented)
    }
    async fn series(&self, matches: Option<Vec<String>>, start: Option<i64>, end: Option<i64>) -> Result<SerieResponse, LokiError> {
        Err(LokiError::NotImplemented)
    }
}
