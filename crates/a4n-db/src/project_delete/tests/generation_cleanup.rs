use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, EntityTrait, PaginatorTrait, QueryFilter, Set, TransactionTrait,
};

use a4n_core::models::{
    ai::{generated_image, generated_video},
    media::{media_asset, media_binding, media_provenance},
};

use super::super::{GeneratedImageDeleteScope, delete_chapter_generation_outputs};
use super::support::{
    freeform_video_project, generated_image_with_asset, generated_video_with_asset, setup_db,
};

#[tokio::test]
async fn project_generation_cleanup_removes_only_generated_target_bindings() {
    let db = setup_db().await;
    let project = freeform_video_project(&db, "项目生成 binding 清理测试").await;
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
    let cleanup =
        delete_chapter_generation_outputs(&tx, project.id, GeneratedImageDeleteScope::Project)
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
    let project = freeform_video_project(&db, "自由画布重置目标复核测试").await;
    let (generated, generated_asset) =
        generated_image_with_asset(&db, project.id, "generated-reset.png").await;
    let imported = crate::video::create_generated_image_record(
        &db,
        crate::video::CreateGeneratedImageRecordDraftInput {
            project_id: project.id,
            image_type: "canvas_image".to_string(),
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
            image: imported.clone(),
            generation_spec_id: None,
            origin: "imported".to_string(),
        },
    )
    .await
    .unwrap();
    assert_eq!(generated_asset.id, imported_asset.id);

    let reset = crate::project::reset_freeform_video_generated_data_records(&db, project.id)
        .await
        .unwrap();

    assert_eq!(reset.artifact_row_cleanup.deleted_generated_images, 1);
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
