use super::generic::{lineage_ref, optional_prompt_sources, prompt_source};
use super::json_records::db_record_result;
use crate::project_dependency_graph::helpers::prompt_type_str_in;
use crate::project_dependency_graph::{
    AssetLineageRecord, PromptSource, VIDEO_FRAME_PROMPT_TYPES, VIDEO_SEGMENT_PLAN_PROMPT_TYPES,
};
use a4n_db_models::models::chapter::chapter_video_prompt;
use a4n_domain::GeneratedMediaReference;
use sea_orm::FromQueryResult;
use serde_json::Value;

#[derive(Debug, Clone, FromQueryResult)]
pub(in crate::project_dependency_graph) struct VideoPromptSourceRecord {
    pub id: i64,
    pub visual_prompt: String,
    pub motion_prompt: Option<String>,
    pub negative_prompt: Option<String>,
}

pub(in crate::project_dependency_graph) fn video_prompt_sources(
    prompt: &chapter_video_prompt::Model,
) -> Vec<PromptSource> {
    video_prompt_sources_from_fields(
        prompt.id,
        &prompt.visual_prompt,
        prompt.motion_prompt.as_deref(),
        prompt.negative_prompt.as_deref(),
    )
}

pub(in crate::project_dependency_graph) fn video_prompt_sources_from_record(
    prompt: &VideoPromptSourceRecord,
) -> Vec<PromptSource> {
    video_prompt_sources_from_fields(
        prompt.id,
        &prompt.visual_prompt,
        prompt.motion_prompt.as_deref(),
        prompt.negative_prompt.as_deref(),
    )
}

fn video_prompt_sources_from_fields(
    id: i64,
    visual_prompt: &str,
    motion_prompt: Option<&str>,
    negative_prompt: Option<&str>,
) -> Vec<PromptSource> {
    optional_prompt_sources(vec![
        prompt_source(
            &format!("chapter_video_visual_prompt:{id}"),
            "video_visual_prompt",
            Some(visual_prompt),
            Some(id),
            None,
        ),
        prompt_source(
            &format!("chapter_video_motion_prompt:{id}"),
            "video_motion_prompt",
            motion_prompt,
            Some(id),
            None,
        ),
        prompt_source(
            &format!("chapter_video_negative_prompt:{id}"),
            "video_negative_prompt",
            negative_prompt,
            Some(id),
            None,
        ),
    ])
}

pub(in crate::project_dependency_graph) fn video_prompt_record(
    prompt: chapter_video_prompt::Model,
) -> AssetLineageRecord {
    AssetLineageRecord {
        id: format!("chapter_video_prompt:{}", prompt.id),
        label: prompt.title.clone().unwrap_or_else(|| {
            format!(
                "第 {} 章 镜头 {} {}",
                prompt.chapter_id, prompt.shot_number, prompt.prompt_type
            )
        }),
        asset_kind: video_prompt_asset_kind(&prompt.prompt_type).to_string(),
        asset_id: Some(prompt.id),
        status: Some(prompt.status.clone()),
        origin: "llm_generated".to_string(),
        source_refs: Vec::new(),
        prompt_sources: video_prompt_sources(&prompt),
        media_sources: prompt
            .reference_image_ids_json
            .as_ref()
            .and_then(Value::as_array)
            .map(|ids| {
                ids.iter()
                    .filter_map(Value::as_i64)
                    .map(|id| {
                        lineage_ref(
                            &GeneratedMediaReference::Image(id).to_string(),
                            "参考图片",
                            "image",
                            "generated_image",
                            Some(id),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default(),
        request_snapshot: prompt
            .metadata_json
            .as_ref()
            .and_then(|value| value.get("provider_request_snapshot").cloned()),
        input_status: "missing".to_string(),
        generation_inputs: Vec::new(),
        result: db_record_result("chapter_video_prompt", &prompt),
        warnings: Vec::new(),
    }
}

pub(in crate::project_dependency_graph) fn video_prompt_asset_kind(
    prompt_type: &str,
) -> &'static str {
    if prompt_type_str_in(prompt_type, VIDEO_SEGMENT_PLAN_PROMPT_TYPES) {
        "video_segment_plan"
    } else if prompt_type_str_in(prompt_type, VIDEO_FRAME_PROMPT_TYPES) {
        "video_frame_prompt"
    } else {
        "chapter_video_prompt"
    }
}
