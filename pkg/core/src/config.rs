use serde::{Deserialize};

#[derive(Deserialize, Debug)]
pub struct ServerConfig {
    pub port: u16,
    pub bind_address: String,
}

#[cfg_attr(not(test), derive(Deserialize))]
#[derive(Debug, Clone)]
pub struct Datasources {
    pub name: String,
    pub urls: Option<Vec<String>>
}

#[derive(Deserialize, Debug)]
pub struct DebugConfig {
    pub log_level: String,
}

#[cfg_attr(not(test), derive(Deserialize))]
#[derive(Debug)]
pub struct Config {
    pub server: ServerConfig,
    pub datasources: Datasources,
    pub debug: DebugConfig,
}