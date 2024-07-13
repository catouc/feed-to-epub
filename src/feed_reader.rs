use feed_rs::model::Feed;
use feed_rs::parser;
use rusqlite::Connection;
use thiserror::Error;
use url::Url;

#[derive(Error, Debug)]
pub enum Error {
    #[error("could nor parse URL")]
    UrlParseError(#[from] url::ParseError),
    #[error("database query failed")]
    DBError(#[from] rusqlite::Error),
    #[error("failed HTTP call")]
    HTTPError(#[from] ureq::Error),
    #[error("no new data on feed")]
    NoNewFeedDataError,
    #[error("invalid feed data")]
    InvalidFeedDataError(#[from] feed_rs::parser::ParseFeedError),
}

#[derive(Debug)]
pub struct FeedRequest {
    pub if_modified_since: Option<String>,
    pub url: Url,
}

impl FeedRequest {
    pub fn from_conn_and_url(db_conn: &Connection, url: Url) -> Result<Self, Error> {
        match get_feed_last_modified(db_conn, &url) {
            Ok(feed_request) => Ok(FeedRequest {
                if_modified_since: Some(feed_request),
                url,
            }),
            Err(..) => Ok(FeedRequest {
                if_modified_since: None,
                url,
            }),
        }
    }
}

impl From<FeedRequest> for ureq::Request {
    fn from(val: FeedRequest) -> Self {
        if let Some(last_modified_since_header) = val.if_modified_since {
            ureq::get(&String::from(val.url)).set("If-Modified-Since", &last_modified_since_header)
        } else {
            ureq::get(&String::from(val.url))
        }
    }
}

fn get_feed_last_modified(conn: &Connection, feed_url: &Url) -> Result<String, Error> {
    let mut statement = conn
        .prepare("SELECT last_modified FROM feeds WHERE feed_url = ?;")
        .expect("sql query wrong");

    let feed_request = statement.query_row([feed_url.to_string()], |r| r.get(0))?;
    Ok(feed_request)
}

pub fn fetch_feed(conn: &Connection, feed_request: FeedRequest) -> Result<Feed, Error> {
    // I don't like this, but I think I need to rethink how my feed logic is working
    let feed_url_string = String::from(feed_request.url.clone());
    let request: ureq::Request = feed_request.into();
    let resp = request.call()?;

    if let Some(last_modified_since) = resp.header("Last-Modified") {
        conn.execute(
            "INSERT OR REPLACE INTO feeds (id, feed_url, last_modified) VALUES
            ((SELECT id FROM feeds WHERE feed_url = ?1), ?1, ?2)",
            (feed_url_string, last_modified_since),
        )?;
    } else {
        conn.execute(
            "INSERT OR REPLACE INTO feeds (id, feed_url, last_modified) VALUES
            ((SELECT id FROM feeds WHERE feed_url = ?1), ?1, ?2)",
            (feed_url_string, rusqlite::types::Null),
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
