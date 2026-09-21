use super::super::constants::CONTAINER_XML;
use super::super::cover::cover_metadata;
use super::super::xhtml::{build_chapter_xhtml, required_stylesheet};
use super::EpubBuilder;
use crate::DocumentExportError;

mod writer;

use writer::write_entry;

pub(super) fn build_epub(builder: &EpubBuilder) -> Result<Vec<u8>, DocumentExportError> {
    let cover = cover_metadata(&builder.metadata)?;
    let mut buffer = std::io::Cursor::new(Vec::new());
    {
        let mut zip_writer = zip::ZipWriter::new(&mut buffer);
        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        write_entry(
            &mut zip_writer,
            "mimetype",
            b"application/epub+zip",
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Stored),
        )?;
        write_entry(
            &mut zip_writer,
            "META-INF/container.xml",
            CONTAINER_XML.as_bytes(),
            options,
        )?;
        let opf = builder.build_opf()?;
        write_entry(
            &mut zip_writer,
            "OEBPS/content.opf",
            opf.as_bytes(),
            options,
        )?;
        let ncx = builder.build_ncx();
        write_entry(&mut zip_writer, "OEBPS/toc.ncx", ncx.as_bytes(), options)?;
        let css = required_stylesheet(&builder.stylesheet)?;
        write_entry(&mut zip_writer, "OEBPS/style.css", css.as_bytes(), options)?;
        for chapter in &builder.chapters {
            let filename = format!("OEBPS/chapter_{}.xhtml", chapter.order);
            let xhtml = build_chapter_xhtml(&chapter.title, &chapter.content);
            write_entry(&mut zip_writer, &filename, xhtml.as_bytes(), options)?;
        }
        if let Some(cover) = cover {
            write_entry(
                &mut zip_writer,
                &format!("OEBPS/cover.{}", cover.extension),
                cover.data,
                options,
            )?;
        }
        zip_writer
            .finish()
            .map_err(|source| DocumentExportError::EpubFinish { source })?;
    }
    Ok(buffer.into_inner())
}
