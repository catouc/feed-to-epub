use crate::feed_reader::config::Config;
use crate::feed_reader::FeedReader;
use crate::transformer::entry_to_epub;
use anyhow::Result;
use clap::Parser;
use expanduser::expanduser;
use std::{fs::File, thread, time::Duration};

pub mod feed_reader;
pub mod storage;
pub mod transformer;

#[derive(Parser, Debug)]
#[command(version, about, long_about=None)]
struct Args {
    #[arg(short, long, default_value = "~/.config/rss-to-epub/config.toml")]
    config: String,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let config_file = File::open(expanduser(&args.config)?)?;
    let config = Config::from_reader(config_file).expect("failed to read config file");
    let feed_reader_v2 = FeedReader::new(config).expect("failed to set up feed reader");

    loop {
        for (feed_name, feed) in feed_reader_v2.config.feeds.iter() {
            let feed_data = match feed_reader_v2.fetch_feed(
                feed_name,
                &feed.download_dir,
                jiff::Timestamp::now(),
            ) {
                Ok(feed_data) => feed_data,
                Err(err) => {
                    eprintln!("encountered error while fetching feed {}: {err}", feed.url);
                    None
                }
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

        thread::sleep(Duration::from_secs(
            feed_reader_v2.config.poll_interval_secs,
        ))
    }
}
