use a4n_api_contract::export::{
    BatchExportFormat, BatchExportInput, ExportFileData, ExportProgress,
    GalgameExportApplicationPort, ProjectExportApplicationPort,
};
use a4n_db::export::{
    ExportChapter, ExportDocument, chapter_heading, render_markdown_document, render_txt_document,
};
use a4n_document_export_core::{
    epub::{EpubBuilder, EpubChapter, EpubMetadata, builtin_epub_stylesheet},
    pdf::render_text_pdf,
};
use a4n_domain::project::required_text;
use a4n_error::AppError;
use base64::Engine as _;

use crate::{
    OrchestratorContext, collaboration::ProjectAccessRole, project::ensure_positive_project_id,
};

impl OrchestratorContext {
    pub async fn run_export_txt_validated(
        &self,
        project_id: i64,
    ) -> Result<ExportFileData, AppError> {
        ensure_positive_project_id(project_id)?;
        let document =
            a4n_db::export::export_document(self.database.connection(), project_id).await?;
        Ok(export_txt_file_data(project_id, &document.chapters))
    }

    pub async fn run_export_markdown_validated(
        &self,
        project_id: i64,
        title: &str,
        author: &str,
    ) -> Result<ExportFileData, AppError> {
        ensure_positive_project_id(project_id)?;
        let title = required_text(title, "Markdown 标题").map_err(AppError::InvalidArgument)?;
        let author = required_text(author, "Markdown 作者").map_err(AppError::InvalidArgument)?;
        let document =
            a4n_db::export::export_document(self.database.connection(), project_id).await?;
        Ok(export_markdown_file_data(
            project_id,
            &document.chapters,
            title,
            author,
        ))
    }

    pub async fn run_export_outline_json_validated(
        &self,
        project_id: i64,
    ) -> Result<ExportFileData, AppError> {
        ensure_positive_project_id(project_id)?;
        let value =
            a4n_db::export::export_outline_json(self.database.connection(), project_id).await?;
        export_json_file_data(project_id, "outline.json", value)
    }

    pub async fn run_export_characters_json_validated(
        &self,
        project_id: i64,
    ) -> Result<ExportFileData, AppError> {
        ensure_positive_project_id(project_id)?;
        let value =
            a4n_db::export::export_characters_json(self.database.connection(), project_id).await?;
        export_json_file_data(project_id, "characters.json", value)
    }

    pub async fn run_export_epub_validated(
        &self,
        project_id: i64,
        title: &str,
        author: &str,
        progress: &ExportProgress<'_>,
    ) -> Result<ExportFileData, AppError> {
        ensure_positive_project_id(project_id)?;
        let title = required_text(title, "EPUB 标题").map_err(AppError::InvalidArgument)?;
        let author = required_text(author, "EPUB 作者").map_err(AppError::InvalidArgument)?;
        let document = self
            .load_export_document_with_progress(project_id, progress, 4)
            .await?;
        export_epub_file_data(project_id, title, author, &document.chapters, progress)
    }

    pub async fn run_export_pdf_validated(
        &self,
        project_id: i64,
        progress: &ExportProgress<'_>,
    ) -> Result<ExportFileData, AppError> {
        ensure_positive_project_id(project_id)?;
        let document = self
            .load_export_document_with_progress(project_id, progress, 5)
            .await?;
        let title =
            required_text(&document.title, "项目标题").map_err(AppError::InvalidArgument)?;
        export_pdf_file_data(project_id, title, &document.chapters, progress)
    }

