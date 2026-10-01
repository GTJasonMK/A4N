use super::{
    ProjectResetDbError, reset_project_generated_data_record, rollback_project_status_record,
};
use a4n_db_models::models::{
    ai::{generated_image, generated_video},
    chapter::chapter,
    project::novel_project,
    scene::scene_occurrence,
    task::cleanup_outbox,
};
use a4n_domain::profile_ownership::PROFILE_OWNERSHIP_AI_SYNC;
use a4n_domain::{project::ProjectStatus, scene_profile::SceneProfileRecordInput};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait,
    PaginatorTrait, QueryFilter, Set,
};
use serde_json::json;
use std::error::Error;

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

async fn text_project(db: &DatabaseConnection, title: &str) -> novel_project::Model {
    crate::project::create_project_record(
        db,
        crate::project::CreateProjectRecordInput {
            user_id: 1,
            title: title.to_string(),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .unwrap()
}

async fn chapter_record(db: &DatabaseConnection, project_id: i64) -> chapter::Model {
    let now = Utc::now().naive_utc();
    chapter::ActiveModel {
        project_id: Set(project_id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("正文".to_string()),
        word_count: Set(2),
        status: Set(chapter::STATUS_COMPLETED.to_string()),
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

async fn generated_image_record(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: Option<i64>,
    file_path: &str,
) -> generated_image::Model {
    crate::video::create_generated_image_record(
        db,
        a4n_domain::media::CreateGeneratedImageRecordDraftInput {
            project_id,
            image_type: "chapter_image".to_string(),
            file_path: file_path.to_string(),
            prompt: "reset outcome test image".to_string(),
            model: "test".to_string(),
            width: 512,
            height: 512,
            file_size: Some(1024),
            chapter_id,
            panel_id: None,
            metadata_json: None,
            created_at: None,
        },
    )
    .await
    .unwrap()
}

async fn completed_generated_video_record(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
    file_path: &str,
    thumbnail_path: &str,
) -> generated_video::Model {
    let video = crate::video::create_generated_video_record(
        db,
        a4n_domain::media::CreateGeneratedVideoRecordDraftInput {
            project_id,
            chapter_id: Some(chapter_id),
            video_type: a4n_domain::video_workflow::VIDEO_TYPE_CHAPTER.to_string(),
            workflow_mode: Some("storyboard".to_string()),
            prompt: "reset outcome test video".to_string(),
            negative_prompt: None,
            model: "test".to_string(),
            width: 1280,
            height: 720,
            duration_secs: 5.0,
            fps: 24,
            source_video_id: None,
            source_image_id: None,
            provider_job_id: None,
            metadata_json: None,
        },
    )
    .await
    .unwrap();
    crate::video::mark_generated_video_completed_and_link_prompt_record(
        db,
        video.id,
        file_path,
        Some(thumbnail_path),
        &a4n_domain::media::VideoFileProperties {
            file_size: 2048,
            width: video.width,
            height: video.height,
            duration_secs: video.duration_secs,
            fps: video.fps,
        },
        None,
        None,
    )
    .await
    .unwrap()
}

#[test]
fn project_not_found_error_keeps_project_id() {
    let error = ProjectResetDbError::ProjectNotFound { project_id: 42 };

    assert_eq!(error.to_string(), "项目不存在: 42");
    assert!(error.source().is_none());
}

#[test]
fn database_error_keeps_sea_orm_source() {
    let error = ProjectResetDbError::Database {
        source: DbErr::Custom("query failed".to_string()),
    };

    assert_eq!(error.to_string(), "Custom Error: query failed");
    assert!(error.source().is_some());
}

#[test]
fn rollback_status_parse_error_is_invalid_argument() {
    let error = ProjectStatus::parse_storage("unknown")
        .map_err(|error| ProjectResetDbError::InvalidArgument(error.to_string()))
        .expect_err("unknown status should fail");

    assert!(matches!(error, ProjectResetDbError::InvalidArgument(_)));
}

#[tokio::test]
async fn reset_returns_only_paths_unreferenced_after_transaction_cleanup() {
    let db = setup_db().await;
    let project = text_project(&db, "生成态重置路径测试").await;
    let other_project = text_project(&db, "生成态重置共享路径项目").await;
    let shared_path = "projects/shared/reset-image.png";
    let unique_path = format!("projects/{}/images/reset-unique.png", project.id);

    generated_image_record(&db, project.id, None, shared_path).await;
    generated_image_record(&db, project.id, None, &unique_path).await;
    let other_image = generated_image_record(&db, other_project.id, None, shared_path).await;

    let mut gated_project: novel_project::ActiveModel = project.clone().into();
    gated_project.lifecycle_gate = Set(crate::generation::PROJECT_GATE_RESETTING.to_string());
    gated_project.update(&db).await.unwrap();
    let outcome = reset_project_generated_data_record(&db, project.id)
        .await
        .unwrap();

    assert_eq!(outcome.media_paths, vec![unique_path]);
    assert_eq!(
        generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.eq(project.id))
            .count(&db)
            .await
            .unwrap(),
        0
    );
    assert!(
        generated_image::Entity::find_by_id(other_image.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn reset_deletes_scene_occurrences_before_chapters() {
    let db = setup_db().await;
    for preserve_source in [false, true] {
        let project = text_project(&db, "场景出场重置顺序测试").await;
        let chapter = chapter_record(&db, project.id).await;
        let scene_profile = crate::scene_profile::create_scene_profile_record(
            &db,
            project.id,
            SceneProfileRecordInput {
                name: "旧车站".to_string(),
                normalized_name: "旧车站".to_string(),
                aliases_json: json!([]),
                scene_type: "location".to_string(),
                importance: "primary".to_string(),
                parent_scene_id: None,
                base_description: "废弃站台".to_string(),
                visual_description: "雨幕中的钟楼".to_string(),
                spatial_layout: "站厅连接钟楼".to_string(),
                atmosphere: "潮湿而紧张".to_string(),
                entry_points_json: json!([]),
                rules_json: json!([]),
                first_seen_chapter: 1,
                last_seen_chapter: 1,
                evidence_json: json!([]),
                image_prompt_seed: "雨夜旧车站".to_string(),
            },
            PROFILE_OWNERSHIP_AI_SYNC,
        )
        .await
        .unwrap();
        let now = Utc::now().naive_utc();
        scene_occurrence::ActiveModel {
            project_id: Set(project.id),
            scene_profile_id: Set(scene_profile.id),
            chapter_id: Set(chapter.id),
            chapter_number: Set(1),
            occurrence_order: Set(1),
            timeline_position: Set(None),
            summary: Set("林岚进入旧车站".to_string()),
            mood: Set(None),
            weather_or_lighting: Set(None),
            source_span_ids_json: Set(json!([])),
            evidence_json: Set(json!([])),
            characters_json: Set(json!([])),
            objects_json: Set(json!([])),
            key_events_json: Set(json!([])),
            entry_state_json: Set(json!({})),
            exit_state_json: Set(json!({})),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();

        let image = generated_image_record(
            &db,
            project.id,
            Some(chapter.id),
            &format!("projects/{}/images/frozen.png", project.id),
        )
        .await;
        if preserve_source {
            use crate::chapter_visual_plan::SaveChapterVisualPlanDraftInput;
            use a4n_domain::chapter_visual_plan::{
                ChapterVisualPlanAuthoringMode, ChapterVisualPlanV3, ConfirmChapterVisualPlanInput,
                SceneBindingV3,
            };
            let video = crate::project::create_derived_project_record(
                &db,
                1,
                crate::project::CreateDerivedProjectParams {
                    root_project_id: project.id,
                    derived_kind: "video".into(),
                    title: "保留视频导演资源".into(),
                    description: String::new(),
                    chapter_scope: crate::project::DerivedChapterScope {
                        start_chapter: 1,
                        end_chapter: 1,
                    },
                },
            )
            .await
            .unwrap();
            let mut plan = ChapterVisualPlanV3::starter();
            plan.points[0].image_id = Some(image.id);
            plan.segments[0].scene_binding = Some(SceneBindingV3::Scene {
                scene_profile_id: scene_profile.id,
            });
            plan.segments[0].prompt = "林岚进入旧车站".into();
            let saved = crate::chapter_visual_plan::save_chapter_visual_plan_draft(
                &db,
                SaveChapterVisualPlanDraftInput {
                    source: crate::project::resolve_chapter_source(&db, video.id, chapter.id)
                        .await
                        .unwrap(),
                    author_user_id: 1,
                    authoring_mode: ChapterVisualPlanAuthoringMode::Manual,
                    expected: None,
                    expected_source_hash: None,
                    plan,
                },
            )
            .await
            .unwrap();
            crate::chapter_visual_plan::confirm_chapter_visual_plan(
                &db,
                ConfirmChapterVisualPlanInput {
                    record_id: saved.record.id,
                    expected_revision: saved.record.revision,
                },
            )
            .await
            .unwrap();
        }
        let project_id = project.id;
        let mut gated_project: novel_project::ActiveModel = project.into();
        gated_project.lifecycle_gate = Set(crate::generation::PROJECT_GATE_RESETTING.to_string());
        gated_project.update(&db).await.unwrap();

        let outcome = reset_project_generated_data_record(&db, project_id)
            .await
            .unwrap();

        assert_eq!(outcome.result.deleted_scene_occurrences, 1);
        assert_eq!(
            outcome.result.deleted_scene_profiles,
            u64::from(!preserve_source)
        );
        assert_eq!(
            outcome.result.deleted_generated_images,
            u64::from(!preserve_source)
        );
        let retained_profile =
            a4n_db_models::models::scene::scene_profile::Entity::find_by_id(scene_profile.id)
                .one(&db)
                .await
                .unwrap();
        let retained_image = generated_image::Entity::find_by_id(image.id)
            .one(&db)
            .await
            .unwrap();
        assert_eq!(retained_profile.is_some(), preserve_source);
        assert_eq!(retained_image.is_some(), preserve_source);
        if preserve_source {
            let profile = retained_profile.unwrap();
            assert_eq!(profile.name, "旧车站");
            assert_eq!(profile.first_seen_chapter, 0);
            assert_eq!(profile.last_seen_chapter, 0);
            assert_eq!(profile.current_state_json, json!({}));
            assert_eq!(retained_image.unwrap().chapter_id, None);
            assert!(!outcome.media_paths.contains(&image.file_path));
        }
        assert_eq!(
            scene_occurrence::Entity::find()
                .filter(scene_occurrence::Column::ProjectId.eq(project_id))
                .count(&db)
                .await
                .unwrap(),
            0
        );
        assert_eq!(
            chapter::Entity::find_current()
                .filter(chapter::Column::ProjectId.eq(project_id))
                .count(&db)
                .await
                .unwrap(),
            0
        );
        let retained_chapter = chapter::Entity::find_by_id(chapter.id)
            .one(&db)
            .await
            .unwrap();
        assert!(retained_chapter.is_some());
        if let Some(retained) = retained_chapter {
            assert_eq!(retained.status, chapter::STATUS_DELETED);
            assert!(retained.content.is_empty());
            assert_eq!(retained.outline_id, None);
            assert_eq!(retained.title, None);
            assert_eq!(retained.analysis_data, None);
            let replacement = chapter_record(&db, project_id).await;
            assert_ne!(replacement.id, retained.id);
            assert_eq!(
                crate::chapter_content::chapters_for_project(&db, project_id)
                    .await
                    .unwrap(),
                vec![crate::project::chapter_record(replacement)]
            );
        }
    }
}

#[tokio::test]
async fn rollback_schedules_only_paths_unreferenced_after_chapter_cleanup() {
    let db = setup_db().await;
    let project = text_project(&db, "状态回退路径测试").await;
    let other_project = text_project(&db, "状态回退共享路径项目").await;
    let mut active: novel_project::ActiveModel = project.into();
    active.status = Set(ProjectStatus::Writing.as_str().to_string());
    let project = active.update(&db).await.unwrap();
    let chapter = chapter_record(&db, project.id).await;
    let shared_path = "projects/shared/rollback-image.png";
    let video_path = format!("projects/{}/videos/rollback-unique.mp4", project.id);
    let thumbnail_path = format!("projects/{}/videos/rollback-unique.jpg", project.id);

    let target_image = generated_image_record(&db, project.id, Some(chapter.id), shared_path).await;
    let target_video =
        completed_generated_video_record(&db, project.id, chapter.id, &video_path, &thumbnail_path)
            .await;
    let other_image = generated_image_record(&db, other_project.id, None, shared_path).await;

    let outcome = rollback_project_status_record(
        &db,
        project.id,
        ProjectStatus::Writing,
        ProjectStatus::ChapterOutlinesReady,
        Utc::now().naive_utc(),
    )
    .await
    .unwrap();

    assert_eq!(
        outcome.project.status,
        ProjectStatus::ChapterOutlinesReady.as_str()
    );
    assert!(outcome.needs_rag_clear);
    let mut expected_media_paths = vec![video_path, thumbnail_path];
    expected_media_paths.sort();
    let cleanup = cleanup_outbox::Entity::find()
        .filter(cleanup_outbox::Column::DedupeKey.eq(format!(
            "project-status-rollback:{}:chapter_outlines_ready",
            project.id
        )))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    let payload: crate::cleanup_outbox::FileArtifactCleanupPayload =
        serde_json::from_value(cleanup.payload_json).unwrap();
    assert_eq!(payload.paths, expected_media_paths);
    assert_eq!(
        chapter::Entity::find_current()
            .filter(chapter::Column::ProjectId.eq(project.id))
            .count(&db)
            .await
            .unwrap(),
        0
    );
    assert!(
        generated_video::Entity::find_by_id(target_video.id)
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        generated_image::Entity::find_by_id(target_image.id)
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        generated_image::Entity::find_by_id(other_image.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn rollback_status_history_failure_rolls_back_cleanup_and_status() {
    let db = setup_db().await;
    let project = crate::project::create_project_record(
        &db,
        crate::project::CreateProjectRecordInput {
            user_id: 1,
            title: "状态回退事务测试".to_string(),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .unwrap();
    let mut active: novel_project::ActiveModel = project.clone().into();
    active.status = Set("writing".to_string());
    let project = active.update(&db).await.unwrap();
    let now = Utc::now().naive_utc();
    let chapter = chapter::ActiveModel {
        project_id: Set(project.id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("正文".to_string()),
        word_count: Set(2),
        status: Set(chapter::STATUS_COMPLETED.to_string()),
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
    let function_name = format!("a4n_test_block_status_history_{}", project.id);
    let trigger_name = format!("a4n_test_block_status_history_trigger_{}", project.id);
    db.execute_unprepared(&format!(
        "DROP TRIGGER IF EXISTS {trigger_name} ON project_status_history; DROP FUNCTION IF EXISTS {function_name}()"
    ))
    .await
    .unwrap();
    db.execute_unprepared(&format!(
        "CREATE FUNCTION {function_name}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'blocked status history'; END; $$"
    ))
    .await
    .unwrap();
    db.execute_unprepared(&format!(
        "CREATE TRIGGER {trigger_name} BEFORE INSERT ON project_status_history FOR EACH ROW WHEN (NEW.project_id = {}) EXECUTE FUNCTION {function_name}()",
        project.id
    ))
    .await
    .unwrap();

    let error = rollback_project_status_record(
        &db,
        project.id,
        ProjectStatus::Writing,
        ProjectStatus::ChapterOutlinesReady,
        now,
    )
    .await
    .expect_err("history failure must roll back cleanup and status update");
    assert!(error.to_string().contains("blocked status history"));

    db.execute_unprepared(&format!(
        "DROP TRIGGER {trigger_name} ON project_status_history; DROP FUNCTION {function_name}()"
    ))
    .await
    .unwrap();

    assert_eq!(
        novel_project::Entity::find_by_id(project.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap()
            .status,
        "writing"
    );
    assert!(
        chapter::Entity::find_by_id(chapter.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );
    assert_eq!(
        a4n_db_models::models::project::project_status_history::Entity::find()
            .filter(
                a4n_db_models::models::project::project_status_history::Column::ProjectId
                    .eq(project.id),
            )
            .count(&db)
            .await
            .unwrap(),
        0
    );
}
