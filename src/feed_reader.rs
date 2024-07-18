use feed_rs::model::Feed;
use feed_rs::parser;
use rusqlite::Connection;
use thiserror::Error;
use url::Url;

#[derive(Error, Debug)]
pub enum Error {
    #[error("database query failed")]
    DBError(#[from] rusqlite::Error),
    #[error("failed HTTP call")]
    HTTPError(#[from] ureq::Error),
    #[error("no new data on feed")]
    NoNewFeedDataError,
    #[error("invalid feed data")]
    InvalidFeedDataError(#[from] feed_rs::parser::ParseFeedError),
}

fn get_feed_last_modified(conn: &Connection, feed_url: &Url) -> Result<String, Error> {
    let mut statement = conn
        .prepare("SELECT last_modified FROM feeds WHERE feed_url = ?;")
        .expect("sql query wrong");

    let feed_request = statement.query_row([feed_url.to_string()], |r| r.get(0))?;
    Ok(feed_request)
}

pub fn fetch_feed(conn: &Connection, agent: &ureq::Agent, url: &Url) -> Result<Feed, Error> {
    let feed_url = url.to_string();

    let resp = match get_feed_last_modified(conn, url) {
        Ok(last_modified) => agent.get(&feed_url).set("If-Modified-Since", &last_modified).call()?,
        // yes this needs to be better, basically I need to, I think return a
        // Result<Option<String>, Err> from the get_feed_last_modified func
        Err(..) => agent.get(&feed_url).call()?,
    };

    if let Some(last_modified_since) = resp.header("Last-Modified") {
        if let Some(etag) = resp.header("ETag") {
            conn.execute(
                "INSERT OR REPLACE INTO feeds (id, feed_url, last_modified, etag) VALUES
                ((SELECT id FROM feeds WHERE feed_url = ?1), ?1, ?2, ?3)",
                (feed_url, last_modified_since, etag),
            )?;
        } else {
            conn.execute(
                "INSERT OR REPLACE INTO feeds (id, feed_url, last_modified) VALUES
                ((SELECT id FROM feeds WHERE feed_url = ?1), ?1, ?2, ?3)",
                (feed_url, last_modified_since),
            )?;
        }
    } else {
        conn.execute(
            "INSERT OR REPLACE INTO feeds (id, feed_url, last_modified) VALUES
            ((SELECT id FROM feeds WHERE feed_url = ?1), ?1, ?2)",
            (feed_url, rusqlite::types::Null),
        )?;
    }

    if resp.status() == 304 {
        Err(Error::NoNewFeedDataError)
    } else {
        let feed_response_reader = resp.into_reader();
        let feed = parser::parse(feed_response_reader)?;
        Ok(feed)
    }
}

