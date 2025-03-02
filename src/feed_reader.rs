use feed_rs::model::Feed;
use feed_rs::parser;
use jiff::tz::TimeZone;
use jiff::Timestamp;
use rusqlite::Connection;
use thiserror::Error;
use ureq::Agent;
use url::Url;

#[derive(Error, Debug)]
pub enum Error {
    #[error("database query failed")]
    DBError(#[from] rusqlite::Error),
    #[error("failed HTTP call")]
    HTTPError(#[from] ureq::Error),
    #[error("invalid feed data")]
    InvalidFeedDataError(#[from] feed_rs::parser::ParseFeedError),
    #[error("invalid timestamp found in feed database: {0}")]
    InvalidTimestamp(#[from] jiff::Error),
}

fn get_feed_last_modified(conn: &Connection, feed_url: &Url) -> Result<String, Error> {
    let mut statement = conn
        .prepare("SELECT last_modified FROM feeds WHERE feed_url = ?;")
        .expect("sql query wrong");

    let feed_request = statement.query_row([feed_url.to_string()], |r| r.get(0))?;
    Ok(feed_request)
}

fn get_feed_last_fetched(conn: &Connection, feed_url: &Url) -> Result<Timestamp, Error> {
    let mut statement = conn
        .prepare("SELECT last_modified FROM feeds WHERE feed_url = ?;")
        .expect("last fetched sql query wrong");

    let last_fetched: String = statement.query_row([feed_url.to_string()], |r| r.get(0))?;
    let last_fetched_ts: Timestamp = last_fetched.parse()?;
    Ok(last_fetched_ts)
}

pub fn fetch_feed(conn: &Connection, agent: &Agent, url: &Url) -> Result<Option<Feed>, Error> {
    let feed_url = url.to_string();

    if let Ok(last_fetched) = get_feed_last_fetched(conn, url) {
        let time_diff = Timestamp::now() - last_fetched;
        if time_diff.get_hours() < 2 {
            eprintln!("{feed_url} was already fetched within the last two hours.");
            return Ok(None);
        };
    }

    let resp = match get_feed_last_modified(conn, url) {
        Ok(last_modified) => agent
            .get(&feed_url)
            .set("If-Modified-Since", &last_modified)
            .call()?,
        // yes this needs to be better, basically I need to, I think return a
        // Result<Option<String>, Err> from the get_feed_last_modified func
        Err(..) => agent.get(&feed_url).call()?,
    };

    let now = Timestamp::now().to_zoned(TimeZone::UTC).to_string();

    if let Some(last_modified_since) = resp.header("Last-Modified") {
        if let Some(etag) = resp.header("ETag") {
            conn.execute(
                "INSERT OR REPLACE INTO feeds (id, feed_url, last_modified, last_fetched, etag) VALUES
                ((SELECT id FROM feeds WHERE feed_url = ?1), ?1, ?2, ?3, ?4)",
                (feed_url, last_modified_since, &now, etag),
            )?;
        } else {
            conn.execute(
                "INSERT OR REPLACE INTO feeds (id, feed_url, last_modified, last_fetched) VALUES
                ((SELECT id FROM feeds WHERE feed_url = ?1), ?1, ?2, ?3)",
                (feed_url, last_modified_since, &now),
            )?;
        }
    } else {
        conn.execute(
            "INSERT OR REPLACE INTO feeds (id, feed_url, last_modified, last_fetched) VALUES
            ((SELECT id FROM feeds WHERE feed_url = ?1), ?1, ?2, ?3)",
            (feed_url, rusqlite::types::Null, &now),
        )?;
    }

    if resp.status() == 304 {
        Ok(None)
    } else {
        let feed_response_reader = resp.into_reader();
        let feed = parser::parse(feed_response_reader)?;
        Ok(Some(feed))
    }
}
