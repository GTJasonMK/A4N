use super::*;
use crate::canvas::CanvasDbError;
use crate::generation::{
    GENERATION_SPEC_TARGET_ENTITY_KIND_CANVAS_NODE, GENERATION_SPEC_TARGET_KIND_CANVAS_VIDEO,
};
use a4n_db_models::models::{
    generation::generation_spec,
    media::{media_asset, media_binding, media_provenance},
    video_project::{assembly_clip, assembly_timeline, video_segment},
};
use a4n_domain::image::IMAGE_TYPE_CANVAS;
use a4n_domain::video_workflow::{WORKFLOW_MODE_CANVAS_ASSEMBLY, WORKFLOW_MODE_CANVAS_VIDEO};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter, QueryOrder,
};
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
        a4n_domain::media::CreateGeneratedImageRecordDraftInput {
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

#[tokio::test]
async fn canvas_assembly_batch_write_preserves_segment_chain_bindings_and_clips() {
    let db = setup_db().await;
    let project = crate::project::create_freeform_video_project_record(
        &db,
        3,
        crate::project::CreateFreeformVideoProjectParams {
            title: "画布成片批量写入合同".to_string(),
            description: String::new(),
        },
    )
    .await
    .unwrap();

    let mut source_video_ids = Vec::new();
    for index in 0..3 {
        let video = crate::video::create_generated_video_record(
            &db,
            crate::video::CreateGeneratedVideoRecordDraftInput {
                project_id: project.id,
                chapter_id: None,
                video_type: a4n_domain::video_workflow::VIDEO_TYPE_CANVAS.to_string(),
                workflow_mode: Some(WORKFLOW_MODE_CANVAS_VIDEO.to_string()),
                prompt: format!("source-{index}"),
                negative_prompt: None,
                model: "test".to_string(),
                width: 1280,
                height: 720,
                duration_secs: index as f32 + 1.0,
                fps: 24,
                source_video_id: None,
                source_image_id: None,
                provider_job_id: None,
                metadata_json: None,
            },
        )
        .await
        .unwrap();
        let video = crate::video::mark_generated_video_completed_and_link_prompt_record(
            &db,
            video.id,
            &format!("projects/{}/source-{index}.mp4", project.id),
            None,
            100,
            None,
            None,
        )
        .await
        .unwrap();
        source_video_ids.push(video.id);
    }
    source_video_ids.push(source_video_ids[0]);
    let assembly = crate::video::create_generated_video_record(
        &db,
        crate::video::CreateGeneratedVideoRecordDraftInput {
            project_id: project.id,
            chapter_id: None,
            video_type: a4n_domain::video_workflow::VIDEO_TYPE_CANVAS_ASSEMBLY.to_string(),
            workflow_mode: Some(WORKFLOW_MODE_CANVAS_ASSEMBLY.to_string()),
            prompt: "assembly".to_string(),
            negative_prompt: None,
            model: "test".to_string(),
            width: 1280,
            height: 720,
            duration_secs: 6.0,
            fps: 24,
            source_video_id: None,
            source_image_id: None,
            provider_job_id: None,
            metadata_json: None,
        },
    )
    .await
    .unwrap();
    let assembly = crate::video::mark_generated_video_completed_and_link_prompt_record(
        &db,
        assembly.id,
        &format!("projects/{}/assembly.mp4", project.id),
        None,
        300,
        None,
        None,
    )
    .await
    .unwrap();

    let summary = sync_canvas_assembly_contract_records(
        &db,
        CanvasAssemblyContractSyncInput {
            project_id: project.id,
            graph_id: 17,
            node_id: 23,
            assembly_video_id: assembly.id,
            source_videos: source_video_ids
                .iter()
                .copied()
                .map(|video_id| CanvasAssemblySourceVideoInput { video_id })
                .collect(),
            generation_spec_id: None,
        },
    )
    .await
    .unwrap();

    let repeated_summary = sync_canvas_assembly_contract_records(
        &db,
        CanvasAssemblyContractSyncInput {
            project_id: project.id,
            graph_id: 17,
            node_id: 23,
            assembly_video_id: assembly.id,
            source_videos: source_video_ids
                .iter()
                .copied()
                .map(|video_id| CanvasAssemblySourceVideoInput { video_id })
                .collect(),
            generation_spec_id: None,
        },
    )
    .await
    .unwrap();

    let timeline = assembly_timeline::Entity::find()
        .filter(assembly_timeline::Column::ProjectId.eq(project.id))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    let segments = video_segment::Entity::find()
        .filter(video_segment::Column::ProjectId.eq(project.id))
        .order_by_asc(video_segment::Column::OrderIndex)
        .all(&db)
        .await
        .unwrap();
    let clips = assembly_clip::Entity::find()
        .filter(assembly_clip::Column::TimelineId.eq(timeline.id))
        .order_by_asc(assembly_clip::Column::OrderIndex)
        .all(&db)
        .await
        .unwrap();
    let bindings = media_binding::Entity::find()
        .filter(media_binding::Column::TargetKind.eq("video_segment"))
        .filter(media_binding::Column::TargetId.is_in(segments.iter().map(|segment| segment.id)))
        .all(&db)
        .await
        .unwrap();

    assert_eq!(
        summary.preview_media_asset_id,
        timeline.preview_media_asset_id.unwrap()
    );
    assert_eq!(repeated_summary, summary);
    assert_eq!(segments.len(), source_video_ids.len());
    assert_eq!(clips.len(), source_video_ids.len());
    assert_eq!(bindings.len(), source_video_ids.len());
    for (index, segment) in segments.iter().enumerate() {
        assert_eq!(
            segment.result_media_asset_id,
            Some(clips[index].media_asset_id)
        );
        assert_eq!(
            segment.previous_segment_id,
            index.checked_sub(1).map(|i| segments[i].id)
        );
        assert_eq!(
            segment.next_segment_id,
            segments.get(index + 1).map(|next| next.id)
        );
        assert_eq!(clips[index].source_segment_id, Some(segment.id));
    }
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
