use epub_builder::{EpubBuilder, EpubContent, ZipLibrary};
use std::fs::File;
use std::path::PathBuf;
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

pub fn entry_to_epub(download_dir: &str, entry: &feed_rs::model::Entry) -> Result<(), Error> {
    let html = extract_html_string_from_entry(entry)?;

    let mut epub_builder= EpubBuilder::new(ZipLibrary::new().unwrap()).unwrap();
    epub_builder
        .metadata("generator", "feed-to-epub").unwrap();

    if let Some(published_date) = &entry.published {
        epub_builder.set_publication_date(*published_date);
    }

    if let Some(summary) = &entry.summary {
        epub_builder
            .metadata("description", &summary.content)
            .unwrap();
    }

    let _ = &entry.authors.iter().map(|author| {
        epub_builder.add_author(&author.name);
    });

    match &entry.title {
        Some(title) => {
            let file_name = entry_title_to_file_name(
                download_dir,
                &title.content.replace('/', "_"),
            );
            let epub_file = File::create(file_name)?;

            epub_builder
                .metadata("title", &title.content).unwrap()
                .add_content(EpubContent::new(&title.content, html.as_bytes()))
                .unwrap()
                .generate(epub_file)
                .unwrap();
            Ok(())
        }
        None => Err(Error::TitleExtractionError)
    }
}

pub fn entry_title_to_file_name(destination_dir: &str, title: &str) -> PathBuf {
    PathBuf::from(format!("{}/{}.epub", destination_dir, title))
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
            return Ok(body);
        }
        Err(Error::BodyExtractionError)
    }
}
