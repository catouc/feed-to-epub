use crate::feed_reader_v2::config::Config;
use crate::storage::Storage;
use feed_rs::model::Feed;
use thiserror::Error;
use ureq::Agent;

pub mod config;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Error, Debug)]
pub enum FetchError {
    #[error("failed to parse feed XML: {0}")]
    FeedParseError(#[from] feed_rs::parser::ParseFeedError),
    #[error("storage create error: {0}")]
    StorageCreateError(#[from] crate::storage::ErrorNew),
    #[error("storage error: {0}")]
    StorageDBOperationError(#[from] crate::storage::ErrorDBOperation),
    #[error("failed to execute HTTP request: {0}")]
    HTTPError(#[from] ureq::Error), 
}

pub struct FeedReader {
    agent: Agent,
    storage: Storage,
    config: Config,
}

impl FeedReader {
    fn new(config: Config) -> Result<Self, crate::storage::ErrorNew> {
        let storage = match Storage::new(&config.db_file) {
            Ok(storage) => storage,
            Err(err) => return Err(err),
        };

        let agent = ureq::AgentBuilder::new()
            .user_agent(&format!(
                "feed-to-epub {}; +https:/github.com/catouc/feed-to-epub",
                VERSION
            ))
            .timeout(std::time::Duration::from_secs(
                config.http_request_timeout_secs,
            ))
            .build();

        Ok(FeedReader { agent, db, config })
    }

    fn fetch_all(&self) -> Result<Vec<Feed>, FetchError> {
        todo!()
    }

    fn fetch_feed(&self) -> Result<Feed, FetchError> {
        todo!()
    }
}
