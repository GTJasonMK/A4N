use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_manga_prompt;
use serde_json::{Value, json};

use super::output_sync::MangaPromptDraft;
use super::pipeline_json::{PageDrawingPrompt, optional_value_field};
use super::pipeline_prompt::MangaPromptGenerationTrace;
use super::reference_assets::page_reference_bindings;
use super::{
    ChapterMangaSourceContext, MANGA_EXTRACTION_SUMMARY_TYPE, MANGA_PAGE_PROMPT_RAW_TYPE,
    MANGA_PAGE_PROMPT_TYPE, MangaService, with_manga_source_metadata,
};
use serde::{Deserialize, Serialize};

pub(super) struct MangaPipelinePersistInput<'a> {
    pub(super) chapter_id: i64,
    pub(super) project_id: i64,
    pub(super) source: ChapterMangaSourceContext<'a>,
    pub(super) source_metadata: &'a Value,
    pub(super) reference_assets_metadata: &'a Value,
    pub(super) prompt_fingerprint: &'a Value,
    pub(super) extraction_summary_out: &'a str,
    pub(super) extraction_summary: &'a Value,
    pub(super) characters_json: &'a Value,
    pub(super) events_json: &'a Value,
    pub(super) stage_traces: Vec<MangaPromptGenerationTrace>,
    pub(super) plan_out: &'a str,
    pub(super) plan_trace: Option<MangaPromptGenerationTrace>,
    pub(super) plan_json: &'a Value,
    pub(super) dp_out: &'a str,
    pub(super) dp_json: &'a Value,
    pub(super) page_traces: Vec<PageMangaPromptGenerationTrace>,
    pub(super) page_prompts: Vec<PageDrawingPrompt<'a>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct PageMangaPromptGenerationTrace {
    pub(super) page_number: i32,
    pub(super) trace: MangaPromptGenerationTrace,
}

impl MangaService {
    pub(super) async fn persist_pipeline_outputs(
        &self,
        input: MangaPipelinePersistInput<'_>,
    ) -> Result<Vec<chapter_manga_prompt::Model>, AppError> {
        let mut drafts = vec![
            MangaPromptDraft {
                chapter_id: input.chapter_id,
                project_id: input.project_id,
                page_number: 0,
                panel_number: 0,
                prompt_type: MANGA_EXTRACTION_SUMMARY_TYPE.to_string(),
                image_prompt: input.extraction_summary_out.to_string(),
                dialogue_text: None,
                characters_in_frame: None,
                layout: Some(json!({
                    "raw": input.extraction_summary,
                    "characters": input.characters_json,
                    "events": input.events_json,
                    "reference_assets": input.reference_assets_metadata,
                    "prompt_fingerprint": input.prompt_fingerprint,
                    "source": input.source_metadata,
                })),
                camera_angle: None,
                dialogue_position: None,
                status: "completed".to_string(),
                generation_traces: input.stage_traces.clone(),
            },
            MangaPromptDraft {
                chapter_id: input.chapter_id,
                project_id: input.project_id,
                page_number: 0,
                panel_number: 0,
                prompt_type: "page_planning".to_string(),
                image_prompt: input.plan_out.to_string(),
                dialogue_text: None,
                characters_in_frame: None,
                layout: Some(json!({
                    "raw": input.plan_json,
                    "reference_assets": input.reference_assets_metadata,
                    "prompt_fingerprint": input.prompt_fingerprint,
                    "source": input.source_metadata,
                })),
                camera_angle: None,
                dialogue_position: None,
                status: "completed".to_string(),
                generation_traces: input.plan_trace.clone().into_iter().collect(),
            },
            MangaPromptDraft {
                chapter_id: input.chapter_id,
                project_id: input.project_id,
                page_number: 0,
                panel_number: 0,
                prompt_type: MANGA_PAGE_PROMPT_RAW_TYPE.to_string(),
                image_prompt: input.dp_out.to_string(),
                dialogue_text: None,
                characters_in_frame: None,
                layout: Some(json!({
                    "raw": input.dp_json,
                    "reference_assets": input.reference_assets_metadata,
                    "prompt_fingerprint": input.prompt_fingerprint,
                    "source": input.source_metadata,
                })),
                camera_angle: None,
                dialogue_position: None,
                status: "completed".to_string(),
                generation_traces: input
                    .page_traces
                    .iter()
                    .map(|trace| trace.trace.clone())
                    .collect(),
            },
        ];

        for page in input.page_prompts {
            let generation_trace = input
                .page_traces
                .iter()
                .find(|trace| trace.page_number == page.page_number)
                .map(|trace| trace.trace.clone());
            let page_plan = manga_page_plan_by_number(input.plan_json, page.page_number);
            let reference_source = page_plan.unwrap_or(page.raw);
            let layout = with_manga_source_metadata(
                Some(json!({
                    "raw": page.raw,
                    "page_plan": page_plan,
                    "reference_assets": input.reference_assets_metadata,
                    "prompt_fingerprint": input.prompt_fingerprint,
                    "reference_bindings": page_reference_bindings(reference_source, input.reference_assets_metadata),
                })),
                input.source,
            );
            self.ensure_pipeline_not_cancelled(input.project_id, input.chapter_id)
                .await?;
            drafts.push(MangaPromptDraft {
                chapter_id: input.chapter_id,
                project_id: input.project_id,
                page_number: page.page_number,
                panel_number: 0,
                prompt_type: MANGA_PAGE_PROMPT_TYPE.to_string(),
                image_prompt: page.image_prompt.to_string(),
                dialogue_text: page.dialogue_text.map(ToString::to_string),
                characters_in_frame: page_plan
                    .and_then(|plan| optional_value_field(plan, "key_characters")),
                layout,
                camera_angle: None,
                dialogue_position: None,
                status: "pending".to_string(),
                generation_traces: generation_trace.into_iter().collect(),
            });
        }

        Ok(self
            .sync_generated_outputs_by_chapter(input.project_id, input.chapter_id, drafts)
            .await?
            .into_iter()
            .filter(|row| row.prompt_type == MANGA_PAGE_PROMPT_TYPE)
            .collect())
    }
}

fn manga_page_plan_by_number(plan_json: &Value, page_number: i32) -> Option<&Value> {
    plan_json.get("pages")?.as_array()?.iter().find(|page| {
        page.get("page_number")
            .and_then(Value::as_i64)
            .is_some_and(|value| value == i64::from(page_number))
    })
}
