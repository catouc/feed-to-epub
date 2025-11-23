use jiff::Timestamp;
use rusqlite::OptionalExtension;
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
    #[error("failed to initialise database: {0}")]
    DBError(#[from] ErrorDBOperation),
}

pub struct Storage {
    db: rusqlite::Connection,
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

    /// new_in_memory is largely only ever used in testing
    /// as a convenience to not have to deal with the life-
    /// cycle of a file handle.
    pub fn new_in_memory() -> Result<Self, ErrorNew> {
        let db = match rusqlite::Connection::open_in_memory() {
            Ok(db) => db,
            Err(err) => {
                return Err(ErrorNew::DBFileOpenError {
                    db_file: "memory".into(),
                    source: err,
                })
            }
        };
        Ok(Storage { db })
    }

    pub fn init_database(&self) -> Result<(), ErrorDBOperation> {
        self.db.execute(
            "CREATE TABLE IF NOT EXISTS feeds (
                id INTEGER PRIMARY KEY,
                feed_url TEXT NOT NULL,
                last_modified TEXT,
                last_fetched TEXT,
                etag TEXT
            )",
            (),
        )?;

        self.db.execute(
            "CREATE TABLE IF NOT EXISTS entries (
                id INTEGER PRIMARY KEY,
                feed_id INTEGER NOT NULL,
                feed_entry_id TEXT,
                title TEXT,
                updated TEXT,
                authors TEXT,
                summary TEXT,
                content BLOB NOT NULL,
                FOREIGN KEY(feed_id) REFERENCES feeds(id)
            )",
            (),
        )?;

        Ok(())
    }
}

#[derive(Debug, PartialEq)]
pub struct FeedStats {
    pub id: u64,
    pub url: String,
    // last_modified will not be a Timestamp since it's just a String we have to
    // pass back to the feed endpoint and not parse ourselves.
    pub last_modified: Option<String>,
    pub last_fetched: Option<Timestamp>,
    pub etag: Option<String>,
}

