use super::identity::should_keep_unmatched_prompt;
use super::*;
use a4n_db_models::models::{ai::generated_image, chapter::chapter_video_prompt};
#[cfg(test)]
use a4n_domain::video_prompt::ChapterVideoPromptRecordFields;
use a4n_domain::video_prompt::ChapterVideoPromptSyncDraft;
use a4n_domain::video_prompt::ChapterVideoPromptSyncPolicy;
use std::collections::BTreeSet;

pub(super) fn preserved_unmatched_prompt_ids_by_policy(
    existing_by_identity: &HashMap<PromptIdentity, chapter_video_prompt::Model>,
    draft_identities: &BTreeSet<PromptIdentity>,
    draft_scopes: &BTreeSet<PromptScope>,
    policy: &ChapterVideoPromptSyncPolicy,
    generated_image_prompt_ids: &BTreeSet<i64>,
) -> BTreeSet<i64> {
    if !policy.preserve_frame_prompts_with_generated_images {
        return BTreeSet::new();
    }
    let mut ids = BTreeSet::new();
    for (identity, row) in existing_by_identity {
        if draft_identities.contains(identity) || should_keep_unmatched_prompt(row, draft_scopes) {
            continue;
        }
        if is_sync_frame_prompt_type(&row.prompt_type)
            && generated_image_prompt_ids.contains(&row.id)
        {
            ids.insert(row.id);
        }
    }
    ids
}

pub(super) fn should_preserve_existing_prompt(
    existing: &chapter_video_prompt::Model,
    draft: &ChapterVideoPromptSyncDraft,
    policy: &ChapterVideoPromptSyncPolicy,
    generated_image_prompt_ids: &BTreeSet<i64>,
) -> bool {
    if policy.preserve_generated_video_prompts
        && existing.prompt_type == chapter_video_prompt::PROMPT_TYPE_VIDEO
        && draft.fields.prompt_type == chapter_video_prompt::PROMPT_TYPE_VIDEO
        && existing.generated_video_id.is_some()
        && !prompt_sync_draft_fields_match(existing, draft)
    {
        return true;
    }
    frame_prompt_preservation_candidate(existing, draft, policy)
        && generated_image_prompt_ids.contains(&existing.id)
}

pub(super) fn frame_prompt_preservation_candidate(
    existing: &chapter_video_prompt::Model,
    draft: &ChapterVideoPromptSyncDraft,
    policy: &ChapterVideoPromptSyncPolicy,
) -> bool {
    policy.preserve_frame_prompts_with_generated_images
        && existing.prompt_type == draft.fields.prompt_type
        && is_sync_frame_prompt_type(&existing.prompt_type)
        && !prompt_sync_draft_fields_match(existing, draft)
}

pub(super) fn generated_frame_image_prompt_ids(
    images: impl IntoIterator<Item = generated_image::Reference>,
) -> BTreeSet<i64> {
    images
        .into_iter()
        .filter_map(|image| {
            a4n_domain::image::image_generation_prompt_id(image.metadata_json.as_ref())
        })
        .collect()
}

pub(super) fn is_sync_frame_prompt_type(prompt_type: &str) -> bool {
    matches!(
        prompt_type,
        chapter_video_prompt::PROMPT_TYPE_FRAME
            | chapter_video_prompt::PROMPT_TYPE_FIRST_FRAME
            | chapter_video_prompt::PROMPT_TYPE_CHAT_VIDEO_FRAME
            | chapter_video_prompt::PROMPT_TYPE_STORYBOARD
    )
}

pub(super) fn prompt_sync_draft_fields_match(
    existing: &chapter_video_prompt::Model,
    draft: &ChapterVideoPromptSyncDraft,
) -> bool {
    let fields = &draft.fields;
    existing.project_id == fields.project_id
        && existing.chapter_id == fields.chapter_id
        && existing.style == fields.style
        && existing.prompt_type == fields.prompt_type
        && existing.workflow_mode == fields.workflow_mode
        && existing.shot_number == fields.shot_number
        && existing.title == fields.title
        && existing.scene_summary == fields.scene_summary
        && existing.visual_prompt == fields.visual_prompt
        && existing.motion_prompt == fields.motion_prompt
        && existing.negative_prompt == fields.negative_prompt
        && existing.duration_secs == fields.duration_secs
        && existing.aspect_ratio == fields.aspect_ratio
        && existing.reference_image_ids_json == fields.reference_image_ids_json
        && existing.metadata_json == fields.metadata_json
}

