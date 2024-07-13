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
}

#[derive(Deserialize)]
pub struct Config {
    pub feeds: HashMap<String, Feed>,
}

#[derive(Deserialize)]
pub struct Feed {
    pub url: String,
}

impl TryFrom<PathBuf> for Config {
    type Error = Error;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let f = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&f)?;
        Ok(config)
    }
}
