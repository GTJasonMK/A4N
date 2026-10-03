//! 章节视频时间线应用规则。

use a4n_api_contract::video::{
    CommitInsertedSegmentDraftInput, DeleteInsertedSegmentInput, GetVideoTimelineInput,
    MoveInsertedSegmentInput, VideoTimelineItemKind as ApiItemKind,
    VideoTimelineItemStatus as ApiItemStatus, VideoTimelineItemSummary, VideoTimelineSummary,
};
use a4n_domain::media::GeneratedVideoRecord;
use a4n_domain::video_prompt as chapter_video_prompt;
use a4n_domain::video_timeline::{
    NativeVideoTimelineKey, VideoTimelineGap, VideoTimelineItemRecord, timeline_gap_index,
};
use a4n_domain::{prompt_language::PromptOutputLanguage, source_text_hash};
use a4n_error::AppError;
use a4n_video_contract::{
    VIDEO_EXECUTION_CONTRACT_EPOCH, VIDEO_SEGMENT_EXECUTION_RECIPE_V1,
    VideoSegmentExecutionRecipeV1, VideoSegmentGuidanceModeV1, VideoSegmentPlanV2,
    VideoSegmentWorkflowModeV1, video_segment_recipes_for_plan_from_metadata,
};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::workflow::WORKFLOW_STATUS_COMPLETED;
use a4n_db::video::ChapterVideoPromptTimelineRecord;
use std::collections::HashMap;

use crate::video::prompts::json::InsertedSegmentDraft;
use crate::{OrchestratorContext, structured_llm::ensure_checkpoint_parsed_output_matches};
use a4n_domain::collaboration::ProjectAccessRole;

pub(crate) const INSERTED_SEGMENT_SOURCE_PROMPT_TYPE: &str = "inserted_segment";
pub(crate) const INSERTED_SEGMENT_WORKFLOW_KIND: &str = "video_inserted_segment";
pub(crate) const INSERTED_SEGMENT_CHECKPOINT_KEY: &str = "inserted_segment_draft";
pub(crate) const VIDEO_TIMELINE_SCOPE_TABLE: &str = "chapter_video_timeline";

pub(crate) struct InsertedSegmentNeighborContext {
    pub(crate) previous: Value,
    pub(crate) next: Value,
    pub(crate) sha256: String,
}

