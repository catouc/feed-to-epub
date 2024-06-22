use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use feed_rs::parser;
use epub_builder::{EpubBuilder,ZipLibrary};

fn main() {
    let feed_file = File::open("./test/atom.xml").expect("feed file does not exist");
    let feed_buf_reader = BufReader::new(feed_file);

    let feed = parser::parse(feed_buf_reader).expect("feed file is not valid feed");

    println!("{}", feed.title.unwrap().content);

    File::create("/tmp/test.epub").expect("what the fuck");

    feed.entries.iter()
        .for_each(|entry| {
            if entry.title.is_some() && entry.content.is_some() {
                let title = &entry.title.as_ref().unwrap().content;
                let content = entry.content.as_ref().unwrap().body.as_ref().unwrap();
                println!("{} => {}", title, entry.content.as_ref().unwrap().content_type);
                let file_name = format!("/tmp/1.epub");
                println!("{}", file_name);
                let entry_epub = File::create(file_name).expect("file for entry could not be created");
                EpubBuilder::new(ZipLibrary::new().unwrap()).unwrap()
                    .add_content(epub_builder::EpubContent::new("1.xhtml", content.as_bytes())).unwrap()
                    .generate(entry_epub).unwrap();
                panic!("stop iter");
            }
        });
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
