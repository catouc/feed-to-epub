use std::fs::File;
use std::path::PathBuf;
use epub_builder::{EpubBuilder, EpubContent, ZipLibrary};
use thiserror::Error;


#[derive(Error, Debug)]
pub enum Error {
    #[error("could not create file")]
    FileCreationError(#[from] std::io::Error),
    #[error("could not get bytes from HTML content")]
    BodyExtractionError,
    #[error("could not get title from entry")]
    TitleExtractionError,
}

pub fn entry_to_epub(entry: &feed_rs::model::Entry) -> Result<(), Error> {
    let html = extract_html_string_from_entry(entry)?;

    if let Some(title) = &entry.title {
        //let file_name = PathBuf::from(updated.checked_add_months()).join(".epub");
        let file_name = entry_title_to_file_name("./test", &title.content.replace("/", "_"));
        println!("{:?}", file_name);
        let epub_file = File::create(file_name)?;
        let _ = EpubBuilder::new(ZipLibrary::new().unwrap()).unwrap()
            .add_content(EpubContent::new(&title.content, html.as_bytes())).unwrap()
            .generate(epub_file);
        Ok(())
    } else {
        return Err(Error::TitleExtractionError) 
    }
}

pub fn entry_title_to_file_name(destination_dir: &str, title: &str) -> PathBuf {
    let path = PathBuf::from(format!("{}/{}.epub", destination_dir, title));
    path
}

fn extract_html_string_from_entry(entry: &feed_rs::model::Entry) -> Result<String, Error> {
    if let Some(content) = &entry.content {
        if let Some(body) = &content.body {
            Ok(body.to_string())
        } else {
            Err(Error::BodyExtractionError) 
        }
    } else {
        if let Some(summary) = &entry.summary {
            let body = summary.content.clone();
            return Ok(body)
        }
        Err(Error::BodyExtractionError)
    }
}
