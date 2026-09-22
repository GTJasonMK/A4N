use super::identity::should_keep_unmatched_prompt;
use super::*;
use a4n_db_models::models::{ai::generated_image, chapter::chapter_video_prompt};
#[cfg(test)]
use a4n_domain::video_prompt::ChapterVideoPromptRecordFields;
use a4n_domain::video_prompt::ChapterVideoPromptSyncDraft;
use a4n_domain::video_prompt::ChapterVideoPromptSyncPolicy;
use a4n_video_contract::{VideoSegmentExecutionRecipeV1, VideoSegmentPlanV2};
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

pub(super) fn segment_plan_extension_preserves_existing(
    existing: &chapter_video_prompt::Model,
    draft: &ChapterVideoPromptSyncDraft,
) -> bool {
    if existing.prompt_type != chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN
        || draft.fields.prompt_type != chapter_video_prompt::PROMPT_TYPE_FRAME_SEGMENT_PLAN
        || existing.workflow_mode != chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN
        || draft.fields.workflow_mode != chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN
        || existing.style != draft.fields.style
        || existing.duration_secs != draft.fields.duration_secs
        || existing.aspect_ratio != draft.fields.aspect_ratio
    {
        return false;
    }
    let Some(existing_metadata) = existing.metadata_json.as_ref() else {
        return false;
    };
    let Some(draft_metadata) = draft.fields.metadata_json.as_ref() else {
        return false;
    };
    if existing_metadata.get("source") != draft_metadata.get("source")
        || existing_metadata.get("contract_version") != draft_metadata.get("contract_version")
    {
        return false;
    }
    let Some(existing_items) = segment_plan_items(existing_metadata) else {
        return false;
    };
    let Some(draft_items) = segment_plan_items(draft_metadata) else {
        return false;
    };
    existing_items.len() <= draft_items.len()
        && existing_items.iter().all(|(key, (segment, recipe))| {
            draft_items.get(key).is_some_and(|draft| {
                plan_items_match_ignoring_display_number((segment, recipe), draft)
            })
        })
}

type StableSegmentKey = (String, i32);
type SegmentPlanItem = (VideoSegmentPlanV2, VideoSegmentExecutionRecipeV1);

fn segment_plan_items(metadata: &Value) -> Option<HashMap<StableSegmentKey, SegmentPlanItem>> {
    let segments =
        serde_json::from_value::<Vec<VideoSegmentPlanV2>>(metadata.get("segments")?.clone())
            .ok()?;
    let recipes = serde_json::from_value::<Vec<VideoSegmentExecutionRecipeV1>>(
        metadata.get("segment_recipes")?.clone(),
    )
    .ok()?;
    if segments.is_empty() || segments.len() != recipes.len() {
        return None;
    }
    let mut recipes_by_number = HashMap::with_capacity(recipes.len());
    for recipe in recipes {
        recipe.validate().ok()?;
        if recipes_by_number
            .insert(recipe.segment_number, recipe)
            .is_some()
        {
            return None;
        }
    }
    let mut items = HashMap::with_capacity(segments.len());
    for segment in segments {
        if segment.segment_number <= 0
            || segment.source_shot_id.trim().is_empty()
            || segment.segment_index_in_shot <= 0
        {
            return None;
        }
        let recipe = recipes_by_number.remove(&segment.segment_number)?;
        if items
            .insert(
                (
                    segment.source_shot_id.clone(),
                    segment.segment_index_in_shot,
                ),
                (segment, recipe),
            )
            .is_some()
        {
            return None;
        }
    }
    recipes_by_number.is_empty().then_some(items)
}

fn plan_items_match_ignoring_display_number(
    left: (&VideoSegmentPlanV2, &VideoSegmentExecutionRecipeV1),
    right: &SegmentPlanItem,
) -> bool {
    let (mut segment, mut recipe) = (left.0.clone(), left.1.clone());
    segment.segment_number = right.0.segment_number;
    recipe.segment_number = right.1.segment_number;
    segment == right.0 && recipe == right.1
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

    fn draft(metadata_json: Value) -> ChapterVideoPromptSyncDraft {
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
    fn segment_plan_extension_uses_stable_segment_identity() {
        let now = chrono::Utc::now().naive_utc();
        let existing = chapter_video_prompt::Model {
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
        assert!(segment_plan_extension_preserves_existing(
            &existing, &appended
        ));

        let mut changed = appended.clone();
        changed.fields.metadata_json.as_mut().unwrap()["segments"][1]["title"] = json!("changed");
        assert!(!segment_plan_extension_preserves_existing(
            &existing, &changed
        ));

        let mut changed_recipe = appended;
        changed_recipe.fields.metadata_json.as_mut().unwrap()["segment_recipes"][1]["workflow_mode"] =
            json!("reference_media");
        assert!(!segment_plan_extension_preserves_existing(
            &existing,
            &changed_recipe
        ));
    }
}
