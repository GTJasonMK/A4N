use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set, TransactionTrait,
};

use a4n_db_models::models::{
    ai::{generated_image, generated_video},
    media::{image_publication, media_asset, media_binding, media_provenance},
    project::novel_project,
    task::cleanup_outbox,
};
use a4n_domain::image::IMAGE_TYPE_CANVAS;

use super::super::{GeneratedImageDeleteScope, delete_chapter_generation_outputs};
use super::support::{
    canvas_project, generated_image_with_asset, generated_video_with_asset, insert_test_user,
    set_project_gate, setup_db, text_project,
};

#[tokio::test]
async fn project_generation_cleanup_removes_only_generated_target_bindings() {
    let db = setup_db().await;
    let project = canvas_project(&db, "项目生成 binding 清理测试").await;
    let (image, image_asset) =
        generated_image_with_asset(&db, project.id, "binding-image.png").await;
    let (video, video_asset) =
        generated_video_with_asset(&db, project.id, "binding-video.mp4").await;
    for asset_id in [image_asset.id, video_asset.id] {
        media_binding::ActiveModel {
            media_asset_id: Set(asset_id),
            target_kind: Set("video_segment".to_string()),
            target_id: Set(9000 + asset_id),
            role: Set("generated_result".to_string()),
            created_at: Set(Utc::now().naive_utc()),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
    }

    let tx = db.begin().await.unwrap();
    let cleanup = delete_chapter_generation_outputs(
        &tx,
        project.id,
        GeneratedImageDeleteScope::Project,
        &crate::project::delete::RetainedSourceResources::default(),
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    assert_eq!(cleanup.deleted_generated_images, 1);
    assert_eq!(cleanup.deleted_generated_videos, 1);
    assert!(
        generated_image::Entity::find_by_id(image.id)
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    assert!(
        generated_video::Entity::find_by_id(video.id)
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    for (target_kind, target_id) in [("generated_image", image.id), ("generated_video", video.id)] {
        assert_eq!(
            media_binding::Entity::find()
                .filter(media_binding::Column::TargetKind.eq(target_kind))
                .filter(media_binding::Column::TargetId.eq(target_id))
                .count(&db)
                .await
                .unwrap(),
            0
        );
    }
    for asset_id in [image_asset.id, video_asset.id] {
        assert!(
            media_asset::Entity::find_by_id(asset_id)
                .one(&db)
                .await
                .unwrap()
                .is_some()
        );
        assert!(
            media_provenance::Entity::find_by_id(asset_id)
                .one(&db)
                .await
                .unwrap()
                .is_some()
        );
        assert_eq!(
            media_binding::Entity::find()
                .filter(media_binding::Column::MediaAssetId.eq(asset_id))
                .filter(media_binding::Column::TargetKind.eq("video_segment"))
                .count(&db)
                .await
                .unwrap(),
            1
        );
    }
}

#[tokio::test]
async fn freeform_reset_rediscovers_targets_and_preserves_imported_media() {
    let db = setup_db().await;
    let project = canvas_project(&db, "自由画布重置目标复核测试").await;
    let (generated, generated_asset) =
        generated_image_with_asset(&db, project.id, "generated-reset.png").await;
    let imported = crate::video::create_generated_image_record(
        &db,
        a4n_domain::media::CreateGeneratedImageRecordDraftInput {
            project_id: project.id,
            image_type: IMAGE_TYPE_CANVAS.to_string(),
            file_path: generated.file_path.clone(),
            prompt: "imported canvas image".to_string(),
            model: "external-import".to_string(),
            width: 512,
            height: 512,
            file_size: Some(1024),
            chapter_id: None,
            panel_id: None,
            metadata_json: Some(serde_json::json!({"canvas_import": true})),
            created_at: None,
        },
    )
    .await
    .unwrap();
    let imported_asset = crate::canvas::ensure_canvas_image_media_asset_record(
        &db,
        crate::canvas::CanvasImageMediaAssetEnsureInput {
            project_id: project.id,
            image: crate::video::read_models::generated_image_by_id(&db, imported.id)
                .await
                .unwrap()
                .unwrap(),
            generation_spec_id: None,
            origin: "imported".to_string(),
        },
    )
    .await
    .unwrap();
    assert_eq!(generated_asset.id, imported_asset.id);

    set_project_gate(
        &db,
        &project,
        a4n_domain::generation::ProjectLifecycleOperation::Reset,
    )
    .await;
    let reset = crate::project::reset_freeform_video_generated_data_records(&db, project.id)
        .await
        .unwrap();

    assert_eq!(
        novel_project::Entity::find_by_id(project.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap()
            .lifecycle_gate,
        "resetting"
    );
    assert_eq!(reset.deleted_generated_images, 1);
    assert!(reset.media_paths.is_empty());
    assert!(
        generated_image::Entity::find_by_id(generated.id)
            .one(&db)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        media_binding::Entity::find()
            .filter(media_binding::Column::TargetKind.eq("generated_image"))
            .filter(media_binding::Column::TargetId.eq(generated.id))
            .count(&db)
            .await
            .unwrap(),
        0
    );
    assert!(
        generated_image::Entity::find_by_id(imported.id)
            .one(&db)
            .await
            .unwrap()
            .is_some()
    );
    let retained_asset = media_asset::Entity::find_by_id(imported_asset.id)
        .one(&db)
        .await
        .unwrap()
        .expect("imported media asset should be retained");
    assert_eq!(retained_asset.origin, "imported");
    assert_eq!(
        media_binding::Entity::find()
            .filter(media_binding::Column::TargetKind.eq("generated_image"))
            .filter(media_binding::Column::TargetId.eq(imported.id))
            .count(&db)
            .await
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn freeform_reset_keeps_gate_closed_and_reclaims_pending_image_uploads() {
    let db = setup_db().await;
    let user = insert_test_user(&db).await;
    let project = canvas_project(&db, "重置图片发布事务测试").await;
    assert_eq!(project.user_id, user.id);
    let publication_id = format!("reset-image-publication-{}", uuid::Uuid::new_v4());
    let staging_path = format!("projects/{}/images/pending.png.part", project.id);
    let final_path = format!("projects/{}/images/pending.png", project.id);
    let expected_sha256 = "c".repeat(64);
    crate::video::create_image_publication_intent(
        &db,
        a4n_domain::media::CreateImagePublicationIntentInput {
            publication_id: publication_id.clone(),
            project_id: project.id,
            requested_by_user_id: project.user_id,
            staging_path: staging_path.clone(),
            final_path: final_path.clone(),
            image: a4n_domain::media::CreateGeneratedImageRecordDraftInput {
                project_id: project.id,
                image_type: "video_reference".to_string(),
                file_path: final_path.clone(),
                prompt: "pending import".to_string(),
                model: a4n_domain::EXTERNAL_IMPORT_MODEL.to_string(),
                width: 32,
                height: 32,
                file_size: Some(4),
                chapter_id: None,
                panel_id: None,
                metadata_json: None,
                created_at: None,
            },
            expected_file_size: 4,
            expected_sha256: expected_sha256.clone(),
            allow_pending_replacement: false,
        },
    )
    .await
    .unwrap();
    set_project_gate(
        &db,
        &project,
        a4n_domain::generation::ProjectLifecycleOperation::Reset,
    )
    .await;

    let reset = crate::project::reset_freeform_video_generated_data_records(&db, project.id)
        .await
        .unwrap();

    let stored_project = novel_project::Entity::find_by_id(project.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored_project.lifecycle_gate, "resetting");
    assert!(reset.media_paths.contains(&staging_path));
    assert!(reset.media_paths.contains(&final_path));
    let publication = image_publication::Entity::find_by_id(&publication_id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        publication.status,
        a4n_domain::media::IMAGE_PUBLICATION_STATUS_RECLAIMING
    );

    let queued_image_write = crate::video::create_generated_image_record(
        &db,
        a4n_domain::media::CreateGeneratedImageRecordDraftInput {
            project_id: project.id,
            image_type: IMAGE_TYPE_CANVAS.to_string(),
            file_path: format!("projects/{}/images/queued-after-reset.png", project.id),
            prompt: "second-window write".to_string(),
            model: "test".to_string(),
            width: 32,
            height: 32,
            file_size: Some(4),
            chapter_id: None,
            panel_id: None,
            metadata_json: None,
            created_at: None,
        },
    )
    .await
    .expect_err("writes must remain blocked until the Host finishes reset cleanup");
    assert!(matches!(
        queued_image_write,
        crate::video::VideoDbError::InvalidState { .. }
    ));

    crate::generation::complete_project_reset_gate(&db, project.id)
        .await
        .unwrap();
    let error = crate::video::publish_image_publication(
        &db,
        &publication_id,
        a4n_domain::media::ImagePublicationFileFacts {
            file_size: 4,
            sha256: expected_sha256,
        },
    )
    .await
    .expect_err("a reset image publication must not be replayed afterward");
    assert!(matches!(
        error,
        crate::video::VideoDbError::InvalidState { .. }
    ));
}

#[tokio::test]
async fn project_delete_reclaims_pending_image_publications_and_cleans_their_paths() {
    let db = setup_db().await;
    let user = insert_test_user(&db).await;
    let project = text_project(&db, "删除期间图片发布测试").await;
    assert_eq!(project.user_id, user.id);
    let publication_id = format!("delete-image-publication-{}", uuid::Uuid::new_v4());
    let staging_path = format!("projects/{}/images/pending-delete.png.part", project.id);
    let final_path = format!("projects/{}/images/pending-delete.png", project.id);
    crate::video::create_image_publication_intent(
        &db,
        a4n_domain::media::CreateImagePublicationIntentInput {
            publication_id: publication_id.clone(),
            project_id: project.id,
            requested_by_user_id: project.user_id,
            staging_path: staging_path.clone(),
            final_path: final_path.clone(),
            image: a4n_domain::media::CreateGeneratedImageRecordDraftInput {
                project_id: project.id,
                image_type: "imported".to_string(),
                file_path: final_path.clone(),
                prompt: "pending import".to_string(),
                model: a4n_domain::EXTERNAL_IMPORT_MODEL.to_string(),
                width: 32,
                height: 32,
                file_size: Some(4),
                chapter_id: None,
                panel_id: None,
                metadata_json: None,
                created_at: None,
            },
            expected_file_size: 4,
            expected_sha256: "d".repeat(64),
            allow_pending_replacement: false,
        },
    )
    .await
    .unwrap();
    set_project_gate(
        &db,
        &project,
        a4n_domain::generation::ProjectLifecycleOperation::Delete,
    )
    .await;

    crate::project::delete::delete_project_cascade_record(&db, project.id)
        .await
        .unwrap();

    let publication = image_publication::Entity::find_by_id(&publication_id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        publication.status,
        a4n_domain::media::IMAGE_PUBLICATION_STATUS_RECLAIMING
    );
    let cleanup = cleanup_outbox::Entity::find()
        .filter(cleanup_outbox::Column::ProjectId.eq(project.id))
        .filter(cleanup_outbox::Column::CleanupKind.eq("project_delete"))
        .one(&db)
        .await
        .unwrap()
        .expect("project deletion must durably carry pending publication paths");
    let paths = cleanup.payload_json["media_paths"].as_array().unwrap();
    assert!(paths.contains(&serde_json::json!(staging_path)));
    assert!(paths.contains(&serde_json::json!(final_path)));
}
