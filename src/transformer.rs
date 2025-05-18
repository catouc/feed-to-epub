use epub_builder::{EpubBuilder, EpubContent, MetadataOpf, ZipLibrary};
use std::fs::File;
use std::path::PathBuf;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("could not create file.\nError: {0}")]
    FileCreationError(#[from] std::io::Error),
    #[error("could not get bytes from HTML content")]
    BodyExtractionError,
    #[error("could not get bytes from HTML summary")]
    SummaryExtractionError,
    #[error("could not get title from entry")]
    TitleExtractionError,
    #[error("could not build epub library builder: {0}")]
    EpubBuilderError(#[from] epub_builder::Error)
}

pub fn entry_to_epub(
    feed_name: &str,
    download_dir: &str,
    entry: &feed_rs::model::Entry,
) -> Result<(), Error> {
    let html = extract_html_string_from_entry(entry)?;
    let xhtml = html_string_to_xhtml_epub_string(&html);

    let mut epub_builder = EpubBuilder::new(ZipLibrary::new()?)?;
    epub_builder
        .metadata("generator", "feed-to-epub")?
        .add_metadata_opf(MetadataOpf {
            name: "calibre:series".into(),
            content: feed_name.into(),
        });

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
            .add_metadata_opf(MetadataOpf {
                name: "dc:creator".into(),
                content: author.name.clone(),
            })
            .add_author(&author.name);
    });

    // TODO: Not sure I enjoy unpacking the title twice...
    // I should probably rewrite this function to have
    // some invariant checks and give me my title variable
    // and all others that I require at the start.
    //
    // This just leads to my annoyment at Rusts Option
    // unpacking since I have to some weird dances.
    let epub_file = match &entry.title {
        Some(title) => {
            let file_name =
                entry_title_to_file_name(download_dir, &title.content.replace('/', "_"));
            File::create(file_name)?
        }
        None => return Err(Error::TitleExtractionError),
    };

    match &entry.title {
        Some(title) => {
            let _ = &epub_builder
                .metadata("title", &title.content)?
                .add_content(EpubContent::new(&title.content, xhtml.as_bytes()))?;
        }
        None => return Err(Error::TitleExtractionError),
    }

    epub_builder.generate(epub_file)?;
    Ok(())
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
        Err(Error::SummaryExtractionError)
    }
}

fn html_string_to_xhtml_epub_string(html: &str) -> String {
    let mut xhtml: String = "".into();
    xhtml.push_str(
        r#"<?xml version="1.0" encoding="UTF-8" ?>
<!DOCTYPE html PUBLIC "-//W3C//DTD XHTML 1.1//EN" "http://www.w3.org/TR/xhtml11/DTD/xhtml11.dtd">
<html xmlns="http://www.w3.org/1999/xhtml" xml:lang="en">
  <head>
    <meta http-equiv="Content-Type" content="application/xhtml+xml; charset=utf-8" />
    <title>Pride and Prejudice</title>
    <link rel="stylesheet" href="css/main.css" type="text/css" />
  </head>
  <body>
"#,
    );
    xhtml.push_str(html);
    xhtml.push_str(
        r#"  </body>
</html>"#,
    );
    xhtml
}
