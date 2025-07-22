use jiff::Timestamp;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ErrorNew {
    #[error("invalid timestamp found in database: {0}")]
    InvalidTimestamp(#[from] jiff::Error),
    #[error("failed to open database file {db_file}: {source}")]
    DBFileOpenError {
        db_file: String,
        source: rusqlite::Error,
    },
}

#[derive(Error, Debug)]
pub enum ErrorDBOperation {
    #[error("database query failed: {0}")]
    DBError(#[from] rusqlite::Error),
}

pub struct Storage {
    db: rusqlite::Connection,
}

#[derive(Debug, PartialEq)]
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
    pub fn new(db_file: &str) -> Result<Self, ErrorNew> {
        let db = match rusqlite::Connection::open(db_file) {
            Ok(db) => db,
            Err(err) => {
                return Err(ErrorNew::DBFileOpenError {
                    db_file: db_file.to_string(),
                    source: err,
                })
            }
        };

        Ok(Storage { db })
    }

    pub fn feed_stats_from_db(&self, url: &str) -> Result<FeedStats, ErrorDBOperation> {
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

    pub fn feed_stats_to_db(&self, feed_stats: &FeedStats) -> Result<(), ErrorDBOperation> {
       let mut statement = self
           .db
           .prepare(
               "INSERT OR REPLACE INTO feeds (id, feed_url, etag, last_modified, last_fetched)
               VALUES (?1, ?2, ?3, ?4, ?5)")
           .expect("SQL syntax error");

        if let Some(last_fetched) = feed_stats.last_fetched {
            let _ = statement.execute(
                (
                    feed_stats.id,
                    &feed_stats.url,
                    &feed_stats.etag,
                    &feed_stats.last_modified,
                    last_fetched.to_string(),
                )
            )?;
            Ok(())
        } else {
             let _ = statement.execute(
                (
                    feed_stats.id,
                    &feed_stats.url,
                    &feed_stats.etag,
                    &feed_stats.last_modified,
                    rusqlite::types::Null,
                )
            )?;
            Ok(())

        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feed_stats_to_and_from_db() {
        let db = rusqlite::Connection::open_in_memory().expect("could not open DB in memory");
        db.execute(
            "CREATE TABLE IF NOT EXISTS feeds (
                    id INTEGER PRIMARY KEY,
                    feed_url TEXT NOT NULL,
                    last_modified TEXT,
                    last_fetched TEXT,
                    etag TEXT
            )",
            (),
        ).expect("failed to set up test DB");

        let storage = Storage{db};

        let now = Timestamp::now();

        let feed_stats = FeedStats{
            id: 1,
            url: "https://example.com".into(),
            last_modified: "1970-01-01T00:00:00Z".into(),
            last_fetched: Some(now),
            etag: Some("foo".into()),
        };

        let _ = storage.feed_stats_to_db(&feed_stats).expect("failed to store feed_stats");
        let db_feed_stats = storage.feed_stats_from_db("https://example.com")
            .expect("failed to read feed_stats back out of DB");
        
        assert_eq!(feed_stats, db_feed_stats);
    }
}
