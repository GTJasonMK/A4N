use a4n_domain::json_contract::positive_i64_array_schema;
use a4n_error::AppError;
use serde_json::{Value, json};

use a4n_llm_provider_contract::provider::ChatRequest;

use super::super::super::parsing::parse_scene_occurrences;
use crate::scene_profile::{SceneOccurrenceDraft, SceneSyncInput};

use super::super::super::progress::{SCENE_OCCURRENCE_PROGRESS_FIELDS, scene_stage_progress_spec};
use super::super::super::types::{
    SCENE_OCCURRENCE_COVERAGE_RULE, SCENE_OCCURRENCE_PROMPT, SCENE_OCCURRENCE_TIMELINE_RULE,
};
use super::super::SceneProfileStageContext;
use super::super::execution::run_scene_profile_stage_with_checkpoint;
use super::super::request::{
    build_scene_profile_stage_request, scene_profile_stage_step_key, scene_profile_stage_step_order,
};

pub(super) async fn run(
    context: &mut SceneProfileStageContext<'_, '_>,
    scenes: &[Value],
    known: &SceneSyncInput,
) -> Result<(ChatRequest, Vec<Value>, Vec<SceneOccurrenceDraft>), AppError> {
    let request_context = context.request_context;
    let known_profiles = &known.profiles;
    let mut request = build_scene_profile_stage_request(
        context.prompt_renderer,
        request_context,
        Some(context.workflow_run_id),
        SCENE_OCCURRENCE_PROMPT,
        "基于已确认场景抽取当前章节中的场景出场，只输出 occurrences",
        json!({ "scenes": scenes }),
        json!({
            "format": "json_object",
            "required_root_field": "occurrences",
            "field_types": {
                "occurrence_ref":{"type":"string","minLength":1}, "scene_ref":{"type":"string","minLength":1},
                "timeline_position":{"type":["string","null"]}, "summary":{"type":"string","minLength":1},
                "mood":{"type":["string","null"]}, "weather_or_lighting":{"type":["string","null"]},
                "characters":{"type":"array","items":{"type":"object","properties":{"profile_id":a4n_domain::json_contract::positive_i64_schema(),"role":{"type":"string","minLength":1}},"required":["profile_id","role"]}},
                "objects":{"type":"array","items":{"type":"object","properties":{"object_profile_id":a4n_domain::json_contract::positive_i64_schema(),"role":{"type":"string","minLength":1}},"required":["object_profile_id","role"]}},
                "key_events":{"type":"array","items":{"type":"string"}}, "entry_state":{"type":"object"}, "exit_state":{"type":"object"},
                "source_anchor_ids": positive_i64_array_schema(true)
            },
            "strict_rules": [
                "只返回 occurrences 数组，不要输出 scenes、transitions 或 entity_states",
                "scene_ref 必须来自 input.previous_stages.scenes[].scene_ref",
                SCENE_OCCURRENCE_COVERAGE_RULE,
                SCENE_OCCURRENCE_TIMELINE_RULE,
                "source_anchor_ids 必须选择 chapter.content 内 ⟦id⟧ 标记的 ID，不含括号"
            ]
        }),
    )
    .await?;
    let step_key = scene_profile_stage_step_key(request_context, "occurrences");
    let step_order = scene_profile_stage_step_order(context.chapter_index, 1)?;
    let stage_progress = scene_stage_progress_spec(
        "occurrences",
        "场景出场",
        SCENE_OCCURRENCE_PROGRESS_FIELDS,
        context.model_span.split(1, 4),
    );
    let (output, parsed) = run_scene_profile_stage_with_checkpoint(
        context,
        &mut request,
        SCENE_OCCURRENCE_PROMPT,
        step_key,
        step_order,
        stage_progress,
        |value| parse_scene_occurrences(value, request_context, known_profiles),
    )
    .await?;
    Ok((request, output, parsed))
}