    pub async fn run_batch_export_validated(
        &self,
        input: BatchExportInput,
        progress: &ExportProgress<'_>,
    ) -> Result<Vec<ExportFileData>, AppError> {
        ensure_positive_project_id(input.project_id)?;
        validate_batch_formats(&input.formats)?;
        let needs_metadata = input
            .formats
            .iter()
            .any(|format| format.requires_metadata());
        let metadata = if needs_metadata {
            let metadata = input.metadata.as_ref().ok_or_else(|| {
                AppError::InvalidArgument(
                    "批量导出 Markdown 或 EPUB 时必须提供作品信息".to_string(),
                )
            })?;
            Some((
                required_text(&metadata.title, "批量导出标题")
                    .map_err(AppError::InvalidArgument)?,
                required_text(&metadata.author, "批量导出作者")
                    .map_err(AppError::InvalidArgument)?,
            ))
        } else {
            None
        };
        let total =
            u32::try_from(input.formats.len()).expect("validated batch format count fits u32") + 1;
        progress.progress(0, total, "正在准备批量导出");
        progress.check_cancelled()?;
        let document =
            a4n_db::export::export_document(self.database.connection(), input.project_id).await?;
        let nested_progress = progress.without_reporting();
        let mut results = Vec::with_capacity(input.formats.len());
        for (index, format) in input.formats.iter().copied().enumerate() {
            progress.check_cancelled()?;
            let current = u32::try_from(index).expect("validated batch format index fits u32") + 1;
            progress.progress(current, total, &format!("正在导出 {}", format.as_str()));
            let item = match (format, metadata) {
                (BatchExportFormat::Txt, _) => {
                    export_txt_file_data(input.project_id, &document.chapters)
                }
                (BatchExportFormat::Markdown, Some((title, author))) => {
                    export_markdown_file_data(input.project_id, &document.chapters, title, author)
                }
                (BatchExportFormat::Epub, Some((title, author))) => export_epub_file_data(
                    input.project_id,
                    title,
                    author,
                    &document.chapters,
                    &nested_progress,
                )?,
                (BatchExportFormat::Markdown | BatchExportFormat::Epub, None) => {
                    return Err(AppError::InvalidArgument(
                        "批量导出 Markdown 或 EPUB 时必须提供作品信息".to_string(),
                    ));
                }
                (BatchExportFormat::OutlineJson, _) => {
                    let value = a4n_db::export::export_outline_json(
                        self.database.connection(),
                        input.project_id,
                    )
                    .await?;
                    export_json_file_data(input.project_id, "outline.json", value)?
                }
                (BatchExportFormat::Pdf, _) => {
                    let title = required_text(&document.title, "项目标题")
                        .map_err(AppError::InvalidArgument)?;
                    export_pdf_file_data(
                        input.project_id,
                        title,
                        &document.chapters,
                        &nested_progress,
                    )?
                }
            };
            results.push(item);
        }
        progress.progress(total, total, "批量导出完成");
        Ok(results)
    }

    async fn load_export_document_with_progress(
        &self,
        project_id: i64,
        progress: &ExportProgress<'_>,
        total: u32,
    ) -> Result<ExportDocument, AppError> {
        progress.progress(0, total, "正在读取可导出章节");
        progress.check_cancelled()?;
        let document =
            a4n_db::export::export_document(self.database.connection(), project_id).await?;
        progress.check_cancelled()?;
        Ok(document)
    }
}

#[async_trait::async_trait]
impl ProjectExportApplicationPort for OrchestratorContext {
    async fn export_txt(&self, user_id: i64, project_id: i64) -> Result<ExportFileData, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.run_export_txt_validated(project_id).await
    }

    async fn export_markdown(
        &self,
        user_id: i64,
        project_id: i64,
        title: &str,
        author: &str,
    ) -> Result<ExportFileData, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.run_export_markdown_validated(project_id, title, author)
            .await
    }

    async fn export_outline_json(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<ExportFileData, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.run_export_outline_json_validated(project_id).await
    }

    async fn export_characters_json(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<ExportFileData, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.run_export_characters_json_validated(project_id).await
    }

    async fn export_epub(
        &self,
        user_id: i64,
        project_id: i64,
        title: &str,
        author: &str,
        progress: &ExportProgress<'_>,
    ) -> Result<ExportFileData, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.run_export_epub_validated(project_id, title, author, progress)
            .await
    }

    async fn export_pdf(
        &self,
        user_id: i64,
        project_id: i64,
        progress: &ExportProgress<'_>,
    ) -> Result<ExportFileData, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.run_export_pdf_validated(project_id, progress).await
    }

    async fn export_batch(
        &self,
        user_id: i64,
        input: BatchExportInput,
        progress: &ExportProgress<'_>,
    ) -> Result<Vec<ExportFileData>, AppError> {
        self.verify_project_permission(input.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.run_batch_export_validated(input, progress).await
    }
}

