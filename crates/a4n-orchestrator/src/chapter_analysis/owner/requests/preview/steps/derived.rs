use a4n_core::{
    chapter_analysis::{
        CHAPTER_ANALYSIS_STAGE_CHARACTER_STATE, CHAPTER_ANALYSIS_STAGE_FORESHADOWING_EVENTS,
        CHAPTER_ANALYSIS_STAGE_SUMMARY,
    },
    error::AppError,
    models::chapter::chapter,
};
use a4n_prompt::{chapter_analysis::ChapterAnalysisRequestStep, rendering::PromptRenderer};
use serde_json::Value;

use crate::chapter_analysis::owner::requests::stages::{
    build_chapter_character_state_request, build_chapter_foreshadowing_events_request,
    build_chapter_summary_synthesis_request,
};

pub(super) async fn build_derived_preview_steps(
    ch: &chapter::Model,
    prompt_renderer: &dyn PromptRenderer,
    facts_placeholder: &Value,
    pending_foreshadowing: &Value,
) -> Result<Vec<ChapterAnalysisRequestStep>, AppError> {
    Ok(vec![
        ChapterAnalysisRequestStep {
            key: CHAPTER_ANALYSIS_STAGE_SUMMARY.to_string(),
            title: "章节分析 2/4：摘要与标签综合".to_string(),
            prompt_name: CHAPTER_ANALYSIS_STAGE_SUMMARY.to_string(),
            request: build_chapter_summary_synthesis_request(
                ch,
                prompt_renderer,
                facts_placeholder,
            )
            .await?,
        },
        ChapterAnalysisRequestStep {
            key: CHAPTER_ANALYSIS_STAGE_CHARACTER_STATE.to_string(),
            title: "章节分析 3/4：角色状态快照".to_string(),
            prompt_name: CHAPTER_ANALYSIS_STAGE_CHARACTER_STATE.to_string(),
            request: build_chapter_character_state_request(ch, prompt_renderer, facts_placeholder)
                .await?,
        },
        ChapterAnalysisRequestStep {
            key: CHAPTER_ANALYSIS_STAGE_FORESHADOWING_EVENTS.to_string(),
            title: "章节分析 4/4：伏笔与关键事件".to_string(),
            prompt_name: CHAPTER_ANALYSIS_STAGE_FORESHADOWING_EVENTS.to_string(),
            request: build_chapter_foreshadowing_events_request(
                ch,
                prompt_renderer,
                facts_placeholder,
                pending_foreshadowing,
            )
            .await?,
        },
    ])
}
