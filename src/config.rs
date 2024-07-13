use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("could nor parse config file, invalid TOML")]
    TOMLParseError(#[from] toml::de::Error),
    #[error("database query failed")]
    FileError(#[from] std::io::Error),
    #[error("behave, the poll interval cannot be set below 1h")]
    PollIntervalTooFastError
}

#[derive(Deserialize)]
pub struct Config {
    pub feeds: HashMap<String, Feed>,
    #[serde(default = "default_poll_interval_secs")]
    pub poll_interval_secs: u64,
}

#[derive(Deserialize)]
pub struct Feed {
    pub url: String,
}

fn default_poll_interval_secs() -> u64 {
    14400
}

impl TryFrom<PathBuf> for Config {
    type Error = Error;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let f = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&f)?;

        if config.poll_interval_secs < 3600 {
            return Err(Error::PollIntervalTooFastError) 
        }

        Ok(config)
    }
}
