use super::{
    contracts::{ExportFileData, ExportProgress},
    epub::{export_chapter_count_total, load_export_chapters},
    rules::required_text,
    scopes::export_pdf_task_scope,
    text::export_binary_file_data,
};
use crate::{AppState, actions::app_error_message};
use a4n_orchestrator::export::{ExportChapter, chapter_heading};

mod encoder;

use encoder::text_pdf_bytes;

pub async fn export_pdf_data(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    progress: &ExportProgress<'_>,
) -> Result<ExportFileData, String> {
    export_pdf_task_scope(state, user_id, project_id).await?;
    progress.progress(0, 5, "正在读取项目信息");
    progress.check_cancelled("PDF 导出已停止")?;
    let title = state
        .orchestrator
        .project_title(project_id)
        .await
        .map_err(app_error_message)?;
    let project_title = required_text(&title, "项目标题")?;
    let chapters = load_export_chapters(state, project_id, progress, 5, "PDF 导出已停止").await?;
    let total = export_chapter_count_total(chapters.len(), 5)?;
    progress.progress(
        2,
        total,
        &format!("已读取 {} 章，正在排版 PDF", chapters.len()),
    );
    let mut current = 2;
    let paragraphs =
        render_pdf_paragraphs(&chapters, progress, &mut current, total, "PDF 导出已停止")?;
    progress.check_cancelled("PDF 导出已停止")?;
    let encoding_progress = total
        .checked_sub(1)
        .ok_or_else(|| "PDF 导出总进度不足以表示编码阶段".to_string())?;
    progress.progress(encoding_progress, total, "正在生成 PDF 文件");
    let pdf = text_pdf_bytes(project_title, &paragraphs);
    progress.progress(total, total, "PDF 导出完成");
    Ok(export_binary_file_data(
        project_id,
        "pdf",
        "application/pdf",
        pdf,
    ))
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
            paragraphs.push(chapter_heading(ch.chapter_number, ch.title.as_deref()));
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
        *current = current
            .checked_add(1)
            .ok_or_else(|| "PDF 导出进度超过 u32 上限".to_string())?;
        progress.progress(
            *current,
            total,
            &format!("正在{action} 第{}章", chapter.chapter_number),
        );
        visit(chapter)?;
    }
    Ok(())
}
