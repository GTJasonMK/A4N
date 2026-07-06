use crate::{AppState, actions::access::verify_project_editor_string};
use a4n_core::models::project::novel_project;
use a4n_services::export::ExportChapter;
use a4n_services::export::epub::{EpubBuilder, EpubChapter, EpubMetadata, builtin_epub_stylesheet};
use base64::Engine as _;
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};
use tokio_util::sync::CancellationToken;

pub type ExportProgressReporter<'a> = &'a (dyn Fn(u32, u32, &str) + Send + Sync);
const BATCH_EXPORT_CANCELLED: &str = "批量导出已停止";

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

#[derive(Debug, Clone, Copy)]
pub struct ExportTaskScope {
    pub project_id: i64,
    pub total: u32,
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

    fn check_cancelled(&self, message: &str) -> Result<(), String> {
        if self.cancel_token.is_some_and(|token| token.is_cancelled()) {
            Err(message.to_string())
        } else {
            Ok(())
        }
    }
}

macro_rules! export_project_scope {
    ($name:ident, $total:expr) => {
        pub async fn $name(
            state: &AppState,
            user_id: i64,
            project_id: i64,
        ) -> Result<ExportTaskScope, String> {
            export_project_task_scope(state, user_id, project_id, $total).await
        }
    };
}

export_project_scope!(export_epub_task_scope, 4);
export_project_scope!(export_txt_task_scope, 2);
export_project_scope!(export_markdown_task_scope, 2);
export_project_scope!(export_pdf_task_scope, 5);

pub async fn export_batch_task_scope(
    state: &AppState,
    user_id: i64,
    input: &BatchExportInput,
) -> Result<ExportTaskScope, String> {
    export_project_task_scope(state, user_id, input.project_id, 4).await
}

export_project_scope!(export_outline_json_task_scope, 2);
export_project_scope!(export_characters_json_task_scope, 2);

async fn build_epub_data(
    state: &AppState,
    project_id: i64,
    title: &str,
    author: &str,
    progress: &ExportProgress<'_>,
) -> Result<Vec<u8>, String> {
    let chapters = load_export_chapters(state, project_id, progress, 4, "EPUB 导出已停止").await?;
    let total = export_chapter_count_total(chapters.len(), 4);
    progress.progress(
        1,
        total,
        &format!("已读取 {} 章，正在生成 EPUB", chapters.len()),
    );

    let mut builder = new_epub_builder(title, author);
    add_export_chapters_to_epub(&mut builder, &chapters, |i, ch| {
        progress.check_cancelled("EPUB 导出已停止")?;
        progress.progress(
            2 + i as u32,
            total,
            &format!("正在写入 EPUB 第{}章", ch.chapter_number),
        );
        Ok(())
    })?;

    progress.check_cancelled("EPUB 导出已停止")?;
    progress.progress(total.saturating_sub(2), total, "正在打包 EPUB");
    builder.build().map_err(|e| e.to_string())
}

