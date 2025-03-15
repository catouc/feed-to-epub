use serde::Deserialize;
use std::collections::HashMap;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("could nor parse config file, invalid TOML: {0}")]
    TOMLParseError(#[from] toml::de::Error),
    #[error("database query failed: {0}")]
    FileError(#[from] std::io::Error),
    #[error("behave, the poll interval cannot be set below 1h: {:?}", feeds)]
    PollIntervalTooFastError { feeds: Vec<String> },
}

#[derive(Deserialize)]
pub struct Config {
    pub feeds: HashMap<String, Feed>,
    #[serde(default = "default_db_file")]
    pub db_file: String,
    #[serde(default = "default_http_request_timeout_secs")]
    pub http_request_timeout_secs: u64,
}

fn default_db_file() -> String {
    String::from("./feed-to-rss.db")
}

fn default_http_request_timeout_secs() -> u64 {
    15
}

#[derive(Deserialize)]
pub struct Feed {
    pub url: String,
    #[serde(default = "default_feed_poll_interval_secs")]
    pub poll_interval_secs: u64,
    pub conditional_type: ConditionalType,
    pub download_dir: String,
}

fn default_feed_poll_interval_secs() -> u64 {
    14400
}

#[derive(Deserialize)]
pub enum ConditionalType {
    ETag,
    LastModified,
}

impl TryFrom<PathBuf> for Config {
    type Error = Error;

    fn try_from(path: PathBuf) -> Result<Self, Self::Error> {
        let f = std::fs::read_to_string(path)?;
        let config: Config = toml::from_str(&f)?;

        let too_fast_feeds: Vec<String> = config
            .feeds
            .iter()
            .filter_map(|(name, feed)| {
                if feed.poll_interval_secs < 3600 {
                    Some(name.clone())
                } else {
                    None
                }
            })
            .collect();

        if !too_fast_feeds.is_empty() {
            return Err(Error::PollIntervalTooFastError {
                feeds: too_fast_feeds,
            });
        }

        Ok(config)
    }
}
