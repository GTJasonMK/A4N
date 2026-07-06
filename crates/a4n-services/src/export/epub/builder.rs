use super::constants::CONTAINER_XML;
use super::cover::cover_metadata;
use super::model::{EpubChapter, EpubMetadata};
use super::xhtml::{build_chapter_xhtml, required_stylesheet};
use a4n_core::error::AppError;
use std::io::Write;

/// EPUB3 构建器
pub struct EpubBuilder {
    pub(super) metadata: EpubMetadata,
    pub(super) chapters: Vec<EpubChapter>,
    pub(super) stylesheet: String,
}

impl EpubBuilder {
    pub fn new(metadata: EpubMetadata, stylesheet: String) -> Self {
        Self {
            metadata,
            chapters: Vec::new(),
            stylesheet,
        }
    }

    /// 添加章节
    pub fn add_chapter(&mut self, chapter: EpubChapter) -> &mut Self {
        self.chapters.push(chapter);
        self
    }

    /// 构建 EPUB 文件（返回字节数据）
    pub fn build(&self) -> Result<Vec<u8>, AppError> {
        let cover = cover_metadata(&self.metadata)?;
        let mut buffer = std::io::Cursor::new(Vec::new());
        {
            let mut zip_writer = zip::ZipWriter::new(&mut buffer);

            let options = zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated);

            zip_writer
                .start_file(
                    "mimetype",
                    zip::write::SimpleFileOptions::default()
                        .compression_method(zip::CompressionMethod::Stored),
                )
                .map_err(|e| AppError::Internal(anyhow::anyhow!("EPUB 写入失败: {e}")))?;
            zip_writer
                .write_all(b"application/epub+zip")
                .map_err(|e| AppError::Internal(anyhow::anyhow!("EPUB 写入失败: {e}")))?;

            zip_writer
                .start_file("META-INF/container.xml", options)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("EPUB 写入失败: {e}")))?;
            zip_writer
                .write_all(CONTAINER_XML.as_bytes())
                .map_err(|e| AppError::Internal(anyhow::anyhow!("EPUB 写入失败: {e}")))?;

            let opf = self.build_opf()?;
            zip_writer
                .start_file("OEBPS/content.opf", options)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("EPUB 写入失败: {e}")))?;
            zip_writer
                .write_all(opf.as_bytes())
                .map_err(|e| AppError::Internal(anyhow::anyhow!("EPUB 写入失败: {e}")))?;

            let ncx = self.build_ncx();
            zip_writer
                .start_file("OEBPS/toc.ncx", options)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("EPUB 写入失败: {e}")))?;
            zip_writer
                .write_all(ncx.as_bytes())
                .map_err(|e| AppError::Internal(anyhow::anyhow!("EPUB 写入失败: {e}")))?;

            let css = required_stylesheet(&self.stylesheet)?;
            zip_writer
                .start_file("OEBPS/style.css", options)
                .map_err(|e| AppError::Internal(anyhow::anyhow!("EPUB 写入失败: {e}")))?;
            zip_writer
                .write_all(css.as_bytes())
                .map_err(|e| AppError::Internal(anyhow::anyhow!("EPUB 写入失败: {e}")))?;

            for chapter in &self.chapters {
                let filename = format!("OEBPS/chapter_{}.xhtml", chapter.order);
                let xhtml = build_chapter_xhtml(&chapter.title, &chapter.content);
                zip_writer
                    .start_file(&filename, options)
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("EPUB 写入失败: {e}")))?;
                zip_writer
                    .write_all(xhtml.as_bytes())
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("EPUB 写入失败: {e}")))?;
            }

            if let Some(cover) = cover {
                zip_writer
                    .start_file(format!("OEBPS/cover.{}", cover.extension), options)
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("EPUB 写入失败: {e}")))?;
                zip_writer
                    .write_all(cover.data)
                    .map_err(|e| AppError::Internal(anyhow::anyhow!("EPUB 写入失败: {e}")))?;
            }

            zip_writer
                .finish()
                .map_err(|e| AppError::Internal(anyhow::anyhow!("EPUB 打包失败: {e}")))?;
        }

        Ok(buffer.into_inner())
    }
}