include!("assembly.rs");
include!("inserted.rs");

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_domain::video_timeline::VideoTimelineRecord;
    use chrono::Utc;

    fn native_item(id: i64, source_shot_id: &str, position: i32) -> VideoTimelineItemRecord {
        let now = Utc::now().naive_utc();
        VideoTimelineItemRecord {
            id,
            timeline_id: 1,
            position: Some(position),
            item_kind: a4n_db::video::TIMELINE_NATIVE_KIND.to_string(),
            status: a4n_db::video::TIMELINE_ACTIVE_STATUS.to_string(),
            native_source_shot_id: Some(source_shot_id.to_string()),
            native_segment_index_in_shot: Some(1),
            insert_plan_prompt_id: None,
            workflow_run_id: None,
            created_at: now,
            updated_at: now,
        }
    }

    fn inserted_item(id: i64) -> VideoTimelineItemRecord {
        let now = Utc::now().naive_utc();
        VideoTimelineItemRecord {
            id,
            timeline_id: 1,
            position: Some(1),
            item_kind: a4n_db::video::TIMELINE_INSERTED_KIND.to_string(),
            status: a4n_db::video::TIMELINE_ACTIVE_STATUS.to_string(),
            native_source_shot_id: None,
            native_segment_index_in_shot: None,
            insert_plan_prompt_id: Some(99),
            workflow_run_id: Some(1),
            created_at: now,
            updated_at: now,
        }
    }

    fn completed_inserted_video(id: i64, timeline_item_id: i64) -> GeneratedVideoRecord {
        let now = Utc::now().naive_utc();
        GeneratedVideoRecord {
            id,
            project_id: 1,
            chapter_id: Some(1),
            video_type: a4n_domain::video_workflow::VIDEO_TYPE_CHAPTER.to_string(),
            workflow_mode: Some("boundary_frames".to_string()),
            file_path: Some(format!("{id}.mp4")),
            thumbnail_path: None,
            prompt: "prompt".to_string(),
            negative_prompt: None,
            model: "test".to_string(),
            width: 1920,
            height: 1080,
            duration_secs: 6.0,
            fps: 24,
            file_size: None,
            source_video_id: None,
            source_image_id: None,
            status: "completed".to_string(),
            provider_job_id: None,
            error_message: None,
            timeline_item_id: Some(timeline_item_id),
            metadata_json: None,
            is_favorite: false,
            created_at: now,
            updated_at: now,
        }
    }

    fn snapshot() -> a4n_db::video::VideoTimelineSnapshot {
        let now = Utc::now().naive_utc();
        a4n_db::video::VideoTimelineSnapshot {
            timeline: VideoTimelineRecord {
                id: 1,
                project_id: 1,
                chapter_id: 1,
                workflow_mode: chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN.to_string(),
                revision: 1,
                created_at: now,
                updated_at: now,
            },
            items: vec![native_item(1, "shot-a", 1), native_item(2, "shot-b", 2)],
        }
    }

    fn inserted_fingerprint(
        guidance_mode: VideoSegmentGuidanceModeV1,
    ) -> InsertedSegmentWorkflowFingerprint {
        InsertedSegmentWorkflowFingerprint {
            schema_version: 1,
            workflow_kind: INSERTED_SEGMENT_WORKFLOW_KIND.to_string(),
            project_id: 1,
            chapter_id: 1,
            source_content_sha256: "source".to_string(),
            context_sha256: "context".to_string(),
            timeline_id: 1,
            timeline_revision: 1,
            gap: VideoTimelineGap {
                previous_item_id: Some(1),
                next_item_id: Some(2),
            },
            payload: InsertedSegmentPayload {
                user_prompt: "反应镜头".to_string(),
                style: "电影感".to_string(),
                prompt_output_language: "zh_cn".to_string(),
                constraints: InsertedSegmentConstraints {
                    selected_duration_secs: 6,
                    duration_secs_options: vec![6],
                    aspect_ratio: "16:9".to_string(),
                    guidance_mode,
                    continuity_source_video_id: Some(9),
                    reference_image_ids: vec![21],
                },
            },
        }
    }

    #[test]
    fn inserted_recipe_routes_the_single_reference_source_by_guidance_mode() {
        let keyframes = inserted_segment_recipe(
            &inserted_fingerprint(VideoSegmentGuidanceModeV1::Keyframes),
            2,
            PromptOutputLanguage::ZhCn,
        )
        .unwrap();
        assert!(keyframes.reference_image_ids.is_empty());
        assert_eq!(keyframes.guidance_reference_image_ids, vec![21]);
        assert_eq!(keyframes.continuity_source_video_id, None);

        let continuity = inserted_segment_recipe(
            &inserted_fingerprint(VideoSegmentGuidanceModeV1::Continuity),
            2,
            PromptOutputLanguage::ZhCn,
        )
        .unwrap();
        assert_eq!(continuity.reference_image_ids, vec![21]);
        assert!(continuity.guidance_reference_image_ids.is_empty());
        assert_eq!(continuity.continuity_source_video_id, Some(9));
    }

    #[test]
    fn inserted_recipe_rejects_system_frame_guidance() {
        assert!(
            inserted_segment_recipe(
                &inserted_fingerprint(VideoSegmentGuidanceModeV1::SystemFrames),
                2,
                PromptOutputLanguage::ZhCn,
            )
            .is_err()
        );
    }

    #[test]
    fn native_reconcile_detects_order_changes_even_when_identity_fields_match() {
        let matching = vec![
            NativeVideoTimelineKey::new("shot-a", 1).unwrap(),
            NativeVideoTimelineKey::new("shot-b", 1).unwrap(),
        ];
        let reordered = vec![matching[1].clone(), matching[0].clone()];

        assert!(!native_timeline_needs_reconcile(&snapshot(), &matching).unwrap());
        assert!(native_timeline_needs_reconcile(&snapshot(), &reordered).unwrap());
    }

    #[test]
    fn assembly_order_excludes_items_that_need_reanchoring() {
        let mut snapshot = snapshot();
        snapshot.items[1].position = Some(3);
        let now = Utc::now().naive_utc();
        snapshot.items.push(VideoTimelineItemRecord {
            id: 9,
            timeline_id: 1,
            position: Some(2),
            item_kind: a4n_db::video::TIMELINE_INSERTED_KIND.to_string(),
            status: a4n_db::video::TIMELINE_NEEDS_REANCHOR_STATUS.to_string(),
            native_source_shot_id: None,
            native_segment_index_in_shot: None,
            insert_plan_prompt_id: Some(99),
            workflow_run_id: Some(9),
            created_at: now,
            updated_at: now,
        });

        let order = video_timeline_assembly_order_from_snapshot(&snapshot).unwrap();

        assert_eq!(order.display_index_by_item_id.get(&1), Some(&1));
        assert!(!order.display_index_by_item_id.contains_key(&9));
        assert_eq!(order.display_index_by_item_id.get(&2), Some(&3));
    }

    #[test]
    fn assembly_order_resolves_inserted_and_native_video_identity() {
        let mut snapshot = snapshot();
        snapshot.items[1].position = Some(3);
        let mut item = inserted_item(9);
        item.position = Some(2);
        snapshot.items.insert(1, item);
        let order = video_timeline_assembly_order_from_snapshot(&snapshot).unwrap();
        let inserted = completed_inserted_video(3, 9);
        let mut native = completed_inserted_video(4, 99);
        native.timeline_item_id = None;
        native.metadata_json = Some(json!({
            "video_execution_identity": {
                "source_shot_id": "shot-b",
                "segment_index_in_shot": 1,
            },
        }));

        assert_eq!(order.display_index_for_video(&inserted), Some(2));
        assert_eq!(order.display_index_for_video(&native), Some(3));
    }

    #[test]
    fn inserted_current_video_uses_the_timeline_item_foreign_key() {
        let item = inserted_item(8);
        let videos = [
            completed_inserted_video(3, 8),
            completed_inserted_video(2, 8),
        ];
        assert_eq!(
            current_timeline_item_video_id(&item, Some(2), &videos).unwrap(),
            Some(2)
        );
        assert_eq!(
            current_timeline_item_video_id(&item, None, &videos).unwrap(),
            None
        );

        let mut invalid = completed_inserted_video(4, 8);
        invalid.timeline_item_id = Some(9);
        assert!(current_timeline_item_video_id(&item, Some(4), &[invalid]).is_err());
    }

    #[test]
    fn native_current_video_uses_stable_source_identity() {
        let item = native_item(8, "shot-a", 1);
        let mut video = completed_inserted_video(3, 99);
        video.metadata_json = Some(json!({
            "video_execution_identity": {
                "source_shot_id": "shot-a",
                "segment_index_in_shot": 1,
            },
        }));
        assert!(current_timeline_item_video_id(&item, Some(3), &[video.clone()]).is_err());
        video.timeline_item_id = None;

        assert_eq!(
            current_timeline_item_video_id(&item, Some(3), &[video]).unwrap(),
            Some(3)
        );
    }

    #[test]
    fn reanchor_item_cannot_be_used_as_a_new_insert_anchor() {
        let mut item = inserted_item(8);
        item.status = a4n_db::video::TIMELINE_NEEDS_REANCHOR_STATUS.to_string();

        assert!(timeline_item_context(Some(8), &[item], &[], &[], &[], &[], &[]).is_err());
    }
}