pub(super) fn prompt_sync_generation_fields_changed(
    existing: &chapter_video_prompt::Model,
    draft: &ChapterVideoPromptSyncDraft,
) -> bool {
    let fields = &draft.fields;
    existing.visual_prompt != fields.visual_prompt
        || existing.motion_prompt != fields.motion_prompt
        || existing.negative_prompt != fields.negative_prompt
        || existing.duration_secs != fields.duration_secs
        || existing.aspect_ratio != fields.aspect_ratio
        || existing.reference_image_ids_json != fields.reference_image_ids_json
        || !prompt_sync_metadata_generation_context_matches(
            existing.metadata_json.as_ref(),
            fields.metadata_json.as_ref(),
        )
}

pub(super) fn segment_plan_update_preserves_outputs(
    existing: &chapter_video_prompt::Model,
    draft: &ChapterVideoPromptSyncDraft,
) -> Result<bool, VideoDbError> {
    if existing.prompt_type != chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN
        || draft.fields.prompt_type != chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN
        || existing.workflow_mode != chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN
        || draft.fields.workflow_mode != chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN
        || existing.style != draft.fields.style
        || existing.duration_secs != draft.fields.duration_secs
        || existing.aspect_ratio != draft.fields.aspect_ratio
    {
        return Ok(false);
    }
    let existing_metadata = existing
        .metadata_json
        .as_ref()
        .ok_or_else(|| VideoDbError::InvalidArgument("已有片段规划缺少 metadata_json".into()))?;
    let draft_metadata = draft
        .fields
        .metadata_json
        .as_ref()
        .ok_or_else(|| VideoDbError::InvalidArgument("新片段规划缺少 metadata_json".into()))?;
    if existing_metadata.get("source") != draft_metadata.get("source")
        || existing_metadata.get("contract_version") != draft_metadata.get("contract_version")
    {
        return Ok(false);
    }
    // 更新共享规划不会覆盖下游记录；各片段凭内容和来源指纹自行判定是否过时。
    let style = existing
        .style
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| VideoDbError::InvalidArgument("片段规划缺少 style".into()))?;
    for metadata in [existing_metadata, draft_metadata] {
        let segments = a4n_video_contract::video_segment_plan_from_metadata(metadata)
            .map_err(VideoDbError::InvalidArgument)?;
        let recipes = a4n_video_contract::video_segment_recipes_from_metadata(Some(metadata))
            .map_err(VideoDbError::InvalidArgument)?;
        a4n_video_contract::validate_video_segment_recipes_for_plan(&recipes, &segments, style)
            .map_err(VideoDbError::InvalidArgument)?;
        a4n_video_contract::validate_video_segment_plan_identity(
            &metadata["video_execution_identity"],
            &segments,
        )
        .map_err(VideoDbError::InvalidArgument)?;
    }
    Ok(true)
}

