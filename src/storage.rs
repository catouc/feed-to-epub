use jiff::Timestamp;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("invalid timestamp found in database: {0}")]
    InvalidTimestamp(#[from] jiff::Error),
    #[error("database query failed: {0}")]
    DBError(#[from] rusqlite::Error),
}

pub struct Storage {
    db: rusqlite::Connection,
}

pub struct FeedStats {
    pub id: u64,
    pub url: String,
    // last_modified will not be a Timestamp since it's just a String we have to
    // pass back to the feed endpoint and not parse ourselves.
    pub last_modified: String,
    pub last_fetched: Option<Timestamp>,
    pub etag: Option<String>,
}

impl Storage {
    fn feed_stats_from_db(&self, url: &str) -> Result<FeedStats, Error> {
        let mut statement = self
            .db
            .prepare("SELECT id, last_modified, last_fetched, etag FROM feeds WHERE feed_url = ?;")
            .expect("sql query wrong");

        Ok(statement.query_row([url], |r| {
            let last_fetched_str: String = r.get(2)?;
            let last_fetched: Timestamp = last_fetched_str
                .parse()
                .expect("we manage our own timestamps, this row is corrupted");
            Ok(FeedStats {
                id: r.get(0)?,
                url: String::from(url),
                last_modified: r.get(1)?,
                last_fetched: Some(last_fetched),
                etag: r.get(3)?,
            })
        })?)
    }
}
