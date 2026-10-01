//! 章节视频时间线持久化。
//!
//! 这里是唯一负责修改章节时间线顺序和 revision 的数据库边界。普通共享
//! 片段规划只通过 `reconcile_native_timeline` 同步 native 项；插入项不会
//! 写回共享 `segments` JSON。

use a4n_db_models::models::chapter::{
    chapter_video_prompt, chapter_video_timeline, chapter_video_timeline_item,
};
use a4n_domain::video_prompt::ChapterVideoPromptRecordFields;
use a4n_domain::video_timeline::{
    NativeVideoTimelineKey, VideoTimelineGap, move_inserted_timeline_item,
    remove_inserted_timeline_item, timeline_gap_index,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, EntityTrait, QueryFilter,
    QueryOrder, QuerySelect, Set, TransactionTrait,
    sea_query::{CaseStatement, Expr, OnConflict},
};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

use super::{VideoDbError, lock_generated_media_project};

pub const TIMELINE_NATIVE_KIND: &str = "native";
pub const TIMELINE_INSERTED_KIND: &str = "inserted";
pub const TIMELINE_ACTIVE_STATUS: &str = "active";
pub const TIMELINE_DELETED_STATUS: &str = "deleted";
pub const TIMELINE_NEEDS_REANCHOR_STATUS: &str = "needs_reanchor";

#[derive(Debug, Clone, PartialEq)]
pub struct VideoTimelineSnapshot {
    pub timeline: chapter_video_timeline::Model,
    pub items: Vec<chapter_video_timeline_item::Model>,
}

#[derive(Debug, Clone)]
pub struct CommitInsertedTimelineItemInput {
    pub timeline_id: i64,
    pub expected_revision: i64,
    pub gap: VideoTimelineGap,
    pub prompt: ChapterVideoPromptRecordFields,
    pub workflow_run_id: i64,
}

include!("operations.rs");
include!("helpers.rs");

#[cfg(test)]
mod tests {
    use super::*;
    fn timeline_item(
        id: i64,
        position: Option<i32>,
        status: &str,
    ) -> chapter_video_timeline_item::Model {
        let now = Utc::now().naive_utc();
        chapter_video_timeline_item::Model {
            id,
            timeline_id: 1,
            position,
            item_kind: TIMELINE_INSERTED_KIND.to_string(),
            status: status.to_string(),
            native_source_shot_id: None,
            native_segment_index_in_shot: None,
            insert_plan_prompt_id: Some(id + 100),
            workflow_run_id: Some(id + 200),
            created_at: now,
            updated_at: now,
        }
    }

