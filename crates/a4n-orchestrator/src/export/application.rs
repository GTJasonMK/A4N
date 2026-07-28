use a4n_core::error::AppError;
use a4n_export_core::{
    epub::{EpubBuilder, EpubChapter, EpubMetadata, builtin_epub_stylesheet},
    pdf::render_text_pdf,
};
use base64::Engine as _;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

use crate::{OrchestratorContext, project::required_text};

use super::{
    ExportChapter, batch_export_metadata_fields, chapter_heading, normalize_batch_export_formats,
};

const EPUB_EXPORT_CANCELLED: &str = "EPUB 导出已停止";
const PDF_EXPORT_CANCELLED: &str = "PDF 导出已停止";

pub type ExportProgressReporter<'a> = &'a (dyn Fn(u32, u32, &str) + Send + Sync);

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExportMarkdownInput {
    pub title: String,
    pub author: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct BatchExportInput {
    pub project_id: i64,
    pub formats: Vec<String>,
    pub title: Option<String>,
    pub author: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ExportFileData {
    pub file_name: String,
    pub mime_type: String,
    pub data_base64: String,
}

pub struct ExportProgress<'a> {
    reporter: Option<ExportProgressReporter<'a>>,
    cancel_token: Option<&'a CancellationToken>,
}

impl<'a> ExportProgress<'a> {
    pub fn new(
        reporter: Option<ExportProgressReporter<'a>>,
        cancel_token: Option<&'a CancellationToken>,
    ) -> Self {
        Self {
            reporter,
            cancel_token,
        }
    }

    fn progress(&self, current: u32, total: u32, message: &str) {
        if let Some(reporter) = self.reporter {
            reporter(current, total.max(1), message);
        }
    }

    fn check_cancelled(&self) -> Result<(), AppError> {
        if self
            .cancel_token
            .is_some_and(CancellationToken::is_cancelled)
        {
            Err(AppError::Cancelled)
        } else {
            Ok(())
        }
    }
}

impl OrchestratorContext {
    pub async fn run_export_txt_validated(
        &self,
        project_id: i64,
    ) -> Result<ExportFileData, AppError> {
        validate_project_id(project_id)?;
        let content = self.export_txt_content(project_id).await?;
        Ok(export_text_file_data(
            project_id,
            "txt",
            "text/plain;charset=utf-8",
            content,
        ))
    }

    pub async fn run_export_markdown_validated(
        &self,
        project_id: i64,
        title: &str,
        author: &str,
    ) -> Result<ExportFileData, AppError> {
        validate_project_id(project_id)?;
        let title = required_text(title, "Markdown 标题").map_err(AppError::InvalidArgument)?;
        let author = required_text(author, "Markdown 作者").map_err(AppError::InvalidArgument)?;
        let content = self
            .export_markdown_content(project_id, title, author)
            .await?;
        Ok(export_text_file_data(
            project_id,
            "md",
            "text/markdown;charset=utf-8",
            content,
        ))
    }

    pub async fn run_export_outline_json_validated(
        &self,
        project_id: i64,
    ) -> Result<ExportFileData, AppError> {
        validate_project_id(project_id)?;
        let value = self.export_outline_json_value(project_id).await?;
        let content = serde_json::to_string_pretty(&value)?;
        Ok(export_text_file_data(
            project_id,
            "outline.json",
            "application/json;charset=utf-8",
            content,
        ))
    }

    pub async fn run_export_characters_json_validated(
        &self,
        project_id: i64,
    ) -> Result<ExportFileData, AppError> {
        validate_project_id(project_id)?;
        let value = self.export_characters_json_value(project_id).await?;
        let content = serde_json::to_string_pretty(&value)?;
        Ok(export_text_file_data(
            project_id,
            "characters.json",
            "application/json;charset=utf-8",
            content,
        ))
    }

    pub async fn run_export_epub_validated(
        &self,
        project_id: i64,
        title: &str,
        author: &str,
        progress: &ExportProgress<'_>,
    ) -> Result<ExportFileData, AppError> {
        validate_project_id(project_id)?;
        let title = required_text(title, "EPUB 标题").map_err(AppError::InvalidArgument)?;
        let author = required_text(author, "EPUB 作者").map_err(AppError::InvalidArgument)?;
        let chapters = self
            .load_export_chapters_with_progress(project_id, progress, 4, EPUB_EXPORT_CANCELLED)
            .await?;
        let total = export_chapter_count_total(chapters.len(), 4)?;
        progress.progress(
            1,
            total,
            &format!("已读取 {} 章，正在生成 EPUB", chapters.len()),
        );

        let mut builder = new_epub_builder(title, author);
        add_export_chapters_to_epub(&mut builder, &chapters, |index, chapter| {
            progress.check_cancelled()?;
            let current = u32::try_from(index)
                .map_err(|_| {
                    AppError::InvalidArgument(format!("EPUB 章节索引超过 u32 上限：{index}"))
                })?
                .checked_add(2)
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "EPUB 导出进度超过 u32 上限：章节索引 {index}"
                    ))
                })?;
            progress.progress(
                current,
                total,
                &format!("正在写入 EPUB 第{}章", chapter.chapter_number),
            );
            Ok(())
        })?;

        progress.check_cancelled()?;
        let packaging_progress = total.checked_sub(2).ok_or_else(|| AppError::InvalidState {
            current: total.to_string(),
            operation: "EPUB 导出总进度不足以表示打包阶段".to_string(),
        })?;
        progress.progress(packaging_progress, total, "正在打包 EPUB");
        let epub = builder.build().map_err(|error| AppError::InvalidState {
            current: "epub_rendering".to_string(),
            operation: error.to_string(),
        })?;
        progress.progress(total, total, "EPUB 导出完成");
        Ok(export_binary_file_data(
            project_id,
            "epub",
            "application/epub+zip",
            epub,
        ))
    }

    pub async fn run_export_pdf_validated(
        &self,
        project_id: i64,
        progress: &ExportProgress<'_>,
    ) -> Result<ExportFileData, AppError> {
        validate_project_id(project_id)?;
        progress.progress(0, 5, "正在读取项目信息");
        progress.check_cancelled()?;
        let title = self.project_title(project_id).await?;
        let title = required_text(&title, "项目标题").map_err(AppError::InvalidArgument)?;
        let chapters = self
            .load_export_chapters_with_progress(project_id, progress, 5, PDF_EXPORT_CANCELLED)
            .await?;
        let total = export_chapter_count_total(chapters.len(), 5)?;
        progress.progress(
            2,
            total,
            &format!("已读取 {} 章，正在排版 PDF", chapters.len()),
        );
        let mut current = 2;
        let paragraphs = render_pdf_paragraphs(&chapters, progress, &mut current, total)?;
        progress.check_cancelled()?;
        let encoding_progress = total.checked_sub(1).ok_or_else(|| AppError::InvalidState {
            current: total.to_string(),
            operation: "PDF 导出总进度不足以表示编码阶段".to_string(),
        })?;
        progress.progress(encoding_progress, total, "正在生成 PDF 文件");
        let pdf = render_text_pdf(title, &paragraphs);
        progress.progress(total, total, "PDF 导出完成");
        Ok(export_binary_file_data(
            project_id,
            "pdf",
            "application/pdf",
            pdf,
        ))
    }

    pub async fn run_batch_export_validated(
        &self,
        input: BatchExportInput,
        progress: &ExportProgress<'_>,
    ) -> Result<Vec<ExportFileData>, AppError> {
        validate_project_id(input.project_id)?;
        let formats =
            normalize_batch_export_formats(&input.formats).map_err(AppError::InvalidArgument)?;
        let needs_metadata = formats.contains(&"markdown") || formats.contains(&"epub");
        let (title, author) = batch_export_metadata_fields(
            input.title.as_deref(),
            input.author.as_deref(),
            needs_metadata,
        )
        .map_err(AppError::InvalidArgument)?;
        let format_count = u32::try_from(formats.len())
            .map_err(|_| AppError::InvalidArgument("批量导出格式数量超出 u32 范围".to_string()))?;
        let total = format_count.checked_add(1).ok_or_else(|| {
            AppError::InvalidArgument("批量导出进度总量超出 u32 范围".to_string())
        })?;
        progress.progress(0, total, "正在准备批量导出");
        let mut results = Vec::with_capacity(formats.len());
        for (index, format) in formats.iter().enumerate() {
            progress.check_cancelled()?;
            let current = u32::try_from(index)
                .map_err(|_| {
                    AppError::InvalidArgument("批量导出进度索引超出 u32 范围".to_string())
                })?
                .checked_add(1)
                .ok_or_else(|| {
                    AppError::InvalidArgument("批量导出进度索引超出 u32 范围".to_string())
                })?;
            progress.progress(current, total, &format!("正在导出 {format}"));
            let item = match *format {
                "txt" => self.run_export_txt_validated(input.project_id).await?,
                "markdown" => {
                    self.run_export_markdown_validated(
                        input.project_id,
                        required_batch_metadata(title, "批量导出标题")?,
                        required_batch_metadata(author, "批量导出作者")?,
                    )
                    .await?
                }
                "epub" => {
                    self.run_export_epub_validated(
                        input.project_id,
                        required_batch_metadata(title, "批量导出标题")?,
                        required_batch_metadata(author, "批量导出作者")?,
                        progress,
                    )
                    .await?
                }
                "json" => {
                    self.run_export_outline_json_validated(input.project_id)
                        .await?
                }
                "pdf" => {
                    self.run_export_pdf_validated(input.project_id, progress)
                        .await?
                }
                _ => unreachable!("normalize_batch_export_formats must reject unsupported formats"),
            };
            results.push(item);
        }
        progress.progress(total, total, "批量导出完成");
        Ok(results)
    }

    async fn load_export_chapters_with_progress(
        &self,
        project_id: i64,
        progress: &ExportProgress<'_>,
        total: u32,
        _cancel_message: &str,
    ) -> Result<Vec<ExportChapter>, AppError> {
        progress.progress(0, total, "正在读取可导出章节");
        progress.check_cancelled()?;
        let chapters = self.export_chapter_rows(project_id).await?;
        progress.check_cancelled()?;
        Ok(chapters)
    }
}

