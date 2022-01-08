use generic_loki_client::{LokiError, LokiClient};
use http_loki_client::HttpLokiClient;
#[cfg(not(test))]
use anyhow::Error;
#[cfg(not(test))]
use log::{error, info};
use grpc_loki_client::GrpcLokiClient;
#[cfg(test)]
use mockall::{automock, predicate::*};
#[cfg(not(test))]
use crate::config::Datasources;

#[derive(Debug, Clone)]
pub struct HttpDataSource {
    pub url: String,
}

#[derive(Debug, Clone)]
pub struct GrpcDataSource {
    pub url: String,
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

#[cfg_attr(test, automock)]
impl DataSourceInstance {
    pub fn new(data_source: DataSource) -> Self {
        Self {
            data_source
        }
    }
    pub fn get_client(&self) -> Result<Box<dyn LokiClient>, LokiError> {
        match self.data_source {
            DataSource::HttpDataSource(ref http_data_source) => {
                let client = HttpLokiClient::new(http_data_source.url.clone());
                Ok(Box::new(client))
            }
            DataSource::GrpcDataSource(ref grpc_data_source) => {
                let client = GrpcLokiClient::new(grpc_data_source.url.clone());
                Ok(Box::new(client))
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct DataSourcesProvider {
    #[cfg(not(test))]
    data_sources_config: Datasources
}

#[cfg_attr(test, automock)]
impl DataSourcesProvider {
    #[cfg(test)]
    pub fn new() -> Self {
        Self {
        }
    }

    #[cfg(not(test))]
    pub fn new(data_sources_config: Datasources) -> Self {
        Self {
            data_sources_config
        }
    }

    #[cfg(test)]
    pub fn get_data_sources(&self) -> Result<Vec<MockDataSourceInstance>, LokiError> {
        todo!("Deliberately no implemenented, only for testing")
    }

    #[cfg(not(test))]
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
            "static-grpc-alpha" => {
                let urls_option = self.data_sources_config.urls.clone();
                if urls_option.is_none() {
                    return Err(LokiError::Other(Error::msg("static-grpc requires urls")));
                }

                let urls = urls_option.unwrap();

                info!("Using static urls {}", urls.join(", "));
                return Ok(urls.iter().map(|url| {
                    DataSourceInstance::new(DataSource::GrpcDataSource(GrpcDataSource {
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