use crate::feed_reader::config::{ConditionalType, Config};
use crate::storage::Storage;
use feed_rs::model::Feed;
use jiff::tz::TimeZone;
use jiff::Timestamp;
use std::fs;
use thiserror::Error;
use ureq::Agent;

pub mod config;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Error, Debug)]
pub enum FetchError {
    #[error("failed to parse feed XML: {0}")]
    FeedParseError(#[from] feed_rs::parser::ParseFeedError),
    #[error("failed to convert feed_entry to storage entry: {0}")]
    EntryConversionError(#[from] crate::storage::EntryConversionError),
    #[error("storage create error: {0}")]
    StorageCreateError(#[from] crate::storage::ErrorNew),
    #[error("storage error: {0}")]
    StorageDBOperationError(#[from] crate::storage::ErrorDBOperation),
    #[error("storage error: {0}")]
    StorageNewFeedError(#[from] crate::storage::ErrorNewFeedStats),
    #[error("failed to execute HTTP request: {0}")]
    HTTPError(#[from] ureq::Error),
}

pub struct FeedReader {
    agent: Agent,
    storage: Storage,
    pub config: Config,
}

impl FeedReader {
    pub fn new(config: Config) -> Result<Self, crate::storage::ErrorNew> {
        let storage = Storage::new(&config.db_file)?;
        storage.init_database()?;

        let agent = ureq::AgentBuilder::new()
            .user_agent(&format!(
                "feed-to-epub {}; +https:/github.com/catouc/feed-to-epub",
                VERSION
            ))
            .timeout(std::time::Duration::from_secs(
                config.http_request_timeout_secs,
            ))
            .build();

        Ok(FeedReader {
            agent,
            storage,
            config,
        })
    }

    pub fn fetch_all(&self, now: Timestamp) -> Vec<Feed> {
        self.config
            .feeds
            .iter()
            .filter_map(|feed_stats| {
                let url = feed_stats.0;
                match self.fetch_feed(url, &feed_stats.1.download_dir, now) {
                    Ok(feed) => feed,
                    Err(err) => {
                        eprintln!("failed to fetch url {url}: {err}");
                        None
                    }
                }
            })
            .collect()
    }

    pub fn fetch_feed(
        &self,
        feed_name: &str,
        download_dir: &str,
        now: Timestamp,
    ) -> Result<Option<Feed>, FetchError> {
        let mut feed_stats = match self
            .storage
            .feed_stats_from_db(&self.config.feeds[feed_name].url)?
        {
            Some(feed_stats) => feed_stats,
            None => self
                .storage
                .new_feed_stats_to_db(&self.config.feeds[feed_name].url)?,
        };

        match fs::create_dir_all(download_dir) {
            Ok(_) => (),
            Err(err) => {
                eprintln!(
                    "failed to create download dir {} for feed {}: {}",
                    download_dir, feed_name, err,
                )
            }
        };

        if let Some(last_fetched) = feed_stats.last_fetched {
            let time_diff = now
                .to_zoned(TimeZone::UTC)
                .duration_since(&last_fetched.to_zoned(TimeZone::UTC));

            if time_diff.as_hours() < 2 {
                println!(
                    "{feed_name} was already fetched within the last two hours at {time_diff}."
                );
                return Ok(None);
            };
        };

        let mut request = self.agent.get(&self.config.feeds[feed_name].url);

        match &self.config.feeds[feed_name].conditional_type {
            ConditionalType::ETag => {
                if let Some(etag) = &feed_stats.etag {
                    request = request.set("ETag", etag);
                }
            }
            ConditionalType::LastModified => {
                // This is essentially only happening on the first time we ever fetch the feed
                if let Some(last_modified) = &feed_stats.last_modified {
                    request = request.set("If-Modified-Since", last_modified);
                }
            }
        };

        let response = request.call()?;
        let feed_data = match response.status() {
            304 => None,
            429 => {
                // TODO: I should add something to maybe a special table of feeds that have
                // been rate limited to then check every iteration on whether we've gone past
                // the `Retry-After` header expiry.
                eprintln!("{feed_name} got a 429 rate limit error");
                None
            }
            _ => {
                if let Some(last_modified_since) = response.header("Last-Modified") {
                    feed_stats.last_modified = Some(last_modified_since.into());
                }

                if let Some(etag) = response.header("ETag") {
                    feed_stats.etag = Some(etag.into());
                }

                let feed = feed_rs::parser::parse(response.into_reader())?;
                Some(feed)
            }
        };

        if let Some(feed) = feed_data {
            feed.entries
                .iter()
                .filter_map(|e| {
                    match crate::storage::entry_from_feed_entry(feed_stats.id, e) {
                        Ok(entry) => Some(entry),
                        Err(err) => {
                            eprintln!("{}", err);
                            None
                        } // TODO: we really shouldn't log the error here I think
                    }
                })
                .for_each(|e| match self.storage.new_entry_to_db(&e) {
                    Ok(_) => (),
                    Err(err) => eprintln!("{}", err),
                });

            feed_stats.last_fetched = Some(jiff::Timestamp::now());
            self.storage.feed_stats_to_db(&feed_stats)?;
            Ok(Some(feed))
        } else {
            // TODO: this is a fucking mess
            Err(FetchError::EntryConversionError(
                crate::storage::EntryConversionError::SummaryExtractionError,
            ))
        }
    }
}
