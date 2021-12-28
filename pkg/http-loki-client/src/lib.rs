use anyhow::anyhow;
use generic_loki_client::{Direction, LabelResponse, LokiClient, LokiError, Response, SerieResponse};
use async_trait::async_trait;

pub struct HttpLokiClient {
    url: String,
}

impl HttpLokiClient {
    pub fn new(url: String) -> Self {
        HttpLokiClient { url }
    }
}

#[async_trait]
impl LokiClient for HttpLokiClient {
    //Query loki api using reqwest asynchronously
    async fn query(&self, query: String, limit: Option<i32>, time: Option<i64>, direction: Option<Direction>) -> Result<Response, LokiError> {
        let mut url = self.url.clone();
        url.push_str("/loki/api/v1/query");
        let mut params = vec![("query", query)];
        if let Some(limit) = limit {
            params.push(("limit", limit.to_string()));
        }
        if let Some(time) = time {
            params.push(("time", time.to_string()));
        }
        if let Some(direction) = direction {
            params.push(("direction", match direction { Direction::Forward => "forward".to_string(), Direction::Backward => "backward".to_string() }));
        }
        let client = reqwest::Client::new();

        let result = client.get(&url).query(&params).send().await.or_else(|e| {
            Err(LokiError::Other(anyhow!("{}", e)))
        });

        if let Ok(result) = result {
            let body = result.text().await.or_else(|e| {
                Err(LokiError::Other(anyhow!("{}", e)))
            });
            if let Ok(body) = body {
                let response = serde_json::from_str(&body).or_else(|e| {
                    Err(LokiError::Other(anyhow!("{}", e)))
                });
                if let Ok(response) = response {
                    Ok(response)
                } else {
                    response
                }
            } else {
                Err(LokiError::Other(anyhow!("Failed to parse response body")))
            }
        } else {
            Err(LokiError::Other(anyhow!("Failed to query loki")))
        }
    }

    async fn query_range(&self, query: String, start: i64, end: i64, limit: Option<i32>, direction: Option<Direction>, step: Option<String>, interval: Option<String>) -> Result<Response, LokiError> {
        let mut url = self.url.clone();
        url.push_str("/loki/api/v1/query_range");
        let mut params = vec![("query", query), ("start", start.to_string()), ("end", end.to_string())];
        if let Some(limit) = limit {
            params.push(("limit", limit.to_string()));
        }
        if let Some(direction) = direction {
            params.push(("direction", match direction { Direction::Forward => "forward".to_string(), Direction::Backward => "backward".to_string() }));
        }
        if let Some(step) = step {
            params.push(("step", step));
        }
        if let Some(interval) = interval {
            params.push(("interval", interval));
        }
        let client = reqwest::Client::new();

        let result = client.get(&url).query(&params).send().await.or_else(|e| {
            Err(LokiError::Other(anyhow!("{}", e)))
        });

        if let Ok(result) = result {
            let body = result.text().await.or_else(|e| {
                Err(LokiError::Other(anyhow!("{}", e)))
            });
            if let Ok(body) = body {
                let response = serde_json::from_str(&body).or_else(|e| {
                    Err(LokiError::Other(anyhow!("{}", e)))
                });
                if let Ok(response) = response {
                    Ok(response)
                } else {
                    response
                }
            } else {
                Err(LokiError::Other(anyhow!("Failed to parse response body")))
            }
        } else {
            Err(LokiError::Other(anyhow!("Failed to query loki")))
        }
    }

    async fn labels(&self, start: Option<i64>, end: Option<i64>) -> Result<LabelResponse, LokiError> {
        let mut url = self.url.clone();
        url.push_str("/loki/api/v1/labels");
        let mut params = vec![];
        if let Some(start) = start {
            params.push(("start", start.to_string()));
        }
        if let Some(end) = end {
            params.push(("end", end.to_string()));
        }
        let client = reqwest::Client::new();

        let result = client.get(&url).query(&params).send().await.or_else(|e| {
            Err(LokiError::Other(anyhow!("{}", e)))
        });

        if let Ok(result) = result {
            let body = result.text().await.or_else(|e| {
                Err(LokiError::Other(anyhow!("{}", e)))
            });
            if let Ok(body) = body {
                let response = serde_json::from_str(&body).or_else(|e| {
                    Err(LokiError::Other(anyhow!("{}", e)))
                });
                if let Ok(response) = response {
                    Ok(response)
                } else {
                    response
                }
            } else {
                Err(LokiError::Other(anyhow!("Failed to parse response body")))
            }
        } else {
            Err(LokiError::Other(anyhow!("Failed to query loki")))
        }
    }

    async fn label_values(&self, label: String, start: Option<i64>, end: Option<i64>) -> Result<LabelResponse, LokiError> {
        let mut url = self.url.clone();
        url.push_str("/loki/api/v1/label/");
        url.push_str(&label);
        url.push_str("/values");
        let mut params = vec![];
        if let Some(start) = start {
            params.push(("start", start.to_string()));
        }
        if let Some(end) = end {
            params.push(("end", end.to_string()));
        }
        let client = reqwest::Client::new();

        let result = client.get(&url).query(&params).send().await.or_else(|e| {
            Err(LokiError::Other(anyhow!("{}", e)))
        });

        if let Ok(result) = result {
            let body = result.text().await.or_else(|e| {
                Err(LokiError::Other(anyhow!("{}", e)))
            });
            if let Ok(body) = body {
                let response = serde_json::from_str(&body).or_else(|e| {
                    Err(LokiError::Other(anyhow!("{}", e)))
                });
                if let Ok(response) = response {
                    Ok(response)
                } else {
                    response
                }
            } else {
                Err(LokiError::Other(anyhow!("Failed to parse response body")))
            }
        } else {
            Err(LokiError::Other(anyhow!("Failed to query loki")))
        }
    }

    async fn series(&self, matches: Option<Vec<String>>, start: Option<i64>, end: Option<i64>) -> Result<SerieResponse, LokiError> {
        let mut url = self.url.clone();
        url.push_str("/loki/api/v1/series");
        let mut params = vec![];
        if let Some(matches) = matches {
            matches.iter().for_each(|m| {
                params.push(("match[]", m.to_string()));
            });
        }
        if let Some(start) = start {
            params.push(("start", start.to_string()));
        }
        if let Some(end) = end {
            params.push(("end", end.to_string()));
        }
        let client = reqwest::Client::new();

        let result = client.post(&url).form(&params).send().await.or_else(|e| {
            Err(LokiError::Other(anyhow!("{}", e)))
        });

        if let Ok(result) = result {
            let body = result.text().await.or_else(|e| {
                Err(LokiError::Other(anyhow!("{}", e)))
            });
            if let Ok(body) = body {
                let response = serde_json::from_str(&body).or_else(|e| {
                    Err(LokiError::Other(anyhow!("{}", e)))
                });
                if let Ok(response) = response {
                    Ok(response)
                } else {
                    response
                }
            } else {
                Err(LokiError::Other(anyhow!("Failed to parse response body")))
            }
        } else {
            Err(LokiError::Other(anyhow!("Failed to query loki")))
        }
    }
}