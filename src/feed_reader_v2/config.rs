use serde::Deserialize;
use std::collections::HashMap;
use std::io::Read;
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
    pub conditional_type: Option<ConditionalType>,
    pub download_dir: String,
}

fn default_feed_poll_interval_secs() -> u64 {
    14400
}

#[derive(Deserialize, Debug, PartialEq)]
pub enum ConditionalType {
    ETag,
    LastModified,
}

impl Config {
    fn from_reader<R: Read>(mut reader: R) -> Result<Self, Error> {
        let mut toml_contents = String::new();
        reader.read_to_string(&mut toml_contents)?;
        let config: Config = toml::from_str(&toml_contents)?;

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_from_reader_defaults() {
        let buf = String::from("
[feeds.test]
url = \"https://example.com/rss\"
download_dir = \"/tmp/test\"
        ");

        let config = Config::from_reader(buf.as_bytes()).expect("failed to parse configuration");
        assert_eq!(config.feeds["test"].url, "https://example.com/rss");
        assert_eq!(config.feeds["test"].download_dir, "/tmp/test");
        assert_eq!(config.feeds["test"].conditional_type, None);
        assert_eq!(config.feeds["test"].poll_interval_secs, 14400);
    }

    #[test]
    fn config_from_reader_custom_etag() {
        let buf = String::from("
[feeds.test]
url = \"https://example.com/rss\"
download_dir = \"/tmp/test\"
poll_interval_secs = 3601
conditional_type = \"ETag\"
        ");

        let config = Config::from_reader(buf.as_bytes()).expect("failed to parse configuration");
        assert_eq!(config.feeds["test"].url, "https://example.com/rss");
        assert_eq!(config.feeds["test"].download_dir, "/tmp/test");
        assert_eq!(config.feeds["test"].conditional_type, Some(ConditionalType::ETag));
        assert_eq!(config.feeds["test"].poll_interval_secs, 3601);
    }

    #[test]
    fn config_from_reader_custom_last_modified() {
        let buf = String::from("
[feeds.test]
url = \"https://example.com/rss\"
download_dir = \"/tmp/test\"
poll_interval_secs = 3601
conditional_type = \"LastModified\"
        ");

        let config = Config::from_reader(buf.as_bytes()).expect("failed to parse configuration");
        assert_eq!(config.feeds["test"].url, "https://example.com/rss");
        assert_eq!(config.feeds["test"].download_dir, "/tmp/test");
        assert_eq!(config.feeds["test"].conditional_type, Some(ConditionalType::LastModified));
        assert_eq!(config.feeds["test"].poll_interval_secs, 3601);
    }

}
