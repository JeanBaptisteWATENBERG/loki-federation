use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use async_trait::async_trait;
use thiserror::Error;

#[derive(Deserialize, Serialize, Debug, Clone)]
pub enum ResultType {
    #[serde(alias = "vector")]
    Vector,
    #[serde(alias = "streams")]
    Streams
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct VectorOrStream {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metric: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<(i64, String)>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stream: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<(String, String)>>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Data {
    pub result_type: ResultType,
    pub result: Vec<VectorOrStream>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct Response {
    pub status: String,
    pub data: Data,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct LabelResponse {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Vec<String>>,
}

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct SerieResponse {
    pub status: String,
    pub data: Vec<HashMap<String, String>>,
}

#[derive(Deserialize, Serialize, Debug, Copy, Clone)]
pub enum Direction {
    Forward,
    Backward
}

#[derive(Error, Debug)]
pub enum LokiError {
    #[error("Not yet implemented")]
    NotImplemented,
    #[error("No data")]
    NoData,
    #[error(transparent)]
    Other(#[from] anyhow::Error)
}

#[async_trait]
pub trait LokiClient {
    async fn query(&self, query: String, limit: Option<i32>, time: Option<i64>, direction: Option<Direction>) -> Result<Response, LokiError>;
    async fn query_range(&self, query: String, start: i64, end: i64, limit: Option<i32>, direction: Option<Direction>, step: Option<String>, interval: Option<String>) -> Result<Response, LokiError>;
    async fn labels(&self, start: Option<i64>, end: Option<i64>) -> Result<LabelResponse, LokiError>;
    async fn label_values(&self, label: String, start: Option<i64>, end: Option<i64>) -> Result<LabelResponse, LokiError>;
    async fn series(&self, matches: Option<Vec<String>>, start: Option<i64>, end: Option<i64>) -> Result<SerieResponse, LokiError>;
}