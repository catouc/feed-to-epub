use std::fs::File;
use std::io::BufReader;
use feed_rs::parser;
use std::path::PathBuf;
use anyhow::Result;

pub mod transformer;

fn main() -> Result<()> {
    let feed_file = File::open("./test/atom.xml")?;
    let feed_buf_reader = BufReader::new(feed_file);
    let feed = parser::parse(feed_buf_reader)?;

    println!("{}", feed.title.unwrap().content);

    feed.entries.iter()
        .for_each(|entry| {
            transformer::entry_to_epub(entry, &PathBuf::from("/tmp/1.epub")).expect("epub failed to create");
        });
    Ok(())
    // rss feed reader daemon
    // gets a config file per feed like:
    // ~/.local/rss-to-epub/feeds.d/some-feed.conf
    // Where we define the 
    // url, destination path and other things to add into the resulting epub
    // Then we read a feed like normal, and start a pipeline to transform the HTML of each post
    // into epub putting it into the output
    //
    // Periodically we wake up and see if there's new stuff on the horizon (once a day seems to be
    // enough)
    // * Handle if-modified-since and/or etags, gotta have a re-read of some stuff for that.
}

