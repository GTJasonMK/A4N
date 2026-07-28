use a4n_core::{error::AppError, video_execution::VideoExecutionProjectionV2};
use a4n_llm::context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedgerPackDraft,
    LlmContextLedgerPackInput,
};
use serde_json::{Value, json};

use super::{
    LlmContextLedger, VIDEO_CONTEXT_LEDGER_VERSION, VIDEO_CONTEXT_SCOPE,
    VIDEO_PREVIOUS_OUTPUT_SHOOTING_SCRIPT, VideoSegmentPlanningContextInput,
    add_optional_value_pack, add_prompt_rule_hash_pack, source_content_sha256, trimmed_optional,
};
use crate::video::prompt_context::builders::chapter_analysis_context;

mod template;
pub(in crate::video) use template::VideoSegmentPlanningContextTemplate;

pub(in crate::video) async fn build_video_segment_planning_context(
    input: VideoSegmentPlanningContextInput<'_>,
) -> Result<VideoSegmentPlanningContextTemplate, AppError> {
    let VideoSegmentPlanningContextInput {
        prompt_renderer,
        project_id,
        variables,
        source,
        shooting_script_projection,
        project_context,
    } = input;
    let mut ledger = LlmContextLedger::new();
    add_segment_planning_chapter_pack(&mut ledger, source)?;
    add_segment_planning_continuity_pack(&mut ledger, shooting_script_projection)?;
    add_prompt_rule_hash_pack(
        &mut ledger,
        prompt_renderer,
        project_id,
        variables,
        &["chapter_video_segment_plan"],
    )
    .await?;
    if let Some(project_context) = project_context {
        add_optional_value_pack(
            &mut ledger,
            "video_project_context",
            json!({
                "ledger_version": VIDEO_CONTEXT_LEDGER_VERSION,
                "kind": "project_context"
            }),
            project_context,
        )?;
    }
    Ok(VideoSegmentPlanningContextTemplate::new(
        ledger.into_manifest(String::new(), None),
    ))
}

fn add_segment_planning_chapter_pack(
    ledger: &mut LlmContextLedger,
    source: &crate::video::ChapterVideoSourceContext<'_>,
) -> Result<(), AppError> {
    let content_sha256 = source_content_sha256(source.content);
    let full = segment_planning_chapter_context(source);
    let minimal = json!({
        "kind": "video_segment_planning_chapter",
        "content_sha256": content_sha256,
        "summary": trimmed_optional(source.summary)
    });
    ledger.add_pack(LlmContextLedgerPackInput {
        scope_kind: VIDEO_CONTEXT_SCOPE.into(),
        pack_kind: "video_segment_planning_chapter".into(),
        source_refs_json: Some(json!({"content_sha256": content_sha256})),
        config_identity_json: json!({
            "ledger_version": VIDEO_CONTEXT_LEDGER_VERSION,
            "kind": "video_segment_planning_chapter",
            "content_sha256": content_sha256
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(COMPRESSION_FULL_RAW, full.clone()),
            LlmContextLedgerPackDraft::new(COMPRESSION_FOCUSED_RAW, full),
            LlmContextLedgerPackDraft::new(COMPRESSION_MINIMAL, minimal),
        ],
    })
}

fn segment_planning_chapter_context(source: &crate::video::ChapterVideoSourceContext<'_>) -> Value {
    json!({
        "kind": "video_segment_planning_chapter",
        "content_sha256": source_content_sha256(source.content),
        "summary": trimmed_optional(source.summary),
        "analysis": chapter_analysis_context(source.analysis)
    })
}

fn add_segment_planning_continuity_pack(
    ledger: &mut LlmContextLedger,
    projection: &VideoExecutionProjectionV2,
) -> Result<(), AppError> {
    let continuity = segment_planning_continuity(projection);
    let continuity_sha256 = source_content_sha256(&serde_json::to_string(&continuity)?);
    ledger.add_pack(LlmContextLedgerPackInput {
        scope_kind: VIDEO_CONTEXT_SCOPE.into(),
        pack_kind: VIDEO_PREVIOUS_OUTPUT_SHOOTING_SCRIPT.into(),
        source_refs_json: Some(json!({
            "record_id": projection.shooting_script_identity.record_id,
            "revision": projection.shooting_script_identity.revision,
            "script_sha256": projection.shooting_script_identity.script_sha256,
            "continuity_sha256": continuity_sha256
        })),
        config_identity_json: json!({
            "ledger_version": VIDEO_CONTEXT_LEDGER_VERSION,
            "kind": "video_segment_planning_continuity",
            "script_sha256": projection.shooting_script_identity.script_sha256,
            "continuity_sha256": continuity_sha256
        }),
        required: true,
        stable_prefix: true,
        variants: vec![LlmContextLedgerPackDraft::new(
            COMPRESSION_FULL_RAW,
            continuity,
        )],
    })
}

fn segment_planning_continuity(projection: &VideoExecutionProjectionV2) -> Value {
    json!({
        "schema_version": "video_segment_planning_continuity_v1",
        "source_name": "chapter_shooting_script_video_projection",
        "shooting_script_identity": projection.shooting_script_identity,
        "directing_statement": projection.directing_statement,
        "scenes": projection.scenes.iter().map(|scene| json!({
            "scene_index": scene.scene_index,
            "scene_identity": scene.scene_identity,
            "sequence_index": scene.sequence_index,
            "timeline_position": scene.timeline_position,
            "intercut_group_id": scene.intercut_group_id,
            "editing_mode": scene.editing_mode,
            "participants": scene.participants,
            "entry_state": scene.entry_state,
            "exit_state": scene.exit_state,
            "transition_contracts": scene.transition_contracts,
            "shots": scene.shots.iter().map(|shot| json!({
                "source_shot_id": shot.source_shot_id,
                "timeline_position": shot.timeline_position,
                "intercut_group_id": shot.intercut_group_id,
                "narrative_purpose": shot.narrative_purpose,
                "participants": shot.participants,
                "start_state": shot.start_state,
                "end_state": shot.end_state,
                "transition_in": shot.transition_in,
                "transition_out": shot.transition_out,
                "continuity_requirements": shot.continuity_requirements,
                "beat_ids": shot.action_beats.iter().map(|beat| &beat.beat_id).collect::<Vec<_>>()
            })).collect::<Vec<_>>()
        })).collect::<Vec<_>>()
    })
}

#[cfg(test)]
mod tests;
