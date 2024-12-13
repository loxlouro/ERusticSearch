use config::{Config as ConfigLib, Environment, File};
use serde::{Deserialize, Serialize};
use std::net::IpAddr;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: IpAddr,
    pub port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    pub data_file: String,
    pub index_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub server: ServerConfig,
    pub storage: StorageConfig,
}

impl Config {
    pub fn load() -> anyhow::Result<Self> {
        let builder = ConfigLib::builder()
            .add_source(File::with_name("config/default"))
            .add_source(Environment::with_prefix("APP"));

        Ok(builder.build()?.try_deserialize()?)
    }
}