#[derive(Error, Debug)]
pub enum ErrorDBOperation {
    #[error("database query failed: {0}")]
    DBError(#[from] rusqlite::Error),
}

#[derive(Error, Debug)]
pub enum ErrorNewFeedStats {
    #[error("database insert failed: {0}")]
    DBInsertError(#[from] rusqlite::Error),
    #[error("failed to feed from database: {0}")]
    DBError(#[from] ErrorDBOperation),
    #[error("somehow cannot find feed we just created, report a bug")]
    NewFeedNotFoundError,
}

impl Storage {
    pub fn feed_stats_from_db(&self, url: &str) -> Result<Option<FeedStats>, ErrorDBOperation> {
        let mut statement = self
            .db
            .prepare("SELECT id, last_modified, last_fetched, etag FROM feeds WHERE feed_url = ?;")
            .expect("sql query wrong");

        let feed_stats = statement
            .query_row([url], |r| {
                let last_fetched_str: Option<String> = r.get(2)?;

                let last_fetched: Option<Timestamp> = match last_fetched_str {
                    Some(last_fetched) => {
                        let last_fetched: Timestamp = last_fetched
                            .parse()
                            .expect("we manage our own timestamps, this row is corrupted");
                        Some(last_fetched)
                    }
                    None => None,
                };

                let feed_stats = FeedStats {
                    id: r.get(0)?,
                    url: String::from(url),
                    last_modified: r.get(1)?,
                    last_fetched,
                    etag: r.get(3)?,
                };

                println!("{feed_stats:#?}");
                Ok(feed_stats)
            })
            .optional()?;

        Ok(feed_stats)
    }

    pub fn feed_stats_to_db(&self, feed_stats: &FeedStats) -> Result<(), ErrorDBOperation> {
        let mut statement = self
            .db
            .prepare(
                "INSERT OR REPLACE INTO feeds (id, feed_url, etag, last_modified, last_fetched)
               VALUES ((SELECT id FROM feeds WHERE feed_url = ?1), ?1, ?2, ?3, ?4)",
            )
            .expect("SQL syntax error");

        if let Some(last_fetched) = feed_stats.last_fetched {
            let _ = statement.execute((
                &feed_stats.url,
                &feed_stats.etag,
                &feed_stats.last_modified,
                last_fetched.to_string(),
            ))?;
            Ok(())
        } else {
            let _ = statement.execute((
                &feed_stats.url,
                &feed_stats.etag,
                &feed_stats.last_modified,
                rusqlite::types::Null,
            ))?;
            Ok(())
        }
    }

    pub fn new_feed_stats_to_db(&self, url: &str) -> Result<FeedStats, ErrorNewFeedStats> {
        let mut statement = self
            .db
            .prepare("INSERT INTO feeds (feed_url) VALUES (?1)")
            .expect("SQL syntax error");

        statement.execute((url,))?;
        match self.feed_stats_from_db(url)? {
            Some(feed_stats) => Ok(feed_stats),
            None => Err(ErrorNewFeedStats::NewFeedNotFoundError),
        }
    }
}

#[derive(Debug, PartialEq)]
pub struct Entry {
    pub feed_id: u64,
    pub feed_entry_id: Option<String>,
    pub title: String,
    pub updated: Option<String>,
    pub authors: Option<String>, // TODO: make this a vec?
    pub summary: String,
    pub content: String,
}

#[derive(Error, Debug)]
pub enum EntryConversionError {
    #[error("could not get bytes from HTML content")]
    BodyExtractionError,
    #[error("could not get bytes from HTML summary")]
    SummaryExtractionError,
    #[error("could not get title from entry")]
    TitleExtractionError,
}

pub fn entry_from_feed_entry(
    feed_id: u64,
    feed_entry: &feed_rs::model::Entry,
) -> Result<Entry, EntryConversionError> {
    let title = match &feed_entry.title {
        Some(title) => title.content.clone(),
        None => "".into(),
    };

    let updated = feed_entry.updated.map(|updated| updated.to_rfc3339());

    let mut authors = Vec::with_capacity(feed_entry.authors.len());
    feed_entry.authors.iter().for_each(|author| {
        authors.push(author.name.clone());
    });

    let mut summary_content: String = "".into();
    if let Some(summary) = &feed_entry.summary {
        // This will definitely be somewhat arbitrary with unicode
        // but we just want to avoid some feeds that stuff their
        // entire content into the summary field from polluting
        // the description fied.
        const MAX_SUMMARY_LENGTH_BYTES: usize = 1000;
        if summary.content.len() < MAX_SUMMARY_LENGTH_BYTES {
            summary_content = summary.content.clone();
        };
    }

    let content = extract_html_string_from_entry(feed_entry)?;

    Ok(Entry {
        feed_id,
        feed_entry_id: Some(feed_entry.id.clone()),
        title,
        updated,
        authors: Some(authors.join(",")),
        summary: summary_content,
        content,
    })
}

pub fn extract_html_string_from_entry(
    entry: &feed_rs::model::Entry,
) -> Result<String, EntryConversionError> {
    if let Some(content) = &entry.content {
        if let Some(body) = &content.body {
            Ok(body.to_string())
        } else {
            Err(EntryConversionError::BodyExtractionError)
        }
    } else {
        if let Some(summary) = &entry.summary {
            let body = summary.content.clone();
            return Ok(body);
        }
        Err(EntryConversionError::SummaryExtractionError)
    }
}

pub fn html_string_to_xhtml_epub_string(html: &str) -> String {
    let mut xhtml: String = "".into();
    xhtml.push_str(
        r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.1//EN" "http://www.w3.org/TR/xhtml11/DTD/xhtml11.dtd">
<html xmlns="http://www.w3.org/1999/xhtml" xml:lang="en">
  <head>
    <meta http-equiv="Content-Type" content="application/xhtml+xml; charset=utf-8" />
    <title>Pride and Prejudice</title>
    <link rel="stylesheet" href="css/main.css" type="text/css" />
  </head>
  <body>
"#,
    );
    xhtml.push_str(html);
    xhtml.push_str(
        r#"  </body>
</html>"#,
    );
    xhtml
}

impl Storage {
    pub fn entry_from_db(&self, feed_entry_id: &str) -> Result<Entry, ErrorDBOperation> {
        let mut statement = self
            .db
            .prepare("SELECT feed_id, feed_entry_id, title, updated, authors, summary, content FROM entries WHERE feed_entry_id = ?;")
            .expect("sql query wrong");

        Ok(statement.query_row([feed_entry_id], |r| {
            Ok(Entry {
                feed_id: r.get(0)?,
                feed_entry_id: r.get(1)?,
                title: r.get(2)?,
                updated: r.get(3)?,
                authors: r.get(4)?,
                summary: r.get(5)?,
                content: r.get(6)?,
            })
        })?)
    }

    pub fn new_entry_to_db(&self, feed_entry: &Entry) -> Result<(), ErrorDBOperation> {
        let mut statement = self
            .db
            .prepare(
                "INSERT OR REPLACE INTO entries (feed_id, feed_entry_id, title, updated, authors, summary, content)
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            ).expect("SQL syntax error");

        statement.execute((
            feed_entry.feed_id,
            &feed_entry.feed_entry_id,
            &feed_entry.title,
            &feed_entry.updated,
            &feed_entry.authors,
            &feed_entry.summary,
            &feed_entry.content,
        ))?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn feed_to_and_from_db() {
        let storage = Storage::new_in_memory().expect("failed to open in memory db");
        storage.init_database().expect("failed to set up test DB");
        let now = Timestamp::now();

        let feed_stats = FeedStats {
            id: 1,
            url: "https://example.com".into(),
            last_modified: Some("1970-01-01T00:00:00Z".into()),
            last_fetched: Some(now),
            etag: Some("foo".into()),
        };

        storage
            .feed_stats_to_db(&feed_stats)
            .expect("failed to store feed_stats");
        let db_feed_stats = storage
            .feed_stats_from_db("https://example.com")
            .expect("failed to read feed_stats back out of DB");

        if let Some(db_feed_stats) = db_feed_stats {
            assert_eq!(feed_stats, db_feed_stats);
        } else {
            panic!("expected a feed_stats struct back, but got None")
        };

        let feed_entry = Entry {
            feed_id: 1,
            feed_entry_id: Some("foo".into()),
            title: "bar".into(),
            updated: Some("baz".into()),
            authors: Some("John Doe".into()),
            summary: "some summary".into(),
            content: "<XML here>".into(),
        };

        storage
            .new_entry_to_db(&feed_entry)
            .expect("failed to store feed_entry");
        let db_feed_entry = storage
            .entry_from_db("foo")
            .expect("failed to read feed_entry out of DB");

        assert_eq!(feed_entry, db_feed_entry);
    }
}
