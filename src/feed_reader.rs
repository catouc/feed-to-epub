use feed_rs::model::Feed;
use feed_rs::parser;
use jiff::tz::TimeZone;
use jiff::Timestamp;
use rusqlite::Connection;
use thiserror::Error;
use ureq::Agent;
use url::Url;

const VERSION: &str = env!("CARGO_PKG_VERSION");

#[derive(Error, Debug)]
pub enum Error {
    #[error("database query failed: {0}")]
    DBError(#[from] rusqlite::Error),
    #[error("failed HTTP call: {0}")]
    HTTPError(#[from] ureq::Error),
    #[error("invalid feed data: {0}")]
    InvalidFeedDataError(#[from] feed_rs::parser::ParseFeedError),
    #[error("invalid timestamp found in feed database: {0}")]
    InvalidTimestamp(#[from] jiff::Error),
    #[error("rate limit reached we should back off")]
    RateLimitError,
}

pub struct FeedReader {
    agent: Agent,
    db: Connection,
}

pub enum ConditionalType {
    LastFetched,
    ETag,
}

impl FeedReader {
    pub fn new(db_file: &str) -> Self {
        let agent = ureq::AgentBuilder::new()
            .user_agent(&format!(
                "feed-to-epub {}; +https:/github.com/catouc/feed-to-epub",
                VERSION
            ))
            .build();

        let db = Connection::open(db_file).expect("failed to connect to feed db");

        FeedReader { agent, db }
    }

    fn get_last_modified(&self, feed_url: &Url) -> Result<String, Error> {
        let mut statement = self
            .db
            .prepare("SELECT last_modified FROM feeds WHERE feed_url = ?;")
            .expect("sql query wrong");

        let last_modified = statement.query_row([feed_url.as_str()], |r| r.get(0))?;
        Ok(last_modified)
    }

    fn get_etag(&self, feed_url: &Url) -> Result<String, Error> {
        let mut statement = self
            .db
            .prepare("SELECT etag FROM feeds WHERE feed_url = ?;")
            .expect("sql query wrong");

        let etag = statement.query_row([feed_url.as_str()], |r| r.get(0))?;
        Ok(etag)
    }

    fn get_last_fetched(&self, feed_url: &Url) -> Result<Timestamp, Error> {
        let mut statement = self
            .db
            .prepare("SELECT last_fetched FROM feeds WHERE feed_url = ?;")
            .expect("last fetched sql query wrong");

        let last_fetched: String = statement.query_row([feed_url.as_str()], |r| r.get(0))?;
        let last_fetched_ts: Timestamp = last_fetched.parse()?;
        Ok(last_fetched_ts)
    }

    pub fn fetch_feed(
        &self,
        feed_url: &Url,
        conditional: ConditionalType,
    ) -> Result<Option<Feed>, Error> {
        if let Ok(last_fetched) = self.get_last_fetched(feed_url) {
            let time_diff = Timestamp::now()
                .to_zoned(TimeZone::UTC)
                .duration_since(&last_fetched.to_zoned(TimeZone::UTC));

            println!("{feed_url} was last fetched {time_diff} ago");
            if time_diff.as_hours() < 2 {
                println!("{feed_url} was already fetched within the last two hours.");
                return Ok(None);
            };
        }

        let resp = match conditional {
            ConditionalType::LastFetched => {
                match self.get_last_modified(feed_url) {
                    Ok(last_modified) => self
                        .agent
                        .get(feed_url.as_str())
                        .set("If-Modified-Since", &last_modified)
                        .call()?,
                    // yes this needs to be better, basically I need to, I think return a
                    // Result<Option<String>, Err> from the get_feed_last_modified func
                    Err(..) => self.agent.get(feed_url.as_str()).call()?,
                }
            }
            ConditionalType::ETag => {
                match self.get_etag(feed_url) {
                    Ok(etag) => self
                        .agent
                        .get(feed_url.as_str())
                        .set("ETag", &etag)
                        .call()?,
                    // yes this needs to be better, basically I need to, I think return a
                    // Result<Option<String>, Err> from the get_feed_last_modified func
                    Err(..) => self.agent.get(feed_url.as_str()).call()?,
                }
            }
        };

        let now = Timestamp::now().to_zoned(TimeZone::UTC).to_string();

        if let Some(last_modified_since) = resp.header("Last-Modified") {
            if let Some(etag) = resp.header("ETag") {
                self.db.execute(
                    "INSERT OR REPLACE INTO feeds (id, feed_url, last_modified, last_fetched, etag)
                    VALUES ((SELECT id FROM feeds WHERE feed_url = ?1), ?1, ?2, ?3, ?4)",
                    (feed_url.as_str(), last_modified_since, &now, etag),
                )?;
            } else {
                self.db.execute(
                    "INSERT OR REPLACE INTO feeds (id, feed_url, last_modified, last_fetched)
                    VALUES ((SELECT id FROM feeds WHERE feed_url = ?1), ?1, ?2, ?3)",
                    (feed_url.as_str(), last_modified_since, &now),
                )?;
            }
        } else {
            self.db.execute(
                "INSERT OR REPLACE INTO feeds (id, feed_url, last_modified, last_fetched)
                VALUES ((SELECT id FROM feeds WHERE feed_url = ?1), ?1, ?2, ?3)",
                (feed_url.as_str(), rusqlite::types::Null, &now),
            )?;
        }

        match resp.status() {
            304 => Ok(None),
            429 => {
                // TODO: I should add something to maybe a special table of feeds that have
                // been rate limited to then check every iteration on whether we've gone past
                // the `Retry-After` header expiry.
                Err(Error::RateLimitError)
            }
            _ => {
                let feed_response_reader = resp.into_reader();
                let feed = parser::parse(feed_response_reader)?;
                Ok(Some(feed))
            }
        }
    }
}
