use epub_builder::{EpubBuilder, EpubContent, EpubVersion, MetadataOpf, MetadataOpfV3, ZipLibrary};
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("could not create file.\nError: {0}")]
    FileCreationError(#[from] std::io::Error),
    #[error("could not build epub library builder: {0}")]
    EpubBuilderError(#[from] epub_builder::Error),
    #[error("could not extract content from entry: {0}")]
    ContentExtractionError(#[from] crate::storage::EntryConversionError),
}

pub struct EpubFile {
    pub file_path: PathBuf,
    pub data: Vec<u8>,
}

pub fn entry_to_epub(
    feed_name: &str,
    download_dir: &str,
    entry: &feed_rs::model::Entry,
) -> Result<EpubFile, Error> {
    let html = crate::storage::extract_html_string_from_entry(entry)?;
    let xhtml = crate::storage::html_string_to_xhtml_epub_string(&html);

    let mut epub_builder = EpubBuilder::new(ZipLibrary::new()?)?;
    epub_builder
        .epub_version(EpubVersion::V33)
        .metadata("generator", "feed-to-epub")?
        .add_metadata_opf(Box::new(MetadataOpfV3::new(
            "belongs-to-collection".into(),
            feed_name.into(),
        )));

    if let Some(published_date) = &entry.published {
        epub_builder.set_publication_date(*published_date);
    }

    if let Some(summary) = &entry.summary {
        // This will definitely be somewhat arbitrary with unicode
        // but we just want to avoid some feeds that stuff their
        // entire content into the summary field from polluting
        // the description fied.
        const MAX_SUMMARY_LENGTH_BYTES: usize = 1000;
        if summary.content.len() < MAX_SUMMARY_LENGTH_BYTES {
            epub_builder.metadata("description", &summary.content)?;
        };
    }

    let _ = &entry.authors.iter().map(|author| {
        epub_builder
            .add_metadata_opf(Box::new(MetadataOpf {
                name: "dc:creator".into(),
                content: author.name.clone(),
            }))
            .add_author(&author.name);
    });

    // TODO: Not sure I enjoy unpacking the title twice...
    // I should probably rewrite this function to have
    // some invariant checks and give me my title variable
    // and all others that I require at the start.
    //
    // This just leads to my annoyment at Rusts Option
    // unpacking since I have to some weird dances.
    let file_path = match &entry.title {
        Some(title) => entry_title_to_file_name(download_dir, &title.content.replace('/', "_")),
        _ => {
            return Err(Error::ContentExtractionError(
                crate::storage::EntryConversionError::TitleExtractionError,
            ))
        }
    };

    match &entry.title {
        Some(title) => {
            let _ = &epub_builder
                .metadata("title", &title.content)?
                .add_content(EpubContent::new(&title.content, xhtml.as_bytes()))?;
        }
        _ => {
            return Err(Error::ContentExtractionError(
                crate::storage::EntryConversionError::TitleExtractionError,
            ))
        }
    }

    let mut data: Vec<u8> = Vec::new();
    epub_builder.generate(&mut data)?;
    Ok(EpubFile { file_path, data })
}

pub fn entry_title_to_file_name(destination_dir: &str, title: &str) -> PathBuf {
    PathBuf::from(format!("{destination_dir}/{title}.epub"))
}
