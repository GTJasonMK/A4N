use super::*;
use crate::canvas::CanvasDbError;
use a4n_core::models::{
    generation::generation_spec,
    media::{media_asset, media_binding, media_provenance},
};
use a4n_domain::image::IMAGE_TYPE_CANVAS;
use crate::generation::{
    GENERATION_SPEC_TARGET_ENTITY_KIND_CANVAS_NODE,
    GENERATION_SPEC_TARGET_KIND_CANVAS_VIDEO,
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter};
use serde_json::json;

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

#[tokio::test]
async fn canvas_image_provenance_failure_rolls_back_asset_and_binding() {
    let db = setup_db().await;
    let project = crate::project::create_freeform_video_project_record(
        &db,
        1,
        crate::project::CreateFreeformVideoProjectParams {
            title: "画布媒体事务回滚测试".to_string(),
            description: String::new(),
        },
    )
    .await
    .unwrap();
    let image = crate::video::create_generated_image_record(
        &db,
        crate::video::CreateGeneratedImageRecordDraftInput {
            project_id: project.id,
            image_type: IMAGE_TYPE_CANVAS.to_string(),
            file_path: format!("projects/{}/images/rollback.png", project.id),
            prompt: "rollback".to_string(),
            model: "test".to_string(),
            width: 512,
            height: 512,
            file_size: Some(1024),
            chapter_id: None,
            panel_id: None,
            metadata_json: None,
            created_at: None,
        },
    )
    .await
    .unwrap();

    let err = ensure_canvas_image_media_asset_record(
        &db,
        CanvasImageMediaAssetEnsureInput {
            project_id: project.id,
            image,
            generation_spec_id: Some(i64::MAX),
            origin: "generated".to_string(),
        },
    )
    .await
    .expect_err("invalid provenance must roll back the canvas media asset");

    assert!(matches!(err, CanvasDbError::NotFound(message) if message.contains("生成意图不存在")));
    assert_eq!(
        media_asset::Entity::find()
            .filter(media_asset::Column::ProjectId.eq(project.id))
            .count(&db)
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        media_provenance::Entity::find().count(&db).await.unwrap(),
        0
    );
    assert_eq!(media_binding::Entity::find().count(&db).await.unwrap(), 0);
}

#[tokio::test]
async fn canvas_generation_spec_upsert_derives_source_version_hash() {
    let db = setup_db().await;
    let project = crate::project::create_freeform_video_project_record(
        &db,
        2,
        crate::project::CreateFreeformVideoProjectParams {
            title: "画布生成意图哈希测试".to_string(),
            description: String::new(),
        },
    )
    .await
    .unwrap();
    let params_json = json!({
        "source_material": {
            "prompt": "雨夜追逐",
            "source_node_ids": [11, 12]
        },
        "params": {"workflow": "generate_video"}
    });

    let spec = upsert_freeform_canvas_generation_spec_record(
        &db,
        FreeformCanvasGenerationSpecUpsertInput {
            project_id: project.id,
            target_entity_id: 21,
            canvas_text_asset_id: None,
            target_kind: GENERATION_SPEC_TARGET_KIND_CANVAS_VIDEO.to_string(),
            target_entity_kind: GENERATION_SPEC_TARGET_ENTITY_KIND_CANVAS_NODE.to_string(),
            status: "running".to_string(),
            params_json: params_json.clone(),
        },
    )
    .await
    .unwrap();

    assert_eq!(
        Some(spec.source_version_hash),
        generation_spec::freeform_source_version_hash(&params_json)
    );
}

#[test]
fn artifact_patch_removes_null_fields_and_updates_values() {
    let mut config = json!({"old": 1, "remove": true});
    merge_canvas_artifact_config_patch(
        &mut config,
        json!({"old": 2, "remove": null, "new": "value"}),
    );

    assert_eq!(config, json!({"old": 2, "new": "value"}));
}

#[test]
fn observation_patch_ignores_null_fields() {
    let mut config = json!({"keep": 1, "replace": "old"});
    merge_canvas_observation_config_patch(
        &mut config,
        json!({"keep": null, "replace": "new", "added": true}),
    );

    assert_eq!(config, json!({"keep": 1, "replace": "new", "added": true}));
}
