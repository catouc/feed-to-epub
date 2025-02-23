use crate::config::Config;
use crate::feed_reader::fetch_feed;
use anyhow::Result;
use clap::Parser;
use expanduser::expanduser;
use rusqlite::Connection;
use std::{thread, fs, time::Duration, path::PathBuf};

pub mod config;
pub mod feed_reader;
pub mod transformer;

const VERSION: &str = env!("CARGO_PKG_VERSION");

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

    let agent = ureq::AgentBuilder::new()
        .user_agent(&format!("feed-to-epub {}; +https:/github.com/catouc/feed-to-epub", VERSION))
        .build();

    let conn = Connection::open("feed-to-rss.db")?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS feeds (
            id INTEGER PRIMARY KEY,
            feed_url TEXT NOT NULL,
            last_modified TEXT,
            etag TEXT
        )",
        (),
    )?;

    loop {
        for (feed_name, feed) in config.feeds.iter() {
            match fs::create_dir_all(&feed.download_dir) {
                Ok(_) => (),
                Err(err) => eprintln!("failed to create download dir {} for feed {}: {}", &feed.download_dir, feed_name, err)
            };

            let url = url::Url::parse(&feed.url).expect("found invalid URL in configuration");
            let feed_data = fetch_feed(&conn, &agent, &url).unwrap();

            if let Some(feed_data) = feed_data {
                feed_data.entries.iter().for_each(|entry| {
                     match transformer::entry_to_epub(feed_name, &feed.download_dir, &entry) {
                        Ok(..) => (),
                        Err(err) => println!("failed to create epub: {}", err)
                    }
                });
            }
        }

        thread::sleep(Duration::from_secs(config.poll_interval_secs))
    }
}

