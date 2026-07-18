use a4n_core::models::chapter::chapter;
use a4n_orchestrator::video::{ChapterVideoSource, VideoPromptRequestPreview};

use super::super::types::{
    GeneratePromptOptionsInput, PREVIEW_CHAPTER_CONTENT_PLACEHOLDER,
    PREVIEW_CHAPTER_TITLE_PLACEHOLDER,
};

pub(in crate::actions::video_prompts) fn prepare_chapter_prompt_preview_inputs(
    chapter: &mut chapter::Model,
    source: &mut ChapterVideoSource,
) -> Vec<String> {
    let mut notes = Vec::new();
    if chapter
        .title
        .as_deref()
        .map(str::trim)
        .unwrap_or_default()
        .is_empty()
    {
        chapter.title = Some(PREVIEW_CHAPTER_TITLE_PLACEHOLDER.to_string());
        notes.push("章节标题为空，预览使用标题占位；真实生成仍需填写章节标题".to_string());
    }
    if source.content.trim().is_empty() {
        source.content = PREVIEW_CHAPTER_CONTENT_PLACEHOLDER.to_string();
        notes.push("章节正文为空，预览使用正文占位；真实生成仍需填写章节正文".to_string());
    }
    notes
}

pub(in crate::actions::video_prompts) fn annotate_preview_placeholder_inputs(
    mut preview: VideoPromptRequestPreview,
    notes: &[String],
) -> VideoPromptRequestPreview {
    if notes.is_empty() {
        return preview;
    }
    let note = notes.join("；");
    for step in &mut preview.steps {
        step.note = Some(match step.note.take() {
            Some(existing) if !existing.trim().is_empty() => format!("{note}；{existing}"),
            _ => note.clone(),
        });
    }
    preview
}

pub(in crate::actions::video_prompts) fn append_prompt_option_preview_notes(
    input: &GeneratePromptOptionsInput,
    notes: &mut Vec<String>,
) {
    if input.style.trim().is_empty() {
        notes.push("视频风格 style 为空，预览使用风格占位；真实生成仍需填写 style".to_string());
    }
    if input.regenerate_existing {
        notes.push("已启用重新生成模式，预览会按跳过现有目标提示词复用的路径展示".to_string());
    }
}