#[async_trait::async_trait]
impl GalgameExportApplicationPort for OrchestratorContext {
    async fn export_galgame_webgal(
        &self,
        user_id: i64,
        project_id: i64,
    ) -> Result<ExportFileData, AppError> {
        self.run_export_galgame_webgal_for_user_validated(user_id, project_id)
            .await
    }
}

fn export_epub_file_data(
    project_id: i64,
    title: &str,
    author: &str,
    chapters: &[ExportChapter],
    progress: &ExportProgress<'_>,
) -> Result<ExportFileData, AppError> {
    let total = export_chapter_count_total(chapters.len(), 4)?;
    progress.progress(
        1,
        total,
        &format!("已读取 {} 章，正在生成 EPUB", chapters.len()),
    );

    let mut builder = new_epub_builder(title, author);
    add_export_chapters_to_epub(&mut builder, chapters, |index, chapter| {
        progress.check_cancelled()?;
        let current = u32::try_from(index).map_err(|_| {
            AppError::InvalidArgument(format!("EPUB 章节索引超过 u32 上限：{index}"))
        })? + 2;
        progress.progress(
            current,
            total,
            &format!("正在写入 EPUB 第{}章", chapter.chapter_number),
        );
        Ok(())
    })?;

    progress.check_cancelled()?;
    progress.progress(total - 2, total, "正在打包 EPUB");
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

fn export_pdf_file_data(
    project_id: i64,
    title: &str,
    chapters: &[ExportChapter],
    progress: &ExportProgress<'_>,
) -> Result<ExportFileData, AppError> {
    let total = export_chapter_count_total(chapters.len(), 5)?;
    progress.progress(
        2,
        total,
        &format!("已读取 {} 章，正在排版 PDF", chapters.len()),
    );
    let mut current = 2;
    let paragraphs = render_pdf_paragraphs(chapters, progress, &mut current, total)?;
    progress.check_cancelled()?;
    progress.progress(total - 1, total, "正在生成 PDF 文件");
    let pdf = render_text_pdf(title, &paragraphs);
    progress.progress(total, total, "PDF 导出完成");
    Ok(export_binary_file_data(
        project_id,
        "pdf",
        "application/pdf",
        pdf,
    ))
}

fn validate_batch_formats(formats: &[BatchExportFormat]) -> Result<(), AppError> {
    if formats.is_empty() {
        return Err(AppError::InvalidArgument(
            "批量导出格式不能为空".to_string(),
        ));
    }
    for (index, format) in formats.iter().enumerate() {
        if formats[..index].contains(format) {
            return Err(AppError::InvalidArgument(format!(
                "批量导出格式重复: {}",
                format.as_str()
            )));
        }
    }
    Ok(())
}

fn export_txt_file_data(project_id: i64, chapters: &[ExportChapter]) -> ExportFileData {
    export_text_file_data(
        project_id,
        "txt",
        "text/plain;charset=utf-8",
        render_txt_document(chapters),
    )
}

fn export_markdown_file_data(
    project_id: i64,
    chapters: &[ExportChapter],
    title: &str,
    author: &str,
) -> ExportFileData {
    export_text_file_data(
        project_id,
        "md",
        "text/markdown;charset=utf-8",
        render_markdown_document(chapters, title, author),
    )
}

fn export_json_file_data(
    project_id: i64,
    extension: &str,
    value: serde_json::Value,
) -> Result<ExportFileData, AppError> {
    Ok(export_text_file_data(
        project_id,
        extension,
        "application/json;charset=utf-8",
        serde_json::to_string_pretty(&value)?,
    ))
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
    chapter_count.checked_add(extra_steps).ok_or_else(|| {
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
        *current += 1;
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
    fn batch_formats_reject_empty_and_duplicate_values() {
        assert!(validate_batch_formats(&[]).is_err());
        assert!(validate_batch_formats(&[BatchExportFormat::Txt, BatchExportFormat::Txt]).is_err());
        assert!(
            validate_batch_formats(&[BatchExportFormat::Txt, BatchExportFormat::Markdown]).is_ok()
        );
    }

    #[test]
    fn batch_input_rejects_legacy_metadata_fields() {
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
