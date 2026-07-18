use crate::OrchestratorContext;
use crate::project::ImportChapterOutlineUpsertInput;
use a4n_core::{
    chapter_analysis::summary_text,
    models::chapter::{chapter, chapter_outline},
};

pub async fn importable_chapters(
    context: &OrchestratorContext,
    project_id: i64,
) -> Result<Vec<chapter::Model>, String> {
    context
        .import_analysis_chapter_candidates(project_id)
        .await
        .map_err(|err| err.to_string())
        .map(|chapters| {
            chapters
                .into_iter()
                .filter(|ch| !ch.content.trim().is_empty())
                .collect()
        })
}

pub fn required_chapter_title(ch: &chapter::Model) -> Result<&str, String> {
    ch.title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
        .ok_or_else(|| {
            format!(
                "第{}章缺少章节标题，无法生成导入章节大纲",
                ch.chapter_number
            )
        })
}

fn is_generated_summary_placeholder(summary: &str, chapter_number: i32) -> bool {
    let summary = summary.trim();
    summary.is_empty()
        || summary.contains("摘要待生成")
        || summary.contains("待分析")
        || summary == format!("第{chapter_number}章")
        || (summary.starts_with(&format!("第{chapter_number}章：")) && summary.ends_with('字'))
}

pub fn chapter_outline_summary(ch: &chapter::Model) -> Result<String, String> {
    ch.analysis_data
        .as_ref()
        .and_then(summary_text)
        .or_else(|| {
            ch.summary
                .as_deref()
                .map(str::trim)
                .filter(|summary| !is_generated_summary_placeholder(summary, ch.chapter_number))
                .map(str::to_string)
        })
        .ok_or_else(|| {
            format!(
                "第{}章缺少结构化摘要，无法生成导入章节大纲",
                ch.chapter_number
            )
        })
}

pub async fn upsert_outline_from_chapter(
    context: &OrchestratorContext,
    ch: &chapter::Model,
) -> Result<chapter_outline::Model, String> {
    let title = required_chapter_title(ch)?.to_string();
    let summary = chapter_outline_summary(ch)?;
    let analysis = ch.analysis_data.as_ref();

    context
        .upsert_import_chapter_outline(ImportChapterOutlineUpsertInput {
            project_id: ch.project_id,
            chapter_number: ch.chapter_number,
            title,
            summary,
            key_scenes: super::analysis::analysis_value_path(
                analysis,
                &["key_events"],
                ch.chapter_number,
            )?,
            foreshadowing_setup: super::analysis::analysis_value_path(
                analysis,
                &["foreshadowing", "planted"],
                ch.chapter_number,
            )?,
            foreshadowing_payoff: super::analysis::analysis_value_path(
                analysis,
                &["foreshadowing", "resolved"],
                ch.chapter_number,
            )?,
            characters_involved: super::analysis::analysis_metadata_array(
                analysis,
                "characters",
                ch.chapter_number,
            )?,
            word_count_target: ch.word_count,
        })
        .await
        .map_err(|err| err.to_string())
}