fn validate_project_id(project_id: i64) -> Result<(), AppError> {
    if project_id <= 0 {
        Err(AppError::InvalidArgument("项目 ID 必须大于 0".to_string()))
    } else {
        Ok(())
    }
}

fn required_batch_metadata<'a>(value: Option<&'a str>, field: &str) -> Result<&'a str, AppError> {
    value.ok_or_else(|| AppError::InvalidArgument(format!("{field}不能为空")))
}

fn export_file_name(project_id: i64, extension: &str) -> String {
    format!("{project_id}_export.{extension}")
}

fn export_text_file_data(
    project_id: i64,
    extension: &str,
    mime_type: &str,
    content: String,
) -> ExportFileData {
    export_binary_file_data(project_id, extension, mime_type, content.into_bytes())
}

fn export_binary_file_data(
    project_id: i64,
    extension: &str,
    mime_type: &str,
    data: impl AsRef<[u8]>,
) -> ExportFileData {
    ExportFileData {
        file_name: export_file_name(project_id, extension),
        mime_type: mime_type.to_string(),
        data_base64: base64::engine::general_purpose::STANDARD.encode(data),
    }
}

fn export_chapter_count_total(chapter_count: usize, extra_steps: u32) -> Result<u32, AppError> {
    let chapter_count = u32::try_from(chapter_count).map_err(|_| {
        AppError::InvalidArgument(format!("导出章节数超过 u32 上限：{chapter_count}"))
    })?;
    chapter_count
        .checked_add(extra_steps)
        .map(|total| total.max(1))
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "导出总进度超过 u32 上限：{chapter_count} + {extra_steps}"
            ))
        })
}

