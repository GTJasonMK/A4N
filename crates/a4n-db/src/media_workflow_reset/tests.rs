use a4n_db_models::models::{ai::generated_image, task::cleanup_outbox};
use a4n_domain::image::IMAGE_TYPE_CANVAS;
use sea_orm::{ColumnTrait, DbErr, EntityTrait, QueryFilter};

use super::rules::{
    validate_media_workflow_reset_project_id, validate_media_workflow_reset_project_kind,
};

#[test]
fn media_workflow_reset_requires_positive_project_id() {
    assert!(matches!(
        validate_media_workflow_reset_project_id(0),
        Err(DbErr::Custom(message)) if message == "project_id 必须是正整数"
    ));
    assert!(validate_media_workflow_reset_project_id(1).is_ok());
}

#[test]
fn media_workflow_reset_accepts_only_supported_project_kinds() {
    for project_kind in ["comic", "video", "galgame"] {
        assert!(validate_media_workflow_reset_project_kind(project_kind).is_ok());
    }
    assert!(matches!(
        validate_media_workflow_reset_project_kind("text"),
        Err(DbErr::Custom(message))
            if message == "v2 媒体工作流重置不支持项目类型: text"
    ));
}

#[tokio::test]
async fn media_workflow_reset_commits_file_cleanup_intent_with_record_deletion() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = crate::project::create_freeform_video_project_record(
        &db,
        1,
        crate::project::CreateFreeformVideoProjectParams {
            title: "媒体 reset cleanup 原子性".to_string(),
            description: String::new(),
        },
    )
    .await
    .unwrap();
    let path = format!("projects/{}/images/reset.png", project.id);
    let image = crate::video::create_generated_image_record(
        &db,
        a4n_domain::media::CreateGeneratedImageRecordDraftInput {
            project_id: project.id,
            image_type: IMAGE_TYPE_CANVAS.to_string(),
            file_path: path.clone(),
            prompt: "reset cleanup".to_string(),
            model: "test".to_string(),
            width: 512,
            height: 512,
            file_size: Some(1),
            chapter_id: None,
            panel_id: None,
            metadata_json: None,
            created_at: None,
        },
    )
    .await
    .unwrap();
    let reset = crate::cleanup_outbox::insert_media_workflow_v2_reset_intent(
        &db,
        format!("test-media-reset-{}", project.id),
        crate::cleanup_outbox::MediaWorkflowV2ResetPayload::new(project.id),
    )
    .await
    .unwrap();

    super::reset_media_workflows_to_v2(&db, project.id, reset.id)
        .await
        .unwrap();

    assert!(
        generated_image::Entity::find_by_id(image.id)
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    let file_cleanup = cleanup_outbox::Entity::find()
        .filter(
            cleanup_outbox::Column::DedupeKey
                .eq(format!("media-workflow-v2-reset-files:{}", reset.id)),
        )
        .one(&db)
        .await
        .unwrap()
        .expect("reset 删除媒体记录时必须原子留下文件 cleanup intent");
    assert_eq!(file_cleanup.project_id, Some(project.id));
    assert_eq!(file_cleanup.payload_json["paths"][0], path);

    super::reset_media_workflows_to_v2(&db, project.id, reset.id)
        .await
        .unwrap();
}
