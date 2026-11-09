use a4n_domain::{manga::*, source_spans::SourceSpan};
use a4n_error::AppError;
use a4n_llm_context::LlmContextManifest;
use serde_json::{Value, json};

use super::common::build_manga_step_user_message;
use crate::source_markers::SourceMarkerMap;

pub(in crate::manga) fn manga_preprocessing_summary(
    preprocessing: &MangaPreprocessingV3,
) -> Result<Value, AppError> {
    let items = preprocessing.items["items"]
        .as_array()
        .ok_or_else(|| AppError::InvalidArgument("manga_preprocessing.items 必须是数组".into()))?;
    Ok(json!({
        "source_events": preprocessing.source_events,
        "characters": preprocessing.characters,
        "speech": preprocessing.speech,
        "visual_anchors": preprocessing.visual_anchors,
        "items": items,
        "comic_synthesis": preprocessing.comic_synthesis,
    }))
}

pub(in crate::manga) struct MangaPreprocessingMessages<'a> {
    pub chapter_content: &'a str,
    pub source_spans: &'a [SourceSpan],
    pub source_markers: &'a SourceMarkerMap,
    pub reference_assets: &'a Value,
}

impl MangaPreprocessingMessages<'_> {
    pub fn build(
        &self,
        step: MangaPreprocessingStep,
        facts: Option<&MangaPreprocessingFacts>,
        speech: Option<&MangaPreprocessingSpeech>,
        visuals: Option<&MangaPreprocessingVisuals>,
    ) -> Result<LlmContextManifest, AppError> {
        let input = match step {
            MangaPreprocessingStep::Facts => json!({
                "chapter": {"content": self.source_markers.mark_text(self.chapter_content, self.source_spans)?},
                "reference_assets": self.reference_assets,
            }),
            MangaPreprocessingStep::Speech | MangaPreprocessingStep::Visuals => json!({
                "chapter": {"content": self.source_markers.mark_text(self.chapter_content, self.source_spans)?},
                "facts": facts.ok_or_else(|| missing_dependency(step))?,
            }),
            MangaPreprocessingStep::Synthesis => {
                let facts = facts.ok_or_else(|| missing_dependency(step))?;
                let visuals = visuals.ok_or_else(|| missing_dependency(step))?;
                json!({
                    "source_events": facts.source_events,
                    "characters": facts.characters.as_object().map(|characters| characters.keys().collect::<Vec<_>>()),
                    "speech": speech.ok_or_else(|| missing_dependency(step))?.speech,
                    "scenes": visuals.visual_anchors["scenes"].as_array().map(|scenes| scenes.iter().map(|scene| json!({"location": scene["location"], "source_event_ids": scene["source_event_ids"]})).collect::<Vec<_>>()),
                    "items": visuals.items["items"].as_array().map(|items| items.iter().map(|item| json!({"name": item["name"], "first_appearance_source_event_id": item["first_appearance_source_event_id"]})).collect::<Vec<_>>()),
                })
            }
        };
        build_manga_step_user_message(
            step.prompt_name(),
            step.title(),
            self.source_markers.encode_input(&input)?,
            json!({
                "format": "json_object", "prompt_name": step.prompt_name(), "output_schema": step.output_schema(),
                "source_rule": if step == MangaPreprocessingStep::Facts {
                    "根据正文建立唯一事件索引，原文来源只使用正文短标签，不得编造来源或输出其他步骤字段。"
                } else {
                    "原文来源只使用正文短标签，事件引用只能使用输入事实索引中的 event_id。不得编造来源、重建事件索引或输出其他步骤字段。"
                },
                "no_page_level_rule": "不输出 unit_id、beat_id、页数、改编取舍或分页内容。",
            }),
        )
    }
}

fn missing_dependency(step: MangaPreprocessingStep) -> AppError {
    AppError::InvalidArgument(format!("{}缺少已校验的上游资料", step.title()))
}