fn prompt_sync_metadata_generation_context_matches(
    existing: Option<&Value>,
    draft: Option<&Value>,
) -> bool {
    existing.and_then(|metadata| metadata.get("source"))
        == draft.and_then(|metadata| metadata.get("source"))
        && existing
            .and_then(|metadata| metadata.get("contract_version"))
            .and_then(Value::as_str)
            == draft
                .and_then(|metadata| metadata.get("contract_version"))
                .and_then(Value::as_str)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn segment(segment_number: i32, source_shot_id: &str, title: &str) -> Value {
        json!({
            "schema_version": "video_segment_plan_v2",
            "segment_number": segment_number,
            "source_shot_id": source_shot_id,
            "segment_index_in_shot": 1,
            "shot_segment_count": 1,
            "covered_beat_ids": [],
            "title": title,
            "scene_summary": "场景",
            "blocking_summary": "调度",
            "spatial_anchors": [],
            "emotion_action_goals": [],
            "video_intent": {
                "hook": "h",
                "audience_effect": "a",
                "rhythm": "r",
                "visual_priority": "v"
            },
            "emotional_arc": {"start": "s", "turn": "t", "end": "e", "function": "f"},
            "performance_plan": {
                "movement_path": "m",
                "body_performance": "b",
                "gaze_and_hands": "g",
                "pause_or_reaction": "p",
                "camera_relationship": "c",
                "time_blocks": [],
                "action_script": []
            },
            "material_coverage": {
                "required_entities": [],
                "missing_or_weak_references": [],
                "risk_note": "r"
            },
            "transition_to_next": null,
            "source_span_ids": ["span-1"],
            "duration_secs": 6,
            "aspect_ratio": "16:9",
            "reference_image_ids": []
        })
    }

    fn recipe(segment_number: i32, workflow_mode: &str) -> Value {
        json!({
            "schema_version": "video_segment_execution_recipe_v1",
            "segment_number": segment_number,
            "workflow_mode": workflow_mode,
            "style": "cinematic",
            "prompt_output_language": "zh_cn",
            "duration_secs": 6,
            "duration_secs_options": [6],
            "aspect_ratio": "16:9",
            "reference_image_ids": [],
            "video_library_image_ids": [],
            "guidance_mode": "none",
            "guidance_reference_image_ids": [],
            "continuity_source_video_id": null,
            "reference_media": null,
            "adopted_video_id": null
        })
    }

    #[test]
    fn generated_frame_image_prompt_ids_deduplicate_generation_context() {
        let images = vec![
            generated_image::Reference {
                id: 1,
                image_type: "video_key_frame".to_string(),
                metadata_json: Some(json!({
                    "generation_context": {"prompt_id": 9}
                })),
            },
            generated_image::Reference {
                id: 2,
                image_type: "video_key_frame".to_string(),
                metadata_json: Some(json!({
                    "generation_context": {"prompt_id": 9}
                })),
            },
            generated_image::Reference {
                id: 3,
                image_type: "video_key_frame".to_string(),
                metadata_json: Some(json!({
                    "generation_context": {"prompt_id": 0}
                })),
            },
        ];

        assert_eq!(
            generated_frame_image_prompt_ids(images),
            [9].into_iter().collect::<BTreeSet<_>>()
        );
    }

    fn bind_identity(metadata: &mut Value) {
        let segments = a4n_video_contract::video_segment_plan_from_metadata(metadata).unwrap();
        metadata["video_execution_identity"] = json!({
            "contract_epoch": a4n_video_contract::VIDEO_EXECUTION_CONTRACT_EPOCH,
            "segment_identities": a4n_video_contract::video_segment_identities(&segments).unwrap(),
        });
    }

    fn draft(mut metadata_json: Value) -> ChapterVideoPromptSyncDraft {
        bind_identity(&mut metadata_json);
        ChapterVideoPromptSyncDraft {
            fields: ChapterVideoPromptRecordFields {
                project_id: 1,
                chapter_id: 2,
                style: Some("cinematic".to_string()),
                prompt_type: "frame_segment_plan".to_string(),
                workflow_mode: "shared_segment_plan".to_string(),
                shot_number: 0,
                title: Some("片段规划".to_string()),
                scene_summary: None,
                visual_prompt: String::new(),
                motion_prompt: None,
                negative_prompt: None,
                duration_secs: 6,
                aspect_ratio: Some("16:9".to_string()),
                reference_image_ids_json: Some(json!([])),
                metadata_json: Some(metadata_json),
            },
            generation_trace: None,
        }
    }

    #[test]
    fn valid_segment_plan_updates_preserve_downstream_records() {
        let now = chrono::Utc::now().naive_utc();
        let mut existing = chapter_video_prompt::Model {
            id: 3,
            project_id: 1,
            chapter_id: 2,
            style: Some("cinematic".to_string()),
            prompt_type: "frame_segment_plan".to_string(),
            workflow_mode: "shared_segment_plan".to_string(),
            shot_number: 0,
            title: Some("片段规划".to_string()),
            scene_summary: None,
            visual_prompt: String::new(),
            motion_prompt: None,
            negative_prompt: None,
            duration_secs: 6,
            aspect_ratio: Some("16:9".to_string()),
            reference_image_ids_json: Some(json!([])),
            status: "completed".to_string(),
            generated_video_id: None,
            timeline_item_id: None,
            metadata_json: Some(json!({
                "contract_version": "v1",
                "source": {"content_sha256": "same"},
                "segments": [segment(1, "shot-2", "existing")],
                "segment_recipes": [recipe(1, "chat_video")],
                "plan_complete": false
            })),
            created_at: now,
            updated_at: now,
        };
        bind_identity(existing.metadata_json.as_mut().unwrap());
        let appended = draft(json!({
            "contract_version": "v1",
            "source": {"content_sha256": "same"},
            "segments": [
                segment(1, "shot-1", "inserted"),
                segment(2, "shot-2", "existing")
            ],
            "segment_recipes": [
                recipe(1, "chat_video"),
                recipe(2, "chat_video")
            ],
            "plan_complete": true
        }));
        assert!(segment_plan_update_preserves_outputs(&existing, &appended).unwrap());

        let mut changed = appended.clone();
        changed.fields.metadata_json.as_mut().unwrap()["segments"][1]["title"] = json!("changed");
        assert!(segment_plan_update_preserves_outputs(&existing, &changed).is_err());
        bind_identity(changed.fields.metadata_json.as_mut().unwrap());
        assert!(segment_plan_update_preserves_outputs(&existing, &changed).unwrap());

        let mut changed_recipe = appended;
        changed_recipe.fields.metadata_json.as_mut().unwrap()["segment_recipes"][1]["workflow_mode"] =
            json!("reference_media");
        assert!(segment_plan_update_preserves_outputs(&existing, &changed_recipe).unwrap());
        changed_recipe.fields.metadata_json.as_mut().unwrap()["segment_recipes"][1]["duration_secs"] =
            json!(0);
        assert!(segment_plan_update_preserves_outputs(&existing, &changed_recipe).is_err());
    }
}
