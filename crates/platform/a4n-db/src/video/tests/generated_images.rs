use a4n_db_models::models::{
    ai::{generated_image, generated_video},
    chapter::{chapter, chapter_manga_prompt, chapter_video_prompt},
    project::novel_project,
};
use a4n_domain::image::{
    IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT, IMAGE_TYPE_SCENE_MATERIAL,
    IMAGE_TYPE_VIDEO_KEY_FRAME,
};
use a4n_domain::media::{
    CreateGeneratedImageRecordDraftInput, CreateGeneratedVideoRecordDraftInput,
    UpdateGeneratedImageRecordInput,
};
use chrono::{Duration, Utc};
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, PaginatorTrait, Set};
use serde_json::json;

use crate::video::{
    VideoDbError, create_generated_image_record, create_generated_video_record,
    delete_generated_image_record_with_cleanup, generated_image_ids_by_ids,
};

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

async fn text_project(db: &impl crate::DbExecutor, title: &str) -> novel_project::Model {
    crate::project::create_project_record(
        db,
        crate::project::CreateProjectRecordInput {
            user_id: 1,
            title: title.to_string(),
            description: "图片记录测试项目".to_string(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .unwrap()
}

async fn chapter_record(db: &impl crate::DbExecutor, project_id: i64) -> chapter::Model {
    let now = Utc::now().naive_utc();
    chapter::ActiveModel {
        project_id: Set(project_id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("图片章节".to_string())),
        content: Set("章节正文".to_string()),
        word_count: Set(4),
        status: Set(chapter::STATUS_DRAFT.to_string()),
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

fn image_input(project_id: i64) -> CreateGeneratedImageRecordDraftInput {
    CreateGeneratedImageRecordDraftInput {
        project_id,
        image_type: "scene".to_string(),
        file_path: "images/scene.png".to_string(),
        prompt: "draw scene".to_string(),
        model: "test-model".to_string(),
        width: 512,
        height: 512,
        file_size: None,
        chapter_id: None,
        panel_id: None,
        metadata_json: None,
        created_at: None,
    }
}

async fn image_count(db: &impl crate::DbExecutor) -> u64 {
    generated_image::Entity::find().count(db).await.unwrap()
}

#[tokio::test]
async fn generated_image_ids_by_ids_returns_existing_only() {
    let db = setup_db().await;
    let project = text_project(&db, "图片 ID 窄投影").await;
    let image = create_generated_image_record(&db, image_input(project.id))
        .await
        .unwrap();

    let mut ids = generated_image_ids_by_ids(&db, &[image.id, image.id + 1])
        .await
        .unwrap();
    ids.sort_unstable();
    assert_eq!(ids, vec![image.id]);
    assert!(
        generated_image_ids_by_ids(&db, &[])
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn generated_images_for_project_filters_requested_ids_and_project() {
    let db = setup_db().await;
    let project = text_project(&db, "按图片 ID 读取").await;
    let other_project = text_project(&db, "其他项目图片").await;
    let now = Utc::now().naive_utc();

    let mut older_input = image_input(project.id);
    older_input.file_path = "images/older.png".to_string();
    older_input.created_at = Some(now - Duration::minutes(1));
    let older = create_generated_image_record(&db, older_input)
        .await
        .unwrap();

    let mut newer_input = image_input(project.id);
    newer_input.file_path = "images/newer.png".to_string();
    newer_input.created_at = Some(now);
    let newer = create_generated_image_record(&db, newer_input)
        .await
        .unwrap();
    let other = create_generated_image_record(&db, image_input(other_project.id))
        .await
        .unwrap();

    let selected = crate::video::read_models::generated_images_for_project(
        &db,
        project.id,
        Some(vec![older.id, other.id, newer.id, older.id + 1_000_000]),
    )
    .await
    .unwrap();

    assert_eq!(
        selected.iter().map(|image| image.id).collect::<Vec<_>>(),
        vec![newer.id, older.id]
    );
    assert!(
        crate::video::read_models::generated_images_for_project(&db, project.id, Some(Vec::new()))
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn video_profile_material_read_model_returns_domain_image_records() {
    let db = setup_db().await;
    let project = text_project(&db, "视频素材记录投影").await;
    let mut input = image_input(project.id);
    input.image_type = IMAGE_TYPE_SCENE_MATERIAL.to_string();
    let image = crate::video::read_models::create_generated_image_record(&db, input)
        .await
        .unwrap();

    let read_model = crate::video::video_profile_material_read_model(&db, &[project.id])
        .await
        .unwrap();

    assert_eq!(read_model.material_images, vec![image]);
}

async fn manga_prompt(
    db: &impl crate::DbExecutor,
    project_id: i64,
    chapter_id: i64,
    page_number: i32,
    generated_image_id: Option<i64>,
) -> chapter_manga_prompt::Model {
    let now = Utc::now().naive_utc();
    chapter_manga_prompt::ActiveModel {
        chapter_id: Set(chapter_id),
        project_id: Set(project_id),
        page_number: Set(page_number),
        prompt_type: Set("page_generation".to_string()),
        image_prompt: Set("draw page".to_string()),
        dialogue_text: Set(None),
        characters_in_frame: Set(None),
        layout: Set(None),
        generated_image_id: Set(generated_image_id),
        status: Set("completed".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

async fn frame_prompt(
    db: &impl crate::DbExecutor,
    project_id: i64,
    chapter_id: i64,
    shot_number: i32,
    reference_image_ids: Vec<i64>,
) -> chapter_video_prompt::Model {
    let now = Utc::now().naive_utc();
    chapter_video_prompt::ActiveModel {
        project_id: Set(project_id),
        chapter_id: Set(chapter_id),
        style: Set(Some("测试风格".to_string())),
        prompt_type: Set("frame_prompt".to_string()),
        workflow_mode: Set("boundary_frames".to_string()),
        shot_number: Set(shot_number),
        title: Set(Some(format!("边界帧 {shot_number}"))),
        scene_summary: Set(Some("边界帧摘要".to_string())),
        visual_prompt: Set("边界帧提示词".to_string()),
        motion_prompt: Set(None),
        negative_prompt: Set(Some("低质量".to_string())),
        duration_secs: Set(5),
        aspect_ratio: Set(Some("16:9".to_string())),
        reference_image_ids_json: Set(Some(json!(reference_image_ids.clone()))),
        status: Set("pending".to_string()),
        generated_video_id: Set(None),
        metadata_json: Set(Some(json!({
            "prompt_output_language": "zh_cn",
            "boundary_frame": {
                "frame_number": shot_number,
                "reference_image_ids": reference_image_ids,
            }
        }))),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

#[tokio::test]
async fn delete_image_clears_direct_prompt_and_video_bindings_in_batch() {
    let db = setup_db().await;
    let project = text_project(&db, "图片直接引用批量清理").await;
    let chapter = chapter_record(&db, project.id).await;
    let deleted = create_generated_image_record(
        &db,
        CreateGeneratedImageRecordDraftInput {
            file_path: format!("projects/{}/images/delete-batch.png", project.id),
            ..image_input(project.id)
        },
    )
    .await
    .unwrap();
    let retained = create_generated_image_record(
        &db,
        CreateGeneratedImageRecordDraftInput {
            file_path: format!("projects/{}/images/retain-batch.png", project.id),
            ..image_input(project.id)
        },
    )
    .await
    .unwrap();
    let deleted_prompt = manga_prompt(&db, project.id, chapter.id, 1, Some(deleted.id)).await;
    let retained_prompt = manga_prompt(&db, project.id, chapter.id, 2, Some(retained.id)).await;
    let video = create_generated_video_record(
        &db,
        CreateGeneratedVideoRecordDraftInput {
            project_id: project.id,
            chapter_id: Some(chapter.id),
            video_type: a4n_domain::video_workflow::VIDEO_TYPE_CHAPTER.to_string(),
            workflow_mode: Some("boundary_frames".to_string()),
            prompt: "video reference".to_string(),
            negative_prompt: None,
            model: "test-model".to_string(),
            width: 1280,
            height: 720,
            duration_secs: 5.0,
            fps: 24,
            source_video_id: None,
            source_image_id: Some(deleted.id),
            provider_job_id: None,
            metadata_json: None,
        },
    )
    .await
    .unwrap();

    delete_generated_image_record_with_cleanup(&db, deleted.id)
        .await
        .unwrap();

    let deleted_prompt = chapter_manga_prompt::Entity::find_by_id(deleted_prompt.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(deleted_prompt.generated_image_id, None);
    assert_eq!(deleted_prompt.status, "pending");
    let retained_prompt = chapter_manga_prompt::Entity::find_by_id(retained_prompt.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(retained_prompt.generated_image_id, Some(retained.id));
    assert_eq!(
        generated_video::Entity::find_by_id(video.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap()
            .source_image_id,
        None
    );
}

#[tokio::test]
async fn create_rejects_non_positive_dimensions_without_insert() {
    let db = setup_db().await;
    let project = text_project(&db, "图片尺寸校验").await;

    let error = create_generated_image_record(
        &db,
        CreateGeneratedImageRecordDraftInput {
            width: 0,
            ..image_input(project.id)
        },
    )
    .await
    .expect_err("zero-width image must be rejected");

    assert!(matches!(
        error,
        VideoDbError::InvalidArgument(message) if message == "width 必须大于 0"
    ));
    assert_eq!(image_count(&db).await, 0);
}

#[tokio::test]
async fn create_normalizes_required_text_and_blank_panel_id() {
    let db = setup_db().await;
    let project = text_project(&db, "图片字段规范化").await;
    let chapter = chapter_record(&db, project.id).await;

    let image = create_generated_image_record(
        &db,
        CreateGeneratedImageRecordDraftInput {
            image_type: " scene ".to_string(),
            file_path: " images/scene.png ".to_string(),
            prompt: " draw scene ".to_string(),
            model: " test-model ".to_string(),
            chapter_id: Some(chapter.id),
            panel_id: Some("   ".to_string()),
            metadata_json: Some(json!({"source": "test"})),
            ..image_input(project.id)
        },
    )
    .await
    .unwrap();

    assert_eq!(image.image_type, "scene");
    assert_eq!(image.file_path, "images/scene.png");
    assert_eq!(image.prompt, "draw scene");
    assert_eq!(image.model, "test-model");
    assert_eq!(image.panel_id, None);
}

#[tokio::test]
async fn create_boundary_frame_image_does_not_mutate_next_frame_references() {
    let db = setup_db().await;
    let project = text_project(&db, "边界帧引用保护").await;
    let chapter = chapter_record(&db, project.id).await;
    let first = frame_prompt(&db, project.id, chapter.id, 1, vec![3, 9]).await;
    let next = frame_prompt(&db, project.id, chapter.id, 2, vec![3, 9, 999_999]).await;

    create_generated_image_record(
        &db,
        CreateGeneratedImageRecordDraftInput {
            project_id: project.id,
            image_type: IMAGE_TYPE_VIDEO_KEY_FRAME.to_string(),
            file_path: "images/frame-1.png".to_string(),
            prompt: "边界帧提示词".to_string(),
            model: "image-model".to_string(),
            width: 1280,
            height: 720,
            file_size: Some(1024),
            chapter_id: Some(chapter.id),
            panel_id: None,
            metadata_json: Some(json!({
                "generation_context": {
                    "source": IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT,
                    "prompt_id": first.id,
                }
            })),
            created_at: None,
        },
    )
    .await
    .unwrap();

    let stored = chapter_video_prompt::Entity::find_by_id(next.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        stored.reference_image_ids_json,
        Some(json!([3, 9, 999_999]))
    );
    assert_eq!(
        stored
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("boundary_frame"))
            .and_then(|frame| frame.get("reference_image_ids")),
        Some(&json!([3, 9, 999_999]))
    );
}

#[tokio::test]
async fn create_rejects_missing_project_without_insert() {
    let db = setup_db().await;

    let error = create_generated_image_record(&db, image_input(404))
        .await
        .expect_err("missing project must not create orphan image");

    assert!(matches!(
        error,
        VideoDbError::NotFound(message) if message == "项目不存在: 404"
    ));
    assert_eq!(image_count(&db).await, 0);
}

#[tokio::test]
async fn create_rejects_chapter_from_another_project_without_insert() {
    let db = setup_db().await;
    let first_project = text_project(&db, "图片项目一").await;
    let second_project = text_project(&db, "图片项目二").await;
    let other_chapter = chapter_record(&db, second_project.id).await;

    let error = create_generated_image_record(
        &db,
        CreateGeneratedImageRecordDraftInput {
            chapter_id: Some(other_chapter.id),
            ..image_input(first_project.id)
        },
    )
    .await
    .expect_err("cross-project chapter binding must be rejected");

    assert!(matches!(
        error,
        VideoDbError::InvalidArgument(message)
            if message == format!("章节 #{} 不属于文本项目 {}", other_chapter.id, first_project.id)
    ));
    assert_eq!(image_count(&db).await, 0);
}

#[tokio::test]
async fn update_rejects_non_finite_ai_score_without_update() {
    let db = setup_db().await;
    let project = text_project(&db, "图片评分拒绝").await;
    let image = create_generated_image_record(&db, image_input(project.id))
        .await
        .unwrap();

    let error = crate::video::read_models::update_generated_image_record(
        &db,
        image.id,
        UpdateGeneratedImageRecordInput {
            ai_score: Some(f32::NAN),
            ..Default::default()
        },
    )
    .await
    .expect_err("NaN score must be rejected");

    assert!(matches!(
        error,
        VideoDbError::InvalidArgument(message) if message == "ai_score 必须是有限数"
    ));
    let stored = generated_image::Entity::find_by_id(image.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.ai_score, None);
}

#[tokio::test]
async fn update_persists_finite_ai_score() {
    let db = setup_db().await;
    let project = text_project(&db, "图片评分更新").await;
    let image = create_generated_image_record(&db, image_input(project.id))
        .await
        .unwrap();

    let updated = crate::video::read_models::update_generated_image_record(
        &db,
        image.id,
        UpdateGeneratedImageRecordInput {
            ai_score: Some(0.8),
            ..Default::default()
        },
    )
    .await
    .unwrap();

    assert_eq!(updated.ai_score, Some(0.8));
    let stored = generated_image::Entity::find_by_id(image.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.ai_score, Some(0.8));
}

#[tokio::test]
async fn delete_missing_image_keeps_orphan_manga_reference_unchanged() {
    let db = setup_db().await;
    let project = text_project(&db, "缺失图片删除").await;
    let chapter = chapter_record(&db, project.id).await;
    let prompt = manga_prompt(&db, project.id, chapter.id, 1, Some(999_999)).await;

    let error = delete_generated_image_record_with_cleanup(&db, 999_999)
        .await
        .expect_err("missing image deletion must be visible");

    assert!(matches!(
        error,
        VideoDbError::NotFound(message) if message == "图片不存在"
    ));
    let stored = chapter_manga_prompt::Entity::find_by_id(prompt.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.generated_image_id, Some(999_999));
    assert_eq!(stored.status, "completed");
}

#[tokio::test]
async fn delete_existing_image_clears_manga_prompt_reference() {
    let db = setup_db().await;
    let project = text_project(&db, "漫画图片删除").await;
    let chapter = chapter_record(&db, project.id).await;
    let image = create_generated_image_record(
        &db,
        CreateGeneratedImageRecordDraftInput {
            image_type: "manga_page".to_string(),
            file_path: format!("projects/{}/images/page1.png", project.id),
            prompt: "draw page".to_string(),
            chapter_id: Some(chapter.id),
            panel_id: Some("page1".to_string()),
            ..image_input(project.id)
        },
    )
    .await
    .unwrap();
    let prompt = manga_prompt(&db, project.id, chapter.id, 1, Some(image.id)).await;

    delete_generated_image_record_with_cleanup(&db, image.id)
        .await
        .unwrap();

    assert_eq!(image_count(&db).await, 0);
    let stored = chapter_manga_prompt::Entity::find_by_id(prompt.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.generated_image_id, None);
    assert_eq!(stored.status, "pending");
}