async fn build_txt_export_content(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<String, String> {
    export_txt_task_scope(state, user_id, project_id).await?;
    state
        .export
        .export_txt(project_id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn export_txt_data(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ExportFileData, String> {
    let content = build_txt_export_content(state, user_id, project_id).await?;
    Ok(export_text_file_data(
        project_id,
        "txt",
        "text/plain;charset=utf-8",
        content,
    ))
}

async fn build_markdown_export_content(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    title: &str,
    author: &str,
) -> Result<String, String> {
    export_markdown_task_scope(state, user_id, project_id).await?;
    let title = required_text(title, "Markdown 标题")?;
    let author = required_text(author, "Markdown 作者")?;
    state
        .export
        .export_markdown(project_id, title, author)
        .await
        .map_err(|e| e.to_string())
}

pub async fn export_markdown_data(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    title: &str,
    author: &str,
) -> Result<ExportFileData, String> {
    let content = build_markdown_export_content(state, user_id, project_id, title, author).await?;
    Ok(export_text_file_data(
        project_id,
        "md",
        "text/markdown;charset=utf-8",
        content,
    ))
}

pub async fn export_epub_data(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    title: String,
    author: String,
    progress: &ExportProgress<'_>,
) -> Result<ExportFileData, String> {
    export_epub_task_scope(state, user_id, project_id).await?;
    let title = required_text(&title, "EPUB 标题")?;
    let author = required_text(&author, "EPUB 作者")?;
    let epub_data = build_epub_data(state, project_id, title, author, progress).await?;
    progress.progress(4, 4, "EPUB 导出完成");
    Ok(export_binary_file_data(
        project_id,
        "epub",
        "application/epub+zip",
        epub_data,
    ))
}

pub async fn export_pdf_data(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    progress: &ExportProgress<'_>,
) -> Result<ExportFileData, String> {
    export_pdf_task_scope(state, user_id, project_id).await?;
    progress.progress(0, 5, "正在读取项目信息");
    progress.check_cancelled("PDF 导出已停止")?;
    let project = novel_project::Entity::find_by_id(project_id)
        .one(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .ok_or_else(|| format!("项目不存在: project_id={project_id}"))?;
    let project_title = required_text(&project.title, "项目标题")?;
    let chapters = load_export_chapters(state, project_id, progress, 5, "PDF 导出已停止").await?;
    let total = export_chapter_count_total(chapters.len(), 5);
    progress.progress(
        2,
        total,
        &format!("已读取 {} 章，正在排版 PDF", chapters.len()),
    );
    let mut current = 2;
    let paragraphs =
        render_pdf_paragraphs(&chapters, progress, &mut current, total, "PDF 导出已停止")?;
    progress.check_cancelled("PDF 导出已停止")?;
    progress.progress(total.saturating_sub(1), total, "正在生成 PDF 文件");
    let pdf = text_pdf_bytes(project_title, &paragraphs);
    progress.progress(total, total, "PDF 导出完成");
    Ok(export_binary_file_data(
        project_id,
        "pdf",
        "application/pdf",
        pdf,
    ))
}

pub async fn export_batch_data(
    state: &AppState,
    user_id: i64,
    input: BatchExportInput,
    progress: &ExportProgress<'_>,
) -> Result<Vec<ExportFileData>, String> {
    let project_id = input.project_id;
    export_batch_task_scope(state, user_id, &input).await?;
    let formats = normalize_batch_formats(&input.formats)?;
    let needs_metadata = formats.contains(&"markdown") || formats.contains(&"epub");
    let (title, author) = batch_export_metadata(&input, needs_metadata)?;
    let total = formats.len() as u32 + 1;
    progress.progress(0, total, "正在准备批量导出");
    let mut results = Vec::with_capacity(formats.len());
    for (index, format) in formats.iter().enumerate() {
        progress.check_cancelled(BATCH_EXPORT_CANCELLED)?;
        progress.progress(index as u32 + 1, total, &format!("正在导出 {format}"));
        let item = match *format {
            "txt" => export_txt_data(state, user_id, project_id).await?,
            "markdown" => {
                export_markdown_data(
                    state,
                    user_id,
                    project_id,
                    title.ok_or_else(|| "批量导出标题不能为空".to_string())?,
                    author.ok_or_else(|| "批量导出作者不能为空".to_string())?,
                )
                .await?
            }
            "epub" => {
                export_epub_data(
                    state,
                    user_id,
                    project_id,
                    title
                        .ok_or_else(|| "批量导出标题不能为空".to_string())?
                        .to_string(),
                    author
                        .ok_or_else(|| "批量导出作者不能为空".to_string())?
                        .to_string(),
                    progress,
                )
                .await?
            }
            "json" => export_outline_json_data(state, user_id, project_id).await?,
            "pdf" => export_pdf_data(state, user_id, project_id, progress).await?,
            _ => unreachable!("normalize_batch_formats must reject unsupported formats"),
        };
        results.push(item);
    }
    progress.progress(total, total, "批量导出完成");
    Ok(results)
}

async fn build_outline_json_export_value(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<serde_json::Value, String> {
    export_outline_json_task_scope(state, user_id, project_id).await?;
    state
        .export
        .export_outline_json(project_id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn export_outline_json_data(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ExportFileData, String> {
    let value = build_outline_json_export_value(state, user_id, project_id).await?;
    let content = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
    Ok(export_text_file_data(
        project_id,
        "outline.json",
        "application/json;charset=utf-8",
        content,
    ))
}

async fn build_characters_json_export_value(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<serde_json::Value, String> {
    export_characters_json_task_scope(state, user_id, project_id).await?;
    state
        .export
        .export_characters_json(project_id)
        .await
        .map_err(|e| e.to_string())
}

pub async fn export_characters_json_data(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ExportFileData, String> {
    let value = build_characters_json_export_value(state, user_id, project_id).await?;
    let content = serde_json::to_string_pretty(&value).map_err(|e| e.to_string())?;
    Ok(export_text_file_data(
        project_id,
        "characters.json",
        "application/json;charset=utf-8",
        content,
    ))
}

async fn export_project_task_scope(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    total: u32,
) -> Result<ExportTaskScope, String> {
    verify_project_editor_string(state, user_id, project_id).await?;
    Ok(ExportTaskScope { project_id, total })
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

fn epub_chapter_from_export_chapter(chapter: &ExportChapter, order: usize) -> EpubChapter {
    EpubChapter {
        title: a4n_services::export::chapter_heading(
            chapter.chapter_number,
            chapter.title.as_deref(),
        ),
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
    mut before_chapter: impl FnMut(usize, &ExportChapter) -> Result<(), String>,
) -> Result<(), String> {
    for (i, ch) in chapters.iter().enumerate() {
        before_chapter(i, ch)?;
        builder.add_chapter(epub_chapter_from_export_chapter(ch, i + 1));
    }
    Ok(())
}

fn render_pdf_paragraphs(
    chapters: &[ExportChapter],
    progress: &ExportProgress<'_>,
    current: &mut u32,
    total: u32,
    cancel_message: &str,
) -> Result<Vec<String>, String> {
    let mut paragraphs = vec![String::new()];
    visit_export_chapters(
        chapters,
        progress,
        current,
        total,
        cancel_message,
        "排版 PDF",
        |ch| {
            paragraphs.push(a4n_services::export::chapter_heading(
                ch.chapter_number,
                ch.title.as_deref(),
            ));
            paragraphs.push(ch.content.clone());
            Ok(())
        },
    )?;
    Ok(paragraphs)
}

fn visit_export_chapters(
    chapters: &[ExportChapter],
    progress: &ExportProgress<'_>,
    current: &mut u32,
    total: u32,
    cancel_message: &str,
    action: &str,
    mut visit: impl FnMut(&ExportChapter) -> Result<(), String>,
) -> Result<(), String> {
    for chapter in chapters {
        progress.check_cancelled(cancel_message)?;
        *current = current.saturating_add(1);
        progress.progress(
            *current,
            total,
            &format!("正在{action} 第{}章", chapter.chapter_number),
        );
        visit(chapter)?;
    }
    Ok(())
}

async fn load_export_chapters(
    state: &AppState,
    project_id: i64,
    progress: &ExportProgress<'_>,
    total: u32,
    cancel_message: &str,
) -> Result<Vec<ExportChapter>, String> {
    progress.progress(0, total, "正在读取可导出章节");
    progress.check_cancelled(cancel_message)?;
    let chapters = state
        .export
        .export_chapters(project_id)
        .await
        .map_err(|e| e.to_string())?;
    progress.check_cancelled(cancel_message)?;
    Ok(chapters)
}

fn export_chapter_count_total(chapter_count: usize, extra_steps: u32) -> u32 {
    (chapter_count as u32).saturating_add(extra_steps).max(1)
}

fn normalize_batch_formats(formats: &[String]) -> Result<Vec<&'static str>, String> {
    if formats.is_empty() {
        return Err("批量导出格式不能为空".to_string());
    }

    let mut normalized = Vec::with_capacity(formats.len());
    for format in formats {
        let format = format.trim();
        if format.is_empty() {
            return Err("批量导出格式不能为空".to_string());
        }
        let normalized_format = match format.to_ascii_lowercase().as_str() {
            "txt" => "txt",
            "markdown" | "md" => "markdown",
            "epub" => "epub",
            "json" => "json",
            "pdf" => "pdf",
            _ => return Err(format!("不支持的导出格式: {format}")),
        };
        if normalized.contains(&normalized_format) {
            return Err(format!("批量导出格式重复: {normalized_format}"));
        }
        normalized.push(normalized_format);
    }
    Ok(normalized)
}

fn batch_export_metadata(
    input: &BatchExportInput,
    needs_metadata: bool,
) -> Result<(Option<&str>, Option<&str>), String> {
    if !needs_metadata {
        return Ok((None, None));
    }

    Ok((
        Some(required_optional_text(
            input.title.as_deref(),
            "批量导出标题",
        )?),
        Some(required_optional_text(
            input.author.as_deref(),
            "批量导出作者",
        )?),
    ))
}

fn required_text<'a>(value: &'a str, label: &str) -> Result<&'a str, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{label}不能为空"))
    } else {
        Ok(value)
    }
}

fn required_optional_text<'a>(value: Option<&'a str>, label: &str) -> Result<&'a str, String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{label}不能为空"))
}

const PAGE_WIDTH: f32 = 595.0;
const PAGE_HEIGHT: f32 = 842.0;
const LEFT_MARGIN: f32 = 50.0;
const TOP_MARGIN: f32 = 56.0;
const FONT_SIZE: f32 = 12.0;
const LINE_HEIGHT: f32 = 17.0;
const MAX_CHARS_PER_LINE: usize = 42;

fn text_pdf_bytes(title: &str, paragraphs: &[String]) -> Vec<u8> {
    let mut lines = Vec::new();
    if !title.trim().is_empty() {
        lines.push(title.trim().to_string());
        lines.push(String::new());
    }
    for paragraph in paragraphs {
        append_wrapped_lines(&mut lines, paragraph, MAX_CHARS_PER_LINE);
    }
    if lines.is_empty() {
        lines.push("暂无内容".to_string());
    }

    let lines_per_page = ((PAGE_HEIGHT - TOP_MARGIN - 52.0) / LINE_HEIGHT).floor() as usize;
    let pages = lines
        .chunks(lines_per_page.max(1))
        .map(|chunk| chunk.to_vec())
        .collect::<Vec<_>>();

    build_pdf(&pages)
}

fn append_wrapped_lines(out: &mut Vec<String>, text: &str, max_chars: usize) {
    if text.trim().is_empty() {
        out.push(String::new());
        return;
    }

    for raw_line in text.replace("\r\n", "\n").split('\n') {
        let line = raw_line.trim_end();
        if line.is_empty() {
            out.push(String::new());
            continue;
        }

        let mut current = String::new();
        for ch in line.chars() {
            current.push(ch);
            if visual_len(&current) >= max_chars {
                out.push(current.trim_end().to_string());
                current.clear();
            }
        }
        if !current.trim().is_empty() {
            out.push(current.trim_end().to_string());
        }
    }
    out.push(String::new());
}

fn visual_len(s: &str) -> usize {
    s.chars().map(|ch| if ch.is_ascii() { 1 } else { 2 }).sum()
}

fn build_pdf(pages: &[Vec<String>]) -> Vec<u8> {
    let mut objects: Vec<Vec<u8>> = Vec::new();
    let kids = (0..pages.len())
        .map(|i| format!("{} 0 R", 4 + i * 2))
        .collect::<Vec<_>>()
        .join(" ");

    objects.push(b"<< /Type /Catalog /Pages 2 0 R >>".to_vec());
    objects.push(format!("<< /Type /Pages /Kids [{kids}] /Count {} >>", pages.len()).into_bytes());
    objects.push(b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec());

    for (idx, page_lines) in pages.iter().enumerate() {
        let page_id = 4 + idx * 2;
        let content_id = page_id + 1;
        let page = format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {PAGE_WIDTH:.0} {PAGE_HEIGHT:.0}] /Resources << /Font << /F1 3 0 R >> >> /Contents {content_id} 0 R >>"
        );
        objects.push(page.into_bytes());
        objects.push(build_page_stream(page_lines));
    }

    let mut out = Vec::new();
    out.extend_from_slice(b"%PDF-1.4\n%\xE2\xE3\xCF\xD3\n");
    let mut offsets = vec![0usize; objects.len() + 1];
    for (idx, obj) in objects.iter().enumerate() {
        let id = idx + 1;
        offsets[id] = out.len();
        out.extend_from_slice(format!("{id} 0 obj\n").as_bytes());
        out.extend_from_slice(obj);
        out.extend_from_slice(b"\nendobj\n");
    }

    let xref_pos = out.len();
    out.extend_from_slice(format!("xref\n0 {}\n", objects.len() + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_pos}\n%%EOF\n",
            objects.len() + 1
        )
        .as_bytes(),
    );
    out
}

fn build_page_stream(lines: &[String]) -> Vec<u8> {
    let mut content = String::new();
    content.push_str("BT\n");
    content.push_str(&format!("/F1 {FONT_SIZE:.0} Tf\n"));
    content.push_str(&format!("{LINE_HEIGHT:.0} TL\n"));
    content.push_str(&format!(
        "1 0 0 1 {LEFT_MARGIN:.0} {:.0} Tm\n",
        PAGE_HEIGHT - TOP_MARGIN
    ));
    for line in lines {
        if line.is_empty() {
            content.push_str("T*\n");
        } else {
            content.push('<');
            content.push_str(&utf16be_hex(line));
            content.push_str("> Tj\nT*\n");
        }
    }
    content.push_str("ET\n");

    let bytes = content.into_bytes();
    let mut stream = format!("<< /Length {} >>\nstream\n", bytes.len()).into_bytes();
    stream.extend_from_slice(&bytes);
    stream.extend_from_slice(b"endstream");
    stream
}

fn utf16be_hex(text: &str) -> String {
    let mut hex = String::with_capacity(4 + text.len() * 4);
    hex.push_str("FEFF");
    for unit in text.encode_utf16() {
        hex.push_str(&format!("{unit:04X}"));
    }
    hex
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
        assert!(!epub_chapter.content.contains("<body"));
    }

    #[test]
    fn epub_chapter_allows_missing_or_blank_title() {
        for title in [None, Some("  ".to_string())] {
            let chapter = ExportChapter {
                id: 1,
                chapter_number: 3,
                title,
                content: "正文".to_string(),
            };

            let epub_chapter = epub_chapter_from_export_chapter(&chapter, 1);

            assert_eq!(epub_chapter.title, "第3章");
            assert_eq!(epub_chapter.content, "正文");
        }
    }

    #[test]
    fn epub_builder_accepts_export_chapters_for_batch_export() {
        let chapters = vec![
            ExportChapter {
                id: 1,
                chapter_number: 1,
                title: Some("开端".to_string()),
                content: "第一章正文".to_string(),
            },
            ExportChapter {
                id: 2,
                chapter_number: 2,
                title: Some("转折".to_string()),
                content: "第二章正文".to_string(),
            },
        ];
        let mut seen = Vec::new();
        let mut builder = new_epub_builder("测试书名", "测试作者");

        add_export_chapters_to_epub(&mut builder, &chapters, |index, chapter| {
            seen.push((index, chapter.chapter_number));
            Ok(())
        })
        .expect("chapters should be accepted");

        assert_eq!(seen, vec![(0, 1), (1, 2)]);
        let epub_data = builder.build().expect("epub should build");
        assert!(!epub_data.is_empty());
        assert_eq!(&epub_data[0..2], b"PK");
    }

    #[test]
    fn normalize_batch_formats_rejects_empty_unknown_and_duplicate_formats() {
        for formats in [
            Vec::<String>::new(),
            vec![" ".to_string()],
            vec!["docx".to_string()],
            vec!["md".to_string(), "markdown".to_string()],
            vec!["epub".to_string(), "EPUB".to_string()],
        ] {
            normalize_batch_formats(&formats).expect_err("formats should be rejected");
        }
    }

    #[test]
    fn normalize_batch_formats_accepts_supported_formats() {
        let formats = vec![
            "txt".to_string(),
            "MD".to_string(),
            "epub".to_string(),
            "json".to_string(),
            "pdf".to_string(),
        ];

        assert_eq!(
            normalize_batch_formats(&formats),
            Ok(vec!["txt", "markdown", "epub", "json", "pdf"])
        );
    }

    #[test]
    fn export_markdown_input_rejects_unknown_fields() {
        let err = serde_json::from_value::<ExportMarkdownInput>(serde_json::json!({
            "title": "测试书名",
            "author": "测试作者",
            "project_id": 42,
        }))
        .expect_err("unknown markdown input fields should be rejected");

        assert!(err.to_string().contains("unknown field"));
    }

    #[test]
    fn batch_export_input_rejects_unknown_fields() {
        let err = serde_json::from_value::<BatchExportInput>(serde_json::json!({
            "project_id": 42,
            "formats": ["txt"],
            "title": "测试书名",
            "author": "测试作者",
            "output_path": "/tmp/export.txt",
        }))
        .expect_err("unknown batch input fields should be rejected");

        assert!(err.to_string().contains("unknown field"));
    }
}
