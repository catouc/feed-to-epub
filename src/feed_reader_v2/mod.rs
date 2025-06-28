use crate::feed_reader_v2::config::Config;
use feed_rs::model::Feed;
use rusqlite::Connection;
use thiserror::Error;
use ureq::Agent;

pub mod config;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Error, Debug)]
pub enum CreationError {
    #[error("failed to open database file {db_file}: {source}")]
    DBFileOpenError {
        db_file: String,
        source: rusqlite::Error,
    },
}

#[derive(Error, Debug)]
pub enum FetchError {}

pub struct FeedReader {
    agent: Agent,
    db: Connection,
    config: Config,
}

impl FeedReader {
    fn new(config: Config) -> Result<Self, CreationError> {
        let db = match Connection::open(&config.db_file) {
            Ok(db) => db,
            Err(err) => {
                return Err(CreationError::DBFileOpenError {
                    db_file: config.db_file.clone(),
                    source: err,
                })
            }
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
