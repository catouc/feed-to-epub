use crate::config::Config;
use crate::feed_reader::fetch_feed;
use anyhow::Result;
use clap::Parser;
use expanduser::expanduser;
use rusqlite::Connection;
use std::{thread, time::Duration, path::PathBuf};

pub mod config;
pub mod feed_reader;
pub mod transformer;

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    #[arg(short, long, default_value="~/.config/rss-to-epub/config.toml")]
    config: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let config_path = PathBuf::from(expanduser(&args.config)?);
    let config = Config::try_from(config_path).expect("failed to load configuration");

    let conn = Connection::open("feed-to-rss.db")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS feeds (
            id INTEGER PRIMARY KEY,
            feed_url TEXT NOT NULL,
            last_modified TEXT NOT NULL
        )",
        (),
    )?;

    loop {
        config.feeds.iter()
            .filter_map(|feed| url::Url::parse(&feed.1.url).ok())
            .filter_map(|feed_request| fetch_feed(&conn, &feed_request).ok())
            .flat_map(|feed| feed.entries)
            .for_each(|entry| {
                match transformer::entry_to_epub(&entry) {
                    Ok(..) => (),
                    Err(err) => println!("failed to create epub: {}", err)
                }
            });
        thread::sleep(Duration::from_secs(config.poll_interval_secs))
    }
}
