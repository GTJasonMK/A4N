use chrono::Utc;
use sea_orm::{ActiveModelTrait, EntityTrait, Set};

use a4n_core::models::{ai::generated_image, media::media_asset};

use super::support::{freeform_video_project, generated_image_with_asset, setup_db, text_project};

#[tokio::test]
async fn chapter_media_cleanup_rejects_cross_project_chapter() {
    use a4n_core::models::chapter::chapter;

    let db = setup_db().await;
    let project_a = text_project(&db, "章节清理项目 A").await;
    let project_b = text_project(&db, "章节清理项目 B").await;
    let now = Utc::now().naive_utc();
    let chapter_b = chapter::ActiveModel {
        project_id: Set(project_b.id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("B 第一章".to_string())),
        content: Set("B 正文".to_string()),
        word_count: Set(3),
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

    let error = crate::project::cleanup_chapter_media_data(&db, project_a.id, chapter_b.id)
        .await
        .expect_err("cross-project chapter cleanup must be rejected");
    assert!(matches!(
        error,
        crate::project::ProjectCleanupDbError::InvalidState { current, operation }
            if current == format!("chapter_project_id={}", project_b.id)
                && operation.contains(&format!("项目 {}", project_a.id))
    ));
    assert!(
        chapter::Entity::find_by_id(chapter_b.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );
}

#[tokio::test]
async fn chapter_media_cleanup_preserves_path_referenced_by_media_asset() {
    use a4n_core::models::chapter::chapter;

    let db = setup_db().await;
    let project = text_project(&db, "章节媒体路径保留测试").await;
    let asset_project = freeform_video_project(&db, "章节媒体路径跨项目引用").await;
    let (asset_source, asset) =
        generated_image_with_asset(&db, asset_project.id, "chapter-retained.png").await;
    crate::video::delete_generated_image_record(&db, asset_source.id)
        .await
        .unwrap();
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
    let image = crate::video::create_generated_image_record(
        &db,
        crate::video::CreateGeneratedImageRecordDraftInput {
            project_id: project.id,
            image_type: "chapter_image".to_string(),
            file_path: asset_source.file_path,
            prompt: "chapter retained image".to_string(),
            model: "test".to_string(),
            width: 512,
            height: 512,
            file_size: Some(1024),
            chapter_id: Some(chapter.id),
            panel_id: None,
            metadata_json: None,
            created_at: None,
        },
    )
    .await
    .unwrap();
    let paths = crate::project::cleanup_chapter_media_data(&db, project.id, chapter.id)
        .await
        .unwrap();

    assert!(paths.is_empty());
    assert!(
        generated_image::Entity::find_by_id(image.id)
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        media_asset::Entity::find_by_id(asset.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );
}
