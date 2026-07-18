use super::{
    contracts::{ExportFileData, ExportProgress},
    rules::required_text,
    scopes::export_epub_task_scope,
    text::export_binary_file_data,
};
use crate::AppState;
use a4n_export_core::epub::{EpubBuilder, EpubChapter, EpubMetadata, builtin_epub_stylesheet};
use a4n_orchestrator::export::{ExportChapter, chapter_heading};

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

async fn build_epub_data(
    state: &AppState,
    project_id: i64,
    title: &str,
    author: &str,
    progress: &ExportProgress<'_>,
) -> Result<Vec<u8>, String> {
    let chapters = load_export_chapters(state, project_id, progress, 4, "EPUB 导出已停止").await?;
    let total = export_chapter_count_total(chapters.len(), 4)?;
    progress.progress(
        1,
        total,
        &format!("已读取 {} 章，正在生成 EPUB", chapters.len()),
    );

    let mut builder = new_epub_builder(title, author);
    add_export_chapters_to_epub(&mut builder, &chapters, |i, ch| {
        progress.check_cancelled("EPUB 导出已停止")?;
        progress.progress(
            u32::try_from(i)
                .map_err(|_| format!("EPUB 章节索引超过 u32 上限：{i}"))?
                .checked_add(2)
                .ok_or_else(|| format!("EPUB 导出进度超过 u32 上限：章节索引 {i}"))?,
            total,
            &format!("正在写入 EPUB 第{}章", ch.chapter_number),
        );
        Ok(())
    })?;

    progress.check_cancelled("EPUB 导出已停止")?;
    let packaging_progress = total
        .checked_sub(2)
        .ok_or_else(|| "EPUB 导出总进度不足以表示打包阶段".to_string())?;
    progress.progress(packaging_progress, total, "正在打包 EPUB");
    builder.build().map_err(|e| e.to_string())
}

pub(super) async fn load_export_chapters(
    state: &AppState,
    project_id: i64,
    progress: &ExportProgress<'_>,
    total: u32,
    cancel_message: &str,
) -> Result<Vec<ExportChapter>, String> {
    progress.progress(0, total, "正在读取可导出章节");
    progress.check_cancelled(cancel_message)?;
    let chapters = state
        .orchestrator
        .export_chapters(project_id)
        .await
        .map_err(|e| e.to_string())?;
    progress.check_cancelled(cancel_message)?;
    Ok(chapters)
}

pub(super) fn export_chapter_count_total(
    chapter_count: usize,
    extra_steps: u32,
) -> Result<u32, String> {
    let chapter_count = u32::try_from(chapter_count)
        .map_err(|_| format!("导出章节数超过 u32 上限：{chapter_count}"))?;
    chapter_count
        .checked_add(extra_steps)
        .map(|total| total.max(1))
        .ok_or_else(|| format!("导出总进度超过 u32 上限：{chapter_count} + {extra_steps}"))
}

pub(super) fn epub_chapter_from_export_chapter(
    chapter: &ExportChapter,
    order: usize,
) -> EpubChapter {
    EpubChapter {
        title: chapter_heading(chapter.chapter_number, chapter.title.as_deref()),
        content: chapter.content.clone(),
        order,
    }
}

pub(super) fn new_epub_builder(title: &str, author: &str) -> EpubBuilder {
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

pub(super) fn add_export_chapters_to_epub(
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
