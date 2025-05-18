use crate::config::Config;
use crate::feed_reader::{ConditionalType, FeedReader};
use crate::transformer::entry_to_epub;
use anyhow::Result;
use clap::Parser;
use expanduser::expanduser;
use rusqlite::Connection;
use std::{fs, thread, time::Duration};

pub mod config;
pub mod feed_reader;
pub mod feed_reader_v2;
pub mod transformer;

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    #[arg(short, long, default_value = "~/.config/rss-to-epub/config.toml")]
    config: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let config_path = expanduser(&args.config)?;
    let config = Config::try_from(config_path).expect("failed to load configuration");

    let conn = Connection::open("feed-to-rss.db")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS feeds (
            id INTEGER PRIMARY KEY,
            feed_url TEXT NOT NULL,
            last_modified TEXT,
            last_fetched TEXT,
            etag TEXT
        )",
        (),
    )?;

    let feed_reader = FeedReader::new("feed-to-rss.db");

    loop {
        for (feed_name, feed) in config.feeds.iter() {
            match fs::create_dir_all(&feed.download_dir) {
                Ok(_) => (),
                Err(err) => {
                    eprintln!(
                        "failed to create download dir {} for feed {}: {}",
                        &feed.download_dir, feed_name, err,
                    )
                }
            };

            let url = url::Url::parse(&feed.url).expect("found invalid URL in configuration");
            let feed_data = match feed_reader.fetch_feed(&url, ConditionalType::LastFetched) {
                Ok(feed_data) => feed_data,
                Err(err) => {
                    eprintln!("encountered error while fetching feed {url}: {err}");
                    None
                },
            };

            if let Some(feed_data) = feed_data {
                feed_data.entries.iter().for_each(|entry| {
                    match entry_to_epub(feed_name, &feed.download_dir, entry) {
                        Ok(..) => (),
                        Err(err) => println!("failed to create epub: {}", err),
                    }
                });
            }
        }

        thread::sleep(Duration::from_secs(config.poll_interval_secs))
    }
}
