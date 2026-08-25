use a4n_core::video_workflow::WORKFLOW_MODE_MIXED;
use sea_orm::ActiveModelTrait;
use serde_json::json;
use std::collections::{HashMap, HashSet};

use super::*;
use crate::video::test_video_orchestrator;

fn assembly_video(id: i64, shot_number: i32, file_path: Option<&str>) -> generated_video::Model {
    let now = chrono::Utc::now().naive_utc();
    generated_video::Model {
        id,
        project_id: 1,
        chapter_id: Some(2),
        video_type: generated_video::VIDEO_TYPE_CHAPTER.to_string(),
        workflow_mode: Some("first_frame_chain".to_string()),
        file_path: file_path.map(str::to_string),
        thumbnail_path: None,
        prompt: format!("片段 {shot_number}"),
        negative_prompt: None,
        model: "test".to_string(),
        width: 1280,
        height: 720,
        duration_secs: 6.0,
        fps: 24,
        file_size: Some(1),
        source_video_id: None,
        source_image_id: None,
        status: "completed".to_string(),
        provider_job_id: None,
        error_message: None,
        timeline_item_id: None,
        metadata_json: Some(json!({ "shot_number": shot_number })),
        is_favorite: false,
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn generated_video_projection_preserves_wire_fields() {
    let mut video = assembly_video(9, 4, Some("projects/1/videos/video-9.mp4"));
    video.thumbnail_path = Some("projects/1/videos/video-9.jpg".to_string());
    video.negative_prompt = Some("avoid blur".to_string());
    video.file_size = Some(4096);
    video.source_video_id = Some(7);
    video.source_image_id = Some(8);
    video.provider_job_id = Some("provider-job-9".to_string());
    video.error_message = Some("previous error".to_string());
    video.timeline_item_id = Some(10);
    video.is_favorite = true;
    video.created_at = chrono::NaiveDate::from_ymd_opt(2026, 8, 20)
        .unwrap()
        .and_hms_opt(10, 11, 12)
        .unwrap();
    video.updated_at = chrono::NaiveDate::from_ymd_opt(2026, 8, 21)
        .unwrap()
        .and_hms_opt(13, 14, 15)
        .unwrap();

    assert_eq!(
        serde_json::to_value(generated_video_summary(&video)).unwrap(),
        json!({
            "id": 9,
            "project_id": 1,
            "chapter_id": 2,
            "video_type": "chapter_video",
            "workflow_mode": "first_frame_chain",
            "file_path": "projects/1/videos/video-9.mp4",
            "thumbnail_path": "projects/1/videos/video-9.jpg",
            "prompt": "片段 4",
            "negative_prompt": "avoid blur",
            "model": "test",
            "width": 1280,
            "height": 720,
            "duration_secs": 6.0,
            "fps": 24,
            "file_size": 4096,
            "source_video_id": 7,
            "source_image_id": 8,
            "status": "completed",
            "provider_job_id": "provider-job-9",
            "error_message": "previous error",
            "metadata_json": {"shot_number": 4},
            "timeline_item_id": 10,
            "is_favorite": true,
            "created_at": "2026-08-20 10:11:12",
            "updated_at": "2026-08-21 13:14:15",
        })
    );
}

#[test]
fn assembly_uses_available_videos_without_requiring_every_segment() {
    let rows = [
        assembly_video(3, 3, Some("three.mp4")),
        assembly_video(2, 2, None),
        assembly_video(1, 1, Some("one.mp4")),
    ];

    let clips = assembly_clip_records_from_rows(&rows, 1, 2, "first_frame_chain", None, None, None)
        .expect("已有有效视频应可直接组装");

    assert_eq!(
        clips.iter().map(|clip| clip.video_id).collect::<Vec<_>>(),
        [1, 3]
    );
}

#[test]
fn mixed_assembly_combines_timeline_videos_with_external_assembly_insert() {
    let mut native = assembly_video(1, 1, Some("native.mp4"));
    native.workflow_mode = Some(WORKFLOW_MODE_MIXED.to_string());
    native.metadata_json = Some(json!({
        "video_execution_identity": {
            "source_shot_id": "shot-a",
            "segment_index_in_shot": 1,
        },
    }));

    let mut inserted = assembly_video(2, 2, Some("timeline-insert.mp4"));
    inserted.workflow_mode = Some(WORKFLOW_MODE_MIXED.to_string());
    inserted.timeline_item_id = Some(102);
    inserted.metadata_json = Some(json!({}));

    let mut external_insert = assembly_video(3, 1, Some("external-insert.mp4"));
    external_insert.video_type = generated_video::VIDEO_TYPE_ASSEMBLY_INSERT.to_string();
    external_insert.workflow_mode = Some(WORKFLOW_MODE_MIXED.to_string());
    external_insert.metadata_json = Some(json!({
        "assembly_insert": {
            "sort_order": 1500.0,
            "insert_after_video_id": 1,
            "insert_after_shot_number": 1,
        },
    }));

    let timeline_order = super::super::timeline::VideoTimelineAssemblyOrder {
        display_index_by_item_id: HashMap::from([(101, 1), (102, 2)]),
        native_item_id_by_identity: HashMap::from([(("shot-a".to_string(), 1), 101)]),
    };
    let adopted_video_ids = HashSet::from([1, 2]);

    let clips = assembly_clip_records_from_rows(
        &[inserted, external_insert, native],
        1,
        2,
        WORKFLOW_MODE_MIXED,
        None,
        Some(&adopted_video_ids),
        Some(&timeline_order),
    )
    .expect("时间线视频与外部 assembly_insert 应能共同组装");

    assert_eq!(
        clips
            .iter()
            .map(|clip| (clip.video_id, clip.shot_number, clip.sort_order))
            .collect::<Vec<_>>(),
        [(1, 1, 1000.0), (3, 1, 1500.0), (2, 2, 2000.0)]
    );
}

#[test]
fn mixed_assembly_does_not_fall_back_when_timeline_exists_but_is_empty() {
    let mut native = assembly_video(1, 1, Some("native.mp4"));
    native.workflow_mode = Some(WORKFLOW_MODE_MIXED.to_string());
    let adopted_video_ids = HashSet::from([1]);

    let error = assembly_clip_records_from_rows(
        &[native],
        1,
        2,
        WORKFLOW_MODE_MIXED,
        None,
        Some(&adopted_video_ids),
        Some(&super::super::timeline::VideoTimelineAssemblyOrder::default()),
    )
    .unwrap_err();

    assert!(error.to_string().contains("不属于当前章节视频时间线"));
}

#[tokio::test]
async fn mixed_assembly_initializes_timeline_without_opening_the_video_page() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let root = crate::video::insert_video_project(&db, "混合成片时间线初始化").await;
    let chapter = crate::video::insert_video_chapter_without_shooting_script(&db, root.id, 1).await;
    let project = crate::video::insert_derived_video_project(
        std::sync::Arc::new(db.clone()),
        root.id,
        "混合成片派生视频",
    )
    .await;
    let source_hash = a4n_core::source_spans::source_text_hash(&chapter.content);
    let segment = serde_json::from_value::<a4n_core::video_execution::VideoSegmentPlanV2>(json!({
        "schema_version": "video_segment_plan_v2",
        "segment_number": 1,
        "source_shot_id": "shot-1",
        "segment_index_in_shot": 1,
        "shot_segment_count": 1,
        "covered_beat_ids": ["shot-1-beat-1"],
        "title": "测试片段",
        "scene_summary": "测试场景",
        "blocking_summary": "人物保持在画面中央",
        "spatial_anchors": [],
        "emotion_action_goals": [],
        "video_intent": {"hook":"人物入画","audience_effect":"建立人物","rhythm":"平稳","visual_priority":"人物"},
        "emotional_arc": {"start":"平静","turn":"观察","end":"确认","function":"建立场景"},
        "performance_plan": {"movement_path":"原地","body_performance":"克制","gaze_and_hands":"看向镜头外","pause_or_reaction":"短暂停顿","camera_relationship":"中景","time_blocks":[],"action_script":[]},
        "material_coverage": {"required_entities":[],"missing_or_weak_references":[],"risk_note":"无"},
        "transition_to_next": null,
        "source_span_ids": ["span-1"],
        "duration_secs": 6,
        "aspect_ratio": "16:9",
        "reference_image_ids": []
    }))
    .unwrap();
    let recipe = a4n_core::video_execution::VideoSegmentExecutionRecipeV1 {
        schema_version: a4n_core::video_execution::VIDEO_SEGMENT_EXECUTION_RECIPE_V1.to_string(),
        segment_number: 1,
        workflow_mode: a4n_core::video_execution::VideoSegmentWorkflowModeV1::ChatVideo,
        style: "cinematic".to_string(),
        prompt_output_language: "zh_cn".to_string(),
        duration_secs: 6,
        duration_secs_options: vec![6],
        aspect_ratio: "16:9".to_string(),
        reference_image_ids: Vec::new(),
        video_library_image_ids: Vec::new(),
        guidance_mode: a4n_core::video_execution::VideoSegmentGuidanceModeV1::None,
        guidance_reference_image_ids: Vec::new(),
        continuity_source_video_id: None,
        reference_media: None,
        adopted_video_id: None,
    };
    let now = chrono::Utc::now().naive_utc();
    a4n_core::models::chapter::chapter_video_prompt::ActiveModel {
        project_id: sea_orm::Set(project.id),
        chapter_id: sea_orm::Set(chapter.id),
        style: sea_orm::Set(Some("cinematic".to_string())),
        prompt_type: sea_orm::Set("frame_segment_plan".to_string()),
        workflow_mode: sea_orm::Set("shared_segment_plan".to_string()),
        shot_number: sea_orm::Set(0),
        title: sea_orm::Set(Some("共享片段规划".to_string())),
        scene_summary: sea_orm::Set(None),
        visual_prompt: sea_orm::Set("{}".to_string()),
        motion_prompt: sea_orm::Set(None),
        negative_prompt: sea_orm::Set(None),
        duration_secs: sea_orm::Set(6),
        aspect_ratio: sea_orm::Set(Some("16:9".to_string())),
        reference_image_ids_json: sea_orm::Set(None),
        status: sea_orm::Set("completed".to_string()),
        generated_video_id: sea_orm::Set(None),
        timeline_item_id: sea_orm::Set(None),
        metadata_json: sea_orm::Set(Some(json!({
            "video_execution_identity": {"source_content_sha256": source_hash},
            "segments": [segment],
            "segment_recipes": [recipe]
        }))),
        created_at: sea_orm::Set(now),
        updated_at: sea_orm::Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    assert!(
        a4n_db::video::chapter_video_timeline_by_scope(
            &db,
            project.id,
            chapter.id,
            a4n_core::models::chapter::chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN,
        )
        .await
        .unwrap()
        .is_none()
    );

    let service = test_video_orchestrator(std::sync::Arc::new(db.clone())).await;
    let records = service
        .assembly_video_records_validated(
            project.id,
            chapter.id,
            WORKFLOW_MODE_MIXED,
            "测试混合成片",
        )
        .await
        .unwrap();

    assert!(records.clips.is_empty());
    let timeline = a4n_db::video::chapter_video_timeline_by_scope(
        &db,
        project.id,
        chapter.id,
        a4n_core::models::chapter::chapter_video_prompt::WORKFLOW_MODE_SHARED_SEGMENT_PLAN,
    )
    .await
    .unwrap()
    .expect("混合成片必须建立唯一时间线");
    assert_eq!(
        a4n_db::video::active_timeline_order(
            &a4n_db::video::chapter_video_timeline_snapshot(&db, timeline.id)
                .await
                .unwrap(),
        )
        .unwrap()
        .len(),
        1
    );
}

#[test]
fn assembly_push_status_transition_has_one_owner() {
    assert_eq!(
        next_assembly_push_status(AssemblyPushAction::Accept, ASSEMBLY_PUSH_STATUS_PENDING)
            .unwrap(),
        ASSEMBLY_PUSH_STATUS_ACCEPTED
    );
    assert_eq!(
        next_assembly_push_status(AssemblyPushAction::Undo, ASSEMBLY_PUSH_STATUS_ACCEPTED).unwrap(),
        ASSEMBLY_PUSH_STATUS_PENDING
    );
    assert!(
        next_assembly_push_status(AssemblyPushAction::Reject, ASSEMBLY_PUSH_STATUS_ACCEPTED)
            .is_err()
    );
}

#[test]
fn metadata_update_rejects_clearing_system_fields() {
    let err = a4n_db::video::validate_video_metadata_update_preserves_system_fields(
        Some(&json!({
            "config_id": 7,
            "provider_request_snapshot": { "request": "body" },
        })),
        None,
        true,
    )
    .expect_err("system metadata must not be cleared");

    assert!(err.to_string().contains("不能清空"));
    assert!(err.to_string().contains("config_id"));
    assert!(err.to_string().contains("provider_request_snapshot"));
}

#[test]
fn metadata_update_rejects_removing_or_changing_system_fields() {
    let existing = json!({
        "config_id": 7,
        "provider_request_snapshot": { "request": "body" },
        "assembly_insert": { "sort_order": 1000.0 },
    });

    let missing = a4n_db::video::validate_video_metadata_update_preserves_system_fields(
        Some(&existing),
        Some(&json!({
            "provider_request_snapshot": { "request": "body" },
            "assembly_insert": { "sort_order": 2000.0 },
        })),
        false,
    )
    .expect_err("missing config_id must be rejected");
    assert!(missing.to_string().contains("config_id"));

    let changed = a4n_db::video::validate_video_metadata_update_preserves_system_fields(
        Some(&existing),
        Some(&json!({
            "config_id": 8,
            "provider_request_snapshot": { "request": "body" },
            "assembly_insert": { "sort_order": 2000.0 },
        })),
        false,
    )
    .expect_err("changed config_id must be rejected");
    assert!(changed.to_string().contains("config_id"));
}

#[test]
fn metadata_update_allows_business_fields_when_system_fields_are_preserved() {
    a4n_db::video::validate_video_metadata_update_preserves_system_fields(
        Some(&json!({
            "config_id": 7,
            "provider_request_snapshot": { "request": "body" },
            "shot_number": 1,
            "assembly_insert": { "sort_order": 1000.0 },
        })),
        Some(&json!({
            "config_id": 7,
            "provider_request_snapshot": { "request": "body" },
            "shot_number": 2,
            "assembly_insert": { "sort_order": 2000.0 },
        })),
        false,
    )
    .expect("business metadata fields should remain editable");
}

#[test]
fn provider_response_metadata_merges_only_into_objects() {
    let merged = a4n_db::video::merge_video_provider_response_metadata(
        Some(json!({
            "config_id": 7,
            "provider_request_snapshot": {"schema_version": 1}
        })),
        Some(json!({"job_id": "provider-job"})),
    )
    .expect("object metadata can accept provider response")
    .expect("merged metadata should exist");

    assert_eq!(merged["config_id"], json!(7));
    assert_eq!(
        merged["provider_response"],
        json!({"job_id": "provider-job"})
    );

    let created = a4n_db::video::merge_video_provider_response_metadata(
        None,
        Some(json!({"job_id": "provider-job"})),
    )
    .expect("missing metadata can be initialized")
    .expect("created metadata should exist");
    assert_eq!(
        created,
        json!({"provider_response": {"job_id": "provider-job"}})
    );

    let unchanged = a4n_db::video::merge_video_provider_response_metadata(
        Some(json!({"config_id": 7})),
        Some(Value::Null),
    )
    .expect("null provider metadata should be treated as missing")
    .expect("existing metadata should be preserved");
    assert_eq!(unchanged, json!({"config_id": 7}));

    let empty = a4n_db::video::merge_video_provider_response_metadata(None, Some(Value::Null))
        .expect("null provider metadata should not create fake provider_response");
    assert_eq!(empty, None);

    let err = a4n_db::video::merge_video_provider_response_metadata(
        Some(json!("corrupted metadata")),
        Some(json!({"job_id": "provider-job"})),
    )
    .expect_err("non-object metadata must not be wrapped into previous");
    assert_eq!(
        err.to_string(),
        "视频 metadata 必须是对象，不能写入 provider_response"
    );
}
