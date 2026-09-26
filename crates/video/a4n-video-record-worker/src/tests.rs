use super::*;
use a4n_db_models::models::{
    ai::generated_video,
    chapter::{chapter, chapter_video_prompt},
    user::user,
};
use a4n_domain::media::CreateGeneratedVideoRecordDraftInput;
use a4n_video_contract::{
    VIDEO_EXECUTION_CONTRACT_EPOCH, VideoSegmentPlanV2, VideoShootingScriptIdentityV1,
    default_video_segment_recipes, video_segment_content_fingerprint,
};
use a4n_worker_contract::{WorkerLease, WorkerPrincipal, WorkerPrincipalKind};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};
use serde_json::{Value, json};

#[test]
fn required_project_scope_rejects_missing_and_mismatched_scope() {
    let mut request = selection_request(7, 9, 10, 11, 12);
    assert!(validate_required_project_scope(&request, 9).is_ok());

    request.project_id = None;
    assert!(validate_required_project_scope(&request, 9).is_err());

    request.project_id = Some(10);
    assert!(validate_required_project_scope(&request, 9).is_err());
}

#[tokio::test]
async fn select_segment_rejects_stale_or_corrupt_plan_without_adopting_video() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let owner = insert_user(&db).await;
    let project = a4n_db::project::create_project_record(
        &db,
        a4n_db::project::CreateProjectRecordInput {
            user_id: owner.id,
            title: "采用视频片段测试".to_string(),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .unwrap();
    let chapter = insert_chapter(&db, project.id).await;
    let source_content_sha256 = a4n_domain::source_text_hash(&chapter.content);
    let style_sha256 = a4n_domain::source_text_hash("cinematic");
    let segments = vec![segment(1), segment(2)];
    let recipes = default_video_segment_recipes(&segments, "cinematic", "zh_cn").unwrap();
    let recipe = recipes
        .iter()
        .find(|recipe| recipe.segment_number == 2)
        .unwrap()
        .clone();
    let segment_sha256 = video_segment_content_fingerprint(&segments[1]).unwrap();
    let first_segment_sha256 = video_segment_content_fingerprint(&segments[0]).unwrap();
    let recipe_fingerprint = recipe.generation_fingerprint().unwrap();
    let plan = insert_plan(
        &db,
        project.id,
        chapter.id,
        json!({
            "video_execution_identity": {
                "contract_epoch": VIDEO_EXECUTION_CONTRACT_EPOCH,
                "source_content_sha256": source_content_sha256,
                "shooting_script_identity": VideoShootingScriptIdentityV1 {
                    record_id: 1,
                    revision: 1,
                    source_revision_key: "legacy-source".into(),
                    source_content_sha256: source_content_sha256.clone(),
                    script_sha256: "a".repeat(64),
                    shot_fingerprints: Default::default(),
                },
                "style_sha256": style_sha256,
                "duration_secs": 6,
                "aspect_ratio": "16:9",
                "segment_identities": [
                    {
                        "source_shot_id": "stable-shot",
                        "segment_index_in_shot": 1,
                        "segment_sha256": first_segment_sha256
                    },
                    {
                        "source_shot_id": "stable-shot",
                        "segment_index_in_shot": 2,
                        "segment_sha256": segment_sha256
                    }
                ]
            },
            "segments": segments,
            "segment_recipes": recipes
        }),
    )
    .await;
    let video_identity = |segment_recipe_fingerprint: &str| {
        json!({
            "video_execution_identity": {
                "contract_epoch": VIDEO_EXECUTION_CONTRACT_EPOCH,
                "source_content_sha256": source_content_sha256,
                "style_sha256": style_sha256,
                "duration_secs": 6,
                "aspect_ratio": "16:9",
                "segment_plan_prompt_id": plan.id,
                "source_shot_id": "stable-shot",
                "segment_index_in_shot": 2,
                "segment_sha256": segment_sha256,
                "segment_recipe_fingerprint": segment_recipe_fingerprint
            }
        })
    };
    let matching_video = insert_completed_video(
        &db,
        project.id,
        chapter.id,
        video_identity(&recipe_fingerprint),
        "matching-lineage",
    )
    .await;
    let state = VideoRecordWorker::new(
        DbHandle::new(Arc::new(db.clone())),
        VIDEO_RECORD_WORKER_ID,
        "test-token",
    )
    .state;

    let failure = state
        .select_segment(selection_request(
            owner.id,
            project.id,
            chapter.id,
            plan.id,
            matching_video.id,
        ))
        .await
        .expect_err("legacy plans without proven shot sources must not accept videos");
    assert!(failure.message.contains("来源已更新"), "{failure:?}");
    let unchanged = chapter_video_prompt::Entity::find_by_id(plan.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        unchanged.metadata_json.as_ref().unwrap()["segment_recipes"][1]["adopted_video_id"],
        Value::Null
    );

    let mut metadata = unchanged.metadata_json.clone().unwrap();
    metadata["segments"][1]["scene_summary"] = json!("内容已变化但执行指纹未更新");
    let mut tampered: chapter_video_prompt::ActiveModel = unchanged.clone().into();
    tampered.metadata_json = Set(Some(metadata));
    tampered.update(&db).await.unwrap();
    let failure = state
        .select_segment(selection_request(
            owner.id,
            project.id,
            chapter.id,
            plan.id,
            matching_video.id,
        ))
        .await
        .expect_err("adoption must validate the stored plan content before accepting its video");
    assert!(failure.message.contains("内容指纹"));
    let mut restored: chapter_video_prompt::ActiveModel = unchanged.clone().into();
    restored.metadata_json = Set(unchanged.metadata_json);
    restored.update(&db).await.unwrap();

    let updated = chapter_video_prompt::Entity::find_by_id(plan.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        updated.metadata_json.unwrap()["segment_recipes"][1]["adopted_video_id"],
        Value::Null
    );
}

async fn insert_user(db: &sea_orm::DatabaseConnection) -> user::Model {
    let now = Utc::now().naive_utc();
    user::ActiveModel {
        username: Set("video-record-owner".to_string()),
        email: Set("video-record-owner@example.test".to_string()),
        password_hash: Set("test-password-hash".to_string()),
        role: Set("user".to_string()),
        oauth_provider: Set(None),
        oauth_subject: Set(None),
        avatar_url: Set(None),
        is_active: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        last_login_at: Set(None),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

async fn insert_chapter(db: &sea_orm::DatabaseConnection, project_id: i64) -> chapter::Model {
    let now = Utc::now().naive_utc();
    chapter::ActiveModel {
        project_id: Set(project_id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("章节正文".to_string()),
        word_count: Set(4),
        status: Set("draft".to_string()),
        summary: Set(None),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

async fn insert_plan(
    db: &sea_orm::DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
    metadata: Value,
) -> chapter_video_prompt::Model {
    let now = Utc::now().naive_utc();
    chapter_video_prompt::ActiveModel {
        project_id: Set(project_id),
        chapter_id: Set(chapter_id),
        style: Set(Some("cinematic".to_string())),
        prompt_type: Set("frame_segment_plan".to_string()),
        workflow_mode: Set("shared_segment_plan".to_string()),
        shot_number: Set(0),
        title: Set(Some("共享片段规划".to_string())),
        scene_summary: Set(None),
        visual_prompt: Set("{}".to_string()),
        motion_prompt: Set(None),
        negative_prompt: Set(None),
        duration_secs: Set(6),
        aspect_ratio: Set(Some("16:9".to_string())),
        reference_image_ids_json: Set(None),
        status: Set("completed".to_string()),
        generated_video_id: Set(None),
        metadata_json: Set(Some(metadata)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

async fn insert_completed_video(
    db: &sea_orm::DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
    metadata: Value,
    label: &str,
) -> generated_video::Model {
    let video = a4n_db::video::create_generated_video_record(
        db,
        CreateGeneratedVideoRecordDraftInput {
            project_id,
            chapter_id: Some(chapter_id),
            video_type: a4n_domain::video_workflow::VIDEO_TYPE_CHAPTER.to_string(),
            workflow_mode: Some("chat_video".to_string()),
            prompt: "测试视频".to_string(),
            negative_prompt: None,
            model: "test-model".to_string(),
            width: 1280,
            height: 720,
            duration_secs: 6.0,
            fps: 24,
            source_video_id: None,
            source_image_id: None,
            provider_job_id: None,
            metadata_json: Some(metadata),
        },
    )
    .await
    .unwrap();
    let mut active: generated_video::ActiveModel = video.into();
    active.status = Set(a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED.to_string());
    active.file_path = Set(Some(format!("projects/{project_id}/videos/{label}.mp4")));
    active.update(db).await.unwrap()
}

fn selection_request(
    user_id: i64,
    project_id: i64,
    chapter_id: i64,
    plan_id: i64,
    video_id: i64,
) -> WorkerEnvelope<VideoSegmentSelectionInput> {
    WorkerEnvelope {
        schema_version: WORKER_SCHEMA_VERSION,
        request_id: format!("select-segment-{video_id}"),
        trace_id: "select-segment-test".to_string(),
        principal: WorkerPrincipal {
            kind: WorkerPrincipalKind::User,
            subject: user_id.to_string(),
        },
        operation_id: VIDEO_RECORD_SELECT_SEGMENT_OPERATION.to_string(),
        idempotency_key: format!("select-segment-{video_id}"),
        project_id: Some(project_id),
        root_project_id: None,
        target_revision: None,
        runtime_config_snapshot_ref: None,
        lease: WorkerLease {
            worker_id: VIDEO_RECORD_WORKER_ID.to_string(),
            epoch: 1,
            fence_token: "test-fence".to_string(),
            expires_at: "9999-12-31T23:59:59Z".to_string(),
        },
        payload: VideoSegmentSelectionInput {
            project_id,
            chapter_id,
            video_id,
            input: a4n_video_record_worker_contract::SelectSegmentVideoInput {
                segment_plan_prompt_id: plan_id,
                target_segment: a4n_api_contract::video::VideoSegmentTarget {
                    source_shot_id: "stable-shot".to_string(),
                    segment_index_in_shot: 2,
                },
            },
        },
    }
}

fn segment(segment_number: i32) -> VideoSegmentPlanV2 {
    let index = segment_number;
    serde_json::from_value(json!({
        "schema_version": "video_segment_plan_v2",
        "segment_number": segment_number,
        "source_shot_id": "stable-shot",
        "segment_index_in_shot": index,
        "shot_segment_count": 2,
        "covered_beat_ids": [],
        "title": format!("片段 {segment_number}"),
        "scene_summary": "场景",
        "blocking_summary": "调度",
        "spatial_anchors": [],
        "emotion_action_goals": [],
        "video_intent": {"hook":"h","audience_effect":"a","rhythm":"r","visual_priority":"v"},
        "emotional_arc": {"start":"s","turn":"t","end":"e","function":"f"},
        "performance_plan": {"movement_path":"m","body_performance":"b","gaze_and_hands":"g","pause_or_reaction":"p","camera_relationship":"c","time_blocks":[],"action_script":[]},
        "material_coverage": {"required_entities":[],"missing_or_weak_references":[],"risk_note":"r"},
        "transition_to_next": null,
        "source_span_ids": ["span-1"],
        "duration_secs": 6,
        "aspect_ratio": "16:9",
        "reference_image_ids": []
    }))
    .unwrap()
}