fn epub_chapter_from_export_chapter(chapter: &ExportChapter, order: usize) -> EpubChapter {
    EpubChapter {
        title: chapter_heading(chapter.chapter_number, chapter.title.as_deref()),
        content: chapter.content.clone(),
        order,
    }
}

fn new_epub_builder(title: &str, author: &str) -> EpubBuilder {
    let metadata = EpubMetadata {
        title: title.to_string(),
        author: author.to_string(),
        language: "zh-CN".into(),
        publisher: None,
        date: None,
        description: None,
        cover_image: None,
        cover_mime: None,
    };
    EpubBuilder::new(metadata, builtin_epub_stylesheet().to_string())
}

fn add_export_chapters_to_epub(
    builder: &mut EpubBuilder,
    chapters: &[ExportChapter],
    mut before_chapter: impl FnMut(usize, &ExportChapter) -> Result<(), AppError>,
) -> Result<(), AppError> {
    for (index, chapter) in chapters.iter().enumerate() {
        before_chapter(index, chapter)?;
        builder.add_chapter(epub_chapter_from_export_chapter(chapter, index + 1));
    }
    Ok(())
}

fn render_pdf_paragraphs(
    chapters: &[ExportChapter],
    progress: &ExportProgress<'_>,
    current: &mut u32,
    total: u32,
) -> Result<Vec<String>, AppError> {
    let mut paragraphs = vec![String::new()];
    for chapter in chapters {
        progress.check_cancelled()?;
        *current = current
            .checked_add(1)
            .ok_or_else(|| AppError::InvalidArgument("PDF 导出进度超过 u32 上限".to_string()))?;
        progress.progress(
            *current,
            total,
            &format!("正在排版 PDF 第{}章", chapter.chapter_number),
        );
        paragraphs.push(chapter_heading(
            chapter.chapter_number,
            chapter.title.as_deref(),
        ));
        paragraphs.push(chapter.content.clone());
    }
    Ok(paragraphs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epub_chapter_uses_plain_content_without_html_wrapper() {
        let chapter = ExportChapter {
            id: 1,
            chapter_number: 3,
            title: Some("风起".to_string()),
            content: "正文第一段\n\n正文第二段".to_string(),
        };
        let epub_chapter = epub_chapter_from_export_chapter(&chapter, 1);
        assert_eq!(epub_chapter.title, "第3章 风起");
        assert_eq!(epub_chapter.content, "正文第一段\n\n正文第二段");
        assert!(!epub_chapter.content.contains("<html"));
    }

    #[test]
    fn epub_builder_accepts_export_chapters() {
        let chapters = vec![ExportChapter {
            id: 1,
            chapter_number: 1,
            title: Some("开端".to_string()),
            content: "第一章正文".to_string(),
        }];
        let mut builder = new_epub_builder("测试书名", "测试作者");
        add_export_chapters_to_epub(&mut builder, &chapters, |_, _| Ok(()))
            .expect("chapters should be accepted");
        let epub_data = builder.build().expect("epub should build");
        assert_eq!(&epub_data[0..2], b"PK");
    }

    #[test]
    fn export_chapter_count_rejects_progress_overflow() {
        let error = export_chapter_count_total(u32::MAX as usize, 1)
            .expect_err("export progress overflow must be visible");
        assert!(error.to_string().contains("导出总进度超过 u32 上限"));
    }

    #[test]
    fn export_inputs_reject_unknown_fields() {
        let markdown_error = serde_json::from_value::<ExportMarkdownInput>(serde_json::json!({
            "title": "测试书名",
            "author": "测试作者",
            "project_id": 42,
        }))
        .expect_err("unknown markdown input fields should be rejected");
        assert!(markdown_error.to_string().contains("unknown field"));

        let batch_error = serde_json::from_value::<BatchExportInput>(serde_json::json!({
            "project_id": 42,
            "formats": ["txt"],
            "title": "测试书名",
            "author": "测试作者",
            "output_path": "/tmp/export.txt",
        }))
        .expect_err("unknown batch input fields should be rejected");
        assert!(batch_error.to_string().contains("unknown field"));
    }
}