    fn timeline(revision: i64) -> chapter_video_timeline::Model {
        let now = Utc::now().naive_utc();
        chapter_video_timeline::Model {
            id: 1,
            project_id: 1,
            chapter_id: 1,
            workflow_mode: "shared_segment_plan".to_string(),
            revision,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn native_key_rejects_empty_or_non_positive_identity() {
        assert!(NativeVideoTimelineKey::new("", 1).is_err());
        assert!(NativeVideoTimelineKey::new("shot", 0).is_err());
        assert_eq!(
            NativeVideoTimelineKey::new(" shot-1 ", 2)
                .unwrap()
                .source_shot_id,
            "shot-1"
        );
    }

    #[test]
    fn positioned_order_ignores_soft_deleted_items() {
        let items = [
            timeline_item(1, Some(1), TIMELINE_ACTIVE_STATUS),
            timeline_item(2, None, TIMELINE_DELETED_STATUS),
        ];
        assert_eq!(positioned_order(&items).unwrap(), vec![1]);
    }

    #[test]
    fn positioned_order_rejects_gaps_and_duplicates() {
        let items = [
            timeline_item(1, Some(1), TIMELINE_ACTIVE_STATUS),
            timeline_item(2, Some(3), TIMELINE_ACTIVE_STATUS),
        ];
        assert!(positioned_order(&items).is_err());

        let mut items = items;
        items[1].position = Some(1);
        assert!(positioned_order(&items).is_err());
    }

    #[test]
    fn native_reconcile_marks_collapsed_inserted_runs_for_reanchor() {
        let inserted_ids = [8, 9].into_iter().collect();
        let (rebuilt, needs_reanchor) =
            rebuild_native_timeline_order(&[1, 8, 3, 9, 2], vec![1, 2], &inserted_ids);

        assert_eq!(rebuilt, vec![1, 8, 9, 2]);
        assert_eq!(needs_reanchor, inserted_ids);
    }

    #[test]
    fn native_reconcile_keeps_unchanged_inserted_run_active() {
        let inserted_ids = [8, 9].into_iter().collect();
        let (rebuilt, needs_reanchor) =
            rebuild_native_timeline_order(&[1, 8, 9, 2], vec![1, 2], &inserted_ids);

        assert_eq!(rebuilt, vec![1, 8, 9, 2]);
        assert!(needs_reanchor.is_empty());
    }

    #[test]
    fn native_reconcile_keeps_leading_inserted_run_before_first_native() {
        let inserted_ids = [8, 9].into_iter().collect();
        let (rebuilt, needs_reanchor) =
            rebuild_native_timeline_order(&[8, 9, 1], vec![1], &inserted_ids);

        assert_eq!(rebuilt, vec![8, 9, 1]);
        assert!(needs_reanchor.is_empty());
    }

    #[test]
    fn native_reconcile_keeps_following_inserted_items_in_the_previous_gap() {
        let inserted_ids = [8, 9].into_iter().collect();
        let (rebuilt, needs_reanchor) =
            rebuild_native_timeline_order(&[1, 8, 9, 4], vec![1, 2, 3, 4], &inserted_ids);

        assert_eq!(rebuilt, vec![1, 8, 9, 2, 3, 4]);
        assert_eq!(needs_reanchor, inserted_ids);
    }

    #[test]
    fn native_reconcile_never_reverses_inserted_items_when_native_order_changes() {
        let inserted_ids = [8, 9].into_iter().collect();
        let (rebuilt, needs_reanchor) =
            rebuild_native_timeline_order(&[1, 8, 2, 9, 3], vec![3, 2, 1], &inserted_ids);

        assert_eq!(rebuilt, vec![3, 2, 1, 8, 9]);
        assert_eq!(needs_reanchor, inserted_ids);
    }

    #[test]
    fn native_reconcile_never_clears_an_existing_reanchor_requirement() {
        assert_eq!(
            inserted_status_after_native_reconcile(TIMELINE_NEEDS_REANCHOR_STATUS, false),
            TIMELINE_NEEDS_REANCHOR_STATUS
        );
        assert_eq!(
            inserted_status_after_native_reconcile(TIMELINE_ACTIVE_STATUS, false),
            TIMELINE_ACTIVE_STATUS
        );
    }

    #[test]
    fn inserted_commit_identity_is_found_before_revision_checks() {
        let mut item = timeline_item(8, Some(1), TIMELINE_ACTIVE_STATUS);
        item.workflow_run_id = Some(42);

        assert_eq!(
            inserted_timeline_item_by_workflow_run(&[item], 42).map(|item| item.id),
            Some(8)
        );
        assert!(inserted_timeline_item_by_workflow_run(&[], 0).is_none());
    }

    #[test]
    fn inserted_prompt_position_updates_the_recipe_without_duplicating_identity() {
        let mut metadata = json!({
            "segment_recipe": { "segment_number": 1 }
        });

        update_inserted_prompt_display_position(&mut metadata, 3).unwrap();

        assert_eq!(metadata["segment_recipe"]["segment_number"], 3);
        assert!(update_inserted_prompt_display_position(&mut json!({}), 3).is_err());
    }

    #[test]
    fn stale_timeline_revision_is_rejected_explicitly() {
        let error = ensure_revision(&timeline(3), 2).unwrap_err();
        assert!(
            error
                .to_string()
                .contains("timeline_revision_conflict:current=3")
        );
        assert!(error.to_string().contains("expected=2"));
    }

    #[tokio::test]
    async fn timeline_writes_reject_resetting_projects() {
        let db = a4n_migration::test_support::connect_test_database().await;
        let project = crate::project::create_project_record(
            &db,
            crate::project::CreateProjectRecordInput {
                user_id: 1,
                title: "视频时间线写入门禁".to_string(),
                description: String::new(),
                project_type: "novel_long".to_string(),
            },
        )
        .await
        .unwrap();
        let project_id = project.id;
        let now = Utc::now().naive_utc();
        let chapter = a4n_db_models::models::chapter::chapter::ActiveModel {
            project_id: Set(project.id),
            outline_id: Set(None),
            chapter_number: Set(1),
            title: Set(Some("第一章".to_string())),
            content: Set("正文".to_string()),
            word_count: Set(2),
            status: Set(a4n_db_models::models::chapter::chapter::STATUS_IMPORTED.to_string()),
            summary: Set(None),
            style_tags: Set(None),
            analysis_data: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        let timeline =
            ensure_chapter_video_timeline(&db, project.id, chapter.id, "shared_segment_plan")
                .await
                .unwrap();

        let mut active: a4n_db_models::models::project::novel_project::ActiveModel =
            crate::project::project_record_into_model(project.clone()).into();
        active.lifecycle_gate = Set(crate::generation::PROJECT_GATE_RESETTING.to_string());
        active.update(&db).await.unwrap();

        for error in [
            ensure_chapter_video_timeline(&db, project_id, chapter.id, "shared_segment_plan")
                .await
                .expect_err("重置期间不能创建时间线"),
            reconcile_native_timeline(&db, timeline.id, &[])
                .await
                .expect_err("重置期间不能同步时间线"),
        ] {
            assert!(matches!(
                error,
                VideoDbError::InvalidState { current, .. }
                    if current == crate::generation::PROJECT_GATE_RESETTING
            ));
        }
    }

    #[tokio::test]
    async fn completing_imported_inserted_video_copies_the_prompt_timeline_binding() {
        const SOURCE_CONTENT_SHA256: &str =
            "0000000000000000000000000000000000000000000000000000000000000000";
        const SEGMENT_SHA256: &str =
            "1111111111111111111111111111111111111111111111111111111111111111";
        let db = a4n_migration::test_support::connect_test_database().await;
        let project = crate::project::create_project_record(
            &db,
            crate::project::CreateProjectRecordInput {
                user_id: 1,
                title: "插入片段导入绑定".to_string(),
                description: String::new(),
                project_type: "novel_long".to_string(),
            },
        )
        .await
        .unwrap();
        let now = Utc::now().naive_utc();
        let chapter = a4n_db_models::models::chapter::chapter::ActiveModel {
            project_id: Set(project.id),
            outline_id: Set(None),
            chapter_number: Set(1),
            title: Set(Some("第一章".to_string())),
            content: Set("正文".to_string()),
            word_count: Set(2),
            status: Set(a4n_db_models::models::chapter::chapter::STATUS_IMPORTED.to_string()),
            summary: Set(None),
            style_tags: Set(None),
            analysis_data: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        let timeline =
            ensure_chapter_video_timeline(&db, project.id, chapter.id, "shared_segment_plan")
                .await
                .unwrap();
        commit_inserted_timeline_item(
            &db,
            CommitInsertedTimelineItemInput {
                timeline_id: timeline.id,
                expected_revision: timeline.revision,
                gap: VideoTimelineGap {
                    previous_item_id: None,
                    next_item_id: None,
                },
                prompt: ChapterVideoPromptRecordFields {
                    project_id: project.id,
                    chapter_id: chapter.id,
                    style: Some("电影感".to_string()),
                    prompt_type: "video_prompt".to_string(),
                    workflow_mode: "boundary_frames".to_string(),
                    shot_number: 1,
                    title: Some("补镜".to_string()),
                    scene_summary: Some("补充人物反应".to_string()),
                    visual_prompt: "visual".to_string(),
                    motion_prompt: Some("motion".to_string()),
                    negative_prompt: Some("negative".to_string()),
                    duration_secs: 6,
                    aspect_ratio: Some("16:9".to_string()),
                    reference_image_ids_json: Some(json!([])),
                    metadata_json: Some(json!({
                        "prompt_output_language": "zh_cn",
                        "source_prompt_type": "inserted_segment",
                        "segment_recipe": {},
                        "video_execution_identity": {
                            "contract_epoch": a4n_video_contract::VIDEO_EXECUTION_CONTRACT_EPOCH,
                            "source_content_sha256": SOURCE_CONTENT_SHA256,
                            "segment_sha256": SEGMENT_SHA256
                        }
                    })),
                },
                workflow_run_id: 1,
            },
        )
        .await
        .unwrap();
        let prompt = chapter_video_prompt::Entity::find()
            .filter(chapter_video_prompt::Column::ProjectId.eq(project.id))
            .one(&db)
            .await
            .unwrap()
            .unwrap();
        let video = crate::video::create_generated_video_record(
            &db,
            a4n_domain::media::CreateGeneratedVideoRecordDraftInput {
                project_id: project.id,
                chapter_id: Some(chapter.id),
                video_type: a4n_domain::video_workflow::VIDEO_TYPE_CHAPTER.to_string(),
                workflow_mode: Some("boundary_frames".to_string()),
                prompt: "external import".to_string(),
                negative_prompt: None,
                model: "external-import".to_string(),
                width: 1280,
                height: 720,
                duration_secs: 6.0,
                fps: 24,
                source_video_id: None,
                source_image_id: None,
                provider_job_id: None,
                metadata_json: Some(json!({
                    "source_prompt_type": "inserted_segment",
                    "video_execution_identity": {
                        "contract_epoch": a4n_video_contract::VIDEO_EXECUTION_CONTRACT_EPOCH,
                        "source_content_sha256": SOURCE_CONTENT_SHA256,
                        "segment_sha256": SEGMENT_SHA256
                    }
                })),
            },
        )
        .await
        .unwrap();

        let completed = crate::video::mark_generated_video_completed_and_link_prompt_record(
            &db,
            video.id,
            "projects/imported.mp4",
            None,
            &a4n_domain::media::VideoFileProperties {
                file_size: 1,
                width: video.width,
                height: video.height,
                duration_secs: video.duration_secs,
                fps: video.fps,
            },
            None,
            Some(prompt.id),
        )
        .await
        .unwrap();

        assert_eq!(completed.timeline_item_id, prompt.timeline_item_id);
    }
}
