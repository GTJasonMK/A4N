use super::*;
use a4n_db_models::models::{
    ai::{generated_image, generated_video},
    canvas::{canvas_edge, canvas_graph, canvas_node, canvas_text_asset},
    generation::{generation_spec, prompt_artifact},
    media::media_asset,
    video_project::{assembly_clip, assembly_timeline, video_segment},
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DbErr, EntityTrait, QueryFilter, QueryOrder, Set};
use serde_json::json;
use std::collections::HashMap;

fn generation_spec_with_target(
    target_entity_kind: &str,
    target_entity_id: Option<i64>,
) -> generation_spec::Model {
    let now = Utc::now().naive_utc();
    let source_material = json!({"prompt":"prompt","source_node_ids":[]});
    let params_json = json!({"source_material":source_material});
    generation_spec::Model {
        id: 1,
        project_id: 1,
        root_project_id: 1,
        source_context_kind: Some(generation_spec::SOURCE_CONTEXT_FREEFORM.to_string()),
        source_snapshot_id: None,
        source_chapter_id: None,
        source_snapshot_chapter_id: None,
        canvas_text_asset_id: None,
        target_kind: "video".to_string(),
        target_entity_kind: target_entity_kind.to_string(),
        target_entity_id,
        status: "completed".to_string(),
        source_version_hash: generation_spec::freeform_source_version_hash(&params_json).unwrap(),
        params_json,
        created_at: now,
        updated_at: now,
    }
}

#[tokio::test]
async fn video_studio_import_batches_canvas_reference_fixups() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = crate::project::create_canvas_project_record(
        &db,
        1,
        crate::project::CreateCanvasProjectParams {
            title: "画布引用批量回填".to_string(),
            description: String::new(),
        },
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    let graph = canvas_graph::ActiveModel {
        project_id: Set(project.id),
        name: Set("回填测试画布".to_string()),
        graph_kind: Set("video_studio".to_string()),
        version: Set(1),
        viewport_json: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let target_nodes =
        canvas_node::Entity::insert_many((0..20).map(|index| canvas_node::ActiveModel {
            graph_id: Set(graph.id),
            node_kind: Set("LlmTextNode".to_string()),
            ref_type: Set(None),
            ref_id: Set(None),
            config_json: Set(json!({"before": true})),
            position_json: Set(json!({"x": index, "y": 0})),
            state: Set("ready".to_string()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }))
        .exec_with_returning_many(&db)
        .await
        .unwrap();
    let target_images =
        generated_image::Entity::insert_many((0..20).map(|index| generated_image::ActiveModel {
            project_id: Set(project.id),
            image_type: Set("canvas".to_string()),
            file_path: Set(format!("images/{index}.png")),
            prompt: Set(format!("image-{index}")),
            model: Set("test".to_string()),
            width: Set(512),
            height: Set(512),
            file_size: Set(None),
            ai_score: Set(None),
            is_favorite: Set(false),
            chapter_id: Set(None),
            panel_id: Set(None),
            metadata_json: Set(Some(json!({"before": true}))),
            created_at: Set(now),
            ..Default::default()
        }))
        .exec_with_returning_many(&db)
        .await
        .unwrap();
    let target_videos =
        generated_video::Entity::insert_many((0..20).map(|index| generated_video::ActiveModel {
            project_id: Set(project.id),
            chapter_id: Set(None),
            video_type: Set(a4n_domain::video_workflow::VIDEO_TYPE_CANVAS.to_string()),
            workflow_mode: Set(Some("canvas_video".to_string())),
            file_path: Set(None),
            thumbnail_path: Set(None),
            prompt: Set(format!("video-{index}")),
            negative_prompt: Set(None),
            model: Set("test".to_string()),
            width: Set(1280),
            height: Set(720),
            duration_secs: Set(1.0),
            fps: Set(24),
            file_size: Set(None),
            source_video_id: Set(None),
            source_image_id: Set(None),
            status: Set(a4n_domain::video_workflow::VIDEO_STATUS_PENDING.to_string()),
            provider_job_id: Set(None),
            error_message: Set(None),
            timeline_item_id: Set(None),
            metadata_json: Set(Some(json!({"before": true}))),
            is_favorite: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }))
        .exec_with_returning_many(&db)
        .await
        .unwrap();

    let mut maps = VideoStudioProjectImportIdMaps::default();
    let mut canvas_nodes = Vec::with_capacity(20);
    let mut generated_images = Vec::with_capacity(20);
    let mut generated_videos = Vec::with_capacity(20);
    for index in 0usize..20 {
        let old_node_id = 10_000 + index as i64;
        let old_image_id = 20_000 + index as i64;
        let old_video_id = 30_000 + index as i64;
        maps.node_ids.insert(old_node_id, target_nodes[index].id);
        maps.image_ids.insert(old_image_id, target_images[index].id);
        maps.video_ids.insert(old_video_id, target_videos[index].id);
        canvas_nodes.push(a4n_domain::canvas::CanvasNodeRecord {
            id: old_node_id,
            graph_id: 40_000,
            node_kind: "LlmTextNode".to_string(),
            ref_type: None,
            ref_id: None,
            config_json: json!({
                "generated_image_id": old_image_id,
                "generated_video_id": old_video_id,
            }),
            position_json: json!({"x": index, "y": 0}),
            state: "ready".to_string(),
            created_at: now,
            updated_at: now,
        });
        generated_images.push(generated_image::Model {
            id: old_image_id,
            project_id: 99,
            image_type: "canvas".to_string(),
            file_path: format!("images/{index}.png"),
            prompt: format!("image-{index}"),
            model: "test".to_string(),
            width: 512,
            height: 512,
            file_size: None,
            ai_score: None,
            is_favorite: false,
            chapter_id: None,
            panel_id: None,
            metadata_json: Some(json!({
                "generated_video_id": old_video_id,
                "node_id": old_node_id,
            })),
            created_at: now,
        });
        let mut video = target_videos[index].clone();
        video.id = old_video_id;
        video.project_id = 99;
        video.metadata_json = Some(json!({
            "generated_image_id": old_image_id,
            "node_id": old_node_id,
        }));
        generated_videos.push(video);
    }
    let rows = VideoStudioProjectImportRows {
        project: crate::project::project_record_from_model(project.clone()),
        canvas_graphs: Vec::new(),
        canvas_nodes,
        canvas_edges: Vec::new(),
        canvas_text_assets: Vec::new(),
        generation_specs: Vec::new(),
        prompt_artifacts: Vec::new(),
        generated_images,
        generated_videos,
        media_assets: Vec::new(),
        media_provenance: Vec::new(),
        media_bindings: Vec::new(),
        video_segments: Vec::new(),
        assembly_timelines: Vec::new(),
        assembly_clips: Vec::new(),
    };

    super::import_media::update_imported_canvas_node_refs(&db, &rows, &maps, now)
        .await
        .unwrap();
    super::import_media::update_imported_media_metadata(&db, &rows, &maps, now)
        .await
        .unwrap();

    let stored_nodes = canvas_node::Entity::find()
        .filter(canvas_node::Column::GraphId.eq(graph.id))
        .order_by_asc(canvas_node::Column::Id)
        .all(&db)
        .await
        .unwrap();
    for (index, node) in stored_nodes.iter().enumerate() {
        assert_eq!(
            node.config_json["generated_image_id"],
            json!(target_images[index].id)
        );
        assert_eq!(
            node.config_json["generated_video_id"],
            json!(target_videos[index].id)
        );
    }
    let stored_images = generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(project.id))
        .order_by_asc(generated_image::Column::Id)
        .all(&db)
        .await
        .unwrap();
    let stored_videos = generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.eq(project.id))
        .order_by_asc(generated_video::Column::Id)
        .all(&db)
        .await
        .unwrap();
    for (index, image) in stored_images.iter().enumerate() {
        assert_eq!(
            image.metadata_json.as_ref().unwrap()["generated_video_id"],
            json!(target_videos[index].id)
        );
    }
    for (index, video) in stored_videos.iter().enumerate() {
        assert_eq!(
            video.metadata_json.as_ref().unwrap()["generated_image_id"],
            json!(target_images[index].id)
        );
    }
}

#[tokio::test]
async fn video_studio_import_batches_all_media_rows_and_preserves_id_maps() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = crate::project::create_canvas_project_record(
        &db,
        1,
        crate::project::CreateCanvasProjectParams {
            title: "画布媒体批量导入".to_string(),
            description: String::new(),
        },
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    let images = (0..4)
        .map(|index| generated_image::Model {
            id: 10_000 + index,
            project_id: 99,
            image_type: "canvas".to_string(),
            file_path: format!("source/image-{index}.png"),
            prompt: format!("image-{index}"),
            model: "test".to_string(),
            width: 512,
            height: 512,
            file_size: None,
            ai_score: None,
            is_favorite: false,
            chapter_id: None,
            panel_id: None,
            metadata_json: Some(json!({"source": index})),
            created_at: now,
        })
        .collect::<Vec<_>>();
    let videos = (0..3)
        .map(|index| generated_video::Model {
            id: 20_000 + index,
            project_id: 99,
            chapter_id: None,
            video_type: a4n_domain::video_workflow::VIDEO_TYPE_CANVAS.to_string(),
            workflow_mode: Some("canvas_video".to_string()),
            file_path: Some(format!("source/video-{index}.mp4")),
            thumbnail_path: Some(format!("source/video-{index}.webp")),
            prompt: format!("video-{index}"),
            negative_prompt: None,
            model: "test".to_string(),
            width: 1280,
            height: 720,
            duration_secs: 1.0,
            fps: 24,
            file_size: None,
            source_video_id: None,
            source_image_id: None,
            status: a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED.to_string(),
            provider_job_id: None,
            error_message: None,
            timeline_item_id: None,
            metadata_json: Some(json!({"source": index})),
            is_favorite: false,
            created_at: now,
            updated_at: now,
        })
        .collect::<Vec<_>>();
    let assets = (0..2)
        .map(|index| media_asset::Model {
            id: 30_000 + index,
            project_id: 99,
            root_project_id: 99,
            source_snapshot_id: None,
            source_chapter_id: None,
            source_snapshot_chapter_id: None,
            media_kind: "image".to_string(),
            origin: "imported".to_string(),
            file_path: Some(format!("source/asset-{index}.png")),
            thumbnail_path: Some(format!("source/asset-{index}.webp")),
            width: Some(512),
            height: Some(512),
            duration_secs: None,
            fps: None,
            file_size: None,
            checksum: None,
            status: "completed".to_string(),
            error_message: None,
            created_at: now,
            updated_at: now,
        })
        .collect::<Vec<_>>();

    struct TestWriter;
    impl VideoStudioProjectImportMediaFileWriter for TestWriter {
        fn write_imported_media_file(
            &mut self,
            _project_id: i64,
            _owner_kind: &str,
            owner_id: i64,
            field: &str,
            folder: &str,
            _stem: &str,
        ) -> Result<Option<String>, VideoStudioProjectImportMediaError> {
            Ok(Some(format!("{folder}/{owner_id}/{field}")))
        }
    }

    let mut maps = VideoStudioProjectImportIdMaps::default();
    let mut writer = TestWriter;
    let mut context = super::import_project::VideoStudioProjectImportMediaRowsContext {
        db: &db,
        project_id: project.id,
        maps: &mut maps,
        media_file_writer: &mut writer,
        now,
    };
    super::import_project::import_video_studio_generated_images(&mut context, &images)
        .await
        .unwrap();
    let imported_videos =
        super::import_project::import_video_studio_generated_videos(&mut context, &videos)
            .await
            .unwrap();
    super::import_media::import_video_studio_media_assets(&mut context, &assets)
        .await
        .unwrap();

    assert_eq!(imported_videos.len(), videos.len());
    assert_eq!(maps.image_ids.len(), images.len());
    assert_eq!(maps.video_ids.len(), videos.len());
    assert_eq!(maps.media_asset_ids.len(), assets.len());
    for image in &images {
        assert_eq!(
            maps.image_file_paths.get(&image.id),
            Some(&format!("images/{}/file_path", image.id))
        );
    }
    for video in &videos {
        assert_eq!(
            maps.video_file_paths.get(&video.id),
            Some(&Some(format!("videos/{}/file_path", video.id)))
        );
        assert_eq!(
            maps.video_thumbnail_paths.get(&video.id),
            Some(&Some(format!("videos/{}/thumbnail_path", video.id)))
        );
    }
    assert_eq!(
        generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.eq(project.id))
            .all(&db)
            .await
            .unwrap()
            .len(),
        images.len()
    );
    assert_eq!(
        generated_video::Entity::find()
            .filter(generated_video::Column::ProjectId.eq(project.id))
            .all(&db)
            .await
            .unwrap()
            .len(),
        videos.len()
    );
    assert_eq!(
        media_asset::Entity::find()
            .filter(media_asset::Column::ProjectId.eq(project.id))
            .all(&db)
            .await
            .unwrap()
            .len(),
        assets.len()
    );
}

#[test]
fn video_studio_import_remap_canvas_node_refs_rejects_missing_project_local_ref() {
    let maps = VideoStudioProjectImportIdMaps::default();

    let err = remap_video_studio_canvas_node_refs(
        Some("media_asset".to_string()),
        Some(999),
        json!({}),
        &maps,
    )
    .expect_err("missing media asset remap must fail");

    assert_video_studio_invalid_package_error(err, &["canvas_node.ref_id", "999"]);
}

#[test]
fn video_studio_import_remap_canvas_metadata_rejects_missing_project_local_id() {
    let maps = VideoStudioProjectImportIdMaps::default();

    let err = remap_video_studio_canvas_metadata(json!({ "media_asset_id": 999 }), &maps)
        .expect_err("missing media asset metadata remap must fail");

    assert_video_studio_invalid_package_error(err, &["media_asset_id", "999"]);
}

#[test]
fn video_studio_import_remap_canvas_metadata_remaps_known_project_local_ids() {
    let mut maps = VideoStudioProjectImportIdMaps::default();
    maps.media_asset_ids.insert(10, 110);
    maps.video_ids.insert(20, 120);
    maps.image_ids.insert(30, 130);
    maps.graph_ids.insert(40, 140);
    maps.node_ids.insert(50, 150);

    let metadata = remap_video_studio_canvas_metadata(
        json!({
            "media_asset_id": 10,
            "config_id": 60,
            "first_frame_image_id": 30,
            "last_frame_image_id": 30,
            "reference_image_ids": [30],
            "canvas": {
                "graph_id": 40,
                "node_id": 50,
                "generated_video_id": 20,
                "source_node_ids": [50],
                "workflow_run_id": 61
            },
            "source_material": {
                "video_ids": [20],
                "source_node_ids": [50]
            },
            "metadata": {"generated_video_id": 20, "provider_job_id": "job-62"}
        }),
        &maps,
    )
    .expect("known ids should remap");

    assert_eq!(metadata["media_asset_id"], json!(110));
    assert_eq!(metadata["first_frame_image_id"], json!(130));
    assert_eq!(metadata["last_frame_image_id"], json!(130));
    assert_eq!(metadata["reference_image_ids"], json!([130]));
    assert_eq!(metadata["canvas"]["graph_id"], json!(140));
    assert_eq!(metadata["canvas"]["node_id"], json!(150));
    assert_eq!(metadata["canvas"]["generated_video_id"], json!(120));
    assert_eq!(metadata["canvas"]["source_node_ids"], json!([150]));
    assert_eq!(metadata["source_material"]["video_ids"], json!([120]));
    assert_eq!(metadata["source_material"]["source_node_ids"], json!([150]));
    assert_eq!(metadata["metadata"]["generated_video_id"], json!(120));
    assert!(metadata.get("config_id").is_none());
    assert!(metadata["canvas"].get("workflow_run_id").is_none());
    assert!(metadata["metadata"].get("provider_job_id").is_none());
}

#[test]
fn video_studio_import_remap_canvas_cache_key_remaps_known_graph_and_node_ids() {
    let mut maps = VideoStudioProjectImportIdMaps::default();
    maps.graph_ids.insert(10, 110);
    maps.node_ids.insert(20, 120);

    let remapped = remap_video_studio_canvas_cache_key(Some("canvas_graph:10:node:20"), &maps)
        .expect("known graph and node ids should remap");

    assert_eq!(remapped.as_deref(), Some("canvas_graph:110:node:120"));
    assert_eq!(
        remap_video_studio_canvas_cache_key(Some("custom-cache-key"), &maps)
            .expect("custom cache key should be preserved")
            .as_deref(),
        Some("custom-cache-key")
    );
}

#[test]
fn video_studio_import_remap_canvas_cache_key_rejects_missing_node_mapping() {
    let mut maps = VideoStudioProjectImportIdMaps::default();
    maps.graph_ids.insert(10, 110);

    let err = remap_video_studio_canvas_cache_key(Some("canvas_graph:10:node:20"), &maps)
        .expect_err("missing node remap must fail");

    assert_video_studio_invalid_package_error(
        err,
        &["assembly_timeline.preview_cache_key node_id", "20"],
    );
}

#[test]
fn video_studio_import_remap_generation_spec_target_remaps_media_asset_target() {
    let mut maps = VideoStudioProjectImportIdMaps::default();
    maps.media_asset_ids.insert(10, 110);
    let spec = generation_spec_with_target("media_asset", Some(10));

    let target_id = remap_video_studio_generation_spec_target_entity_id(&spec, &maps)
        .expect("known media asset target should remap");

    assert_eq!(target_id, Some(110));
}

#[test]
fn video_studio_import_remap_generation_spec_target_rejects_missing_video_segment_target() {
    let spec = generation_spec_with_target("video_segment", Some(99));

    let err = remap_video_studio_generation_spec_target_entity_id(
        &spec,
        &VideoStudioProjectImportIdMaps::default(),
    )
    .expect_err("missing video segment target must fail");

    assert_video_studio_invalid_package_error(err, &["generation_spec.target_entity_id", "99"]);
}

#[test]
fn video_studio_import_error_preserves_database_source_and_media_classification() {
    let database_error = VideoStudioProjectImportError::Database(DbErr::Custom(
        "canvas import database failure".to_string(),
    ));
    assert!(matches!(
        database_error,
        VideoStudioProjectImportError::Database(DbErr::Custom(message))
            if message == "canvas import database failure"
    ));

    let media_error = VideoStudioProjectImportError::MediaWrite(
        VideoStudioProjectImportMediaError::Write("canvas import media failure".to_string()),
    );
    assert!(matches!(
        media_error,
        VideoStudioProjectImportError::MediaWrite(
            VideoStudioProjectImportMediaError::Write(message)
        )
            if message == "canvas import media failure"
    ));
}

#[tokio::test]
async fn video_studio_import_batches_canvas_text_assets_and_preserves_mapping_order() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = crate::project::create_project_record(
        &db,
        crate::project::CreateProjectRecordInput {
            user_id: 1,
            title: "画布文本资产批量导入".to_string(),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    let assets = (0..20)
        .map(|index| a4n_domain::canvas::CanvasTextAssetRecord {
            id: 10_000 + index,
            project_id: 99,
            title: format!("文本 {index}"),
            text_kind: "prompt".to_string(),
            content: format!("内容 {index}"),
            content_hash: format!("hash-{index}"),
            current_version: 1,
            versions: Vec::new(),
            created_at: now,
            updated_at: now,
        })
        .collect::<Vec<_>>();
    let mut maps = VideoStudioProjectImportIdMaps::default();

    super::import_canvas::import_video_studio_canvas_text_assets(
        &db, project.id, &assets, &mut maps, now,
    )
    .await
    .unwrap();

    let rows = canvas_text_asset::Entity::find()
        .filter(canvas_text_asset::Column::ProjectId.eq(project.id))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(rows.len(), assets.len());
    assert_eq!(maps.text_asset_ids.len(), assets.len());
    for asset in assets {
        assert!(maps.text_asset_ids.contains_key(&asset.id));
    }
}

#[tokio::test]
async fn video_studio_import_batches_canvas_topology_and_builds_id_maps() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = crate::project::create_canvas_project_record(
        &db,
        1,
        crate::project::CreateCanvasProjectParams {
            title: "画布拓扑批量导入".to_string(),
            description: String::new(),
        },
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    let canvas_graphs = (0..20)
        .map(|index| a4n_domain::canvas::CanvasGraphRecord {
            id: 20_000 + index,
            project_id: 99,
            name: format!("图 {index}"),
            graph_kind: "freeform".to_string(),
            version: 1,
            viewport_json: Some(json!({})),
            created_at: now,
            updated_at: now,
        })
        .collect::<Vec<_>>();
    let canvas_nodes = (0..20)
        .map(|index| a4n_domain::canvas::CanvasNodeRecord {
            id: 30_000 + index,
            graph_id: 20_000 + index,
            node_kind: "text_input".to_string(),
            ref_type: None,
            ref_id: None,
            config_json: json!({}),
            position_json: json!({"x": index, "y": 0}),
            state: "ready".to_string(),
            created_at: now,
            updated_at: now,
        })
        .collect::<Vec<_>>();
    let canvas_edges = (1..20)
        .map(|index| a4n_domain::canvas::CanvasEdgeRecord {
            id: 40_000 + index,
            graph_id: 20_000 + index - 1,
            source_node_id: 30_000 + index - 1,
            source_port: "output".to_string(),
            target_node_id: 30_000 + index,
            target_port: "input".to_string(),
            role: "default".to_string(),
            config_json: json!({}),
        })
        .collect::<Vec<_>>();
    let rows = VideoStudioProjectImportRows {
        project: crate::project::project_record_from_model(project.clone()),
        canvas_graphs,
        canvas_nodes,
        canvas_edges,
        canvas_text_assets: Vec::new(),
        generation_specs: Vec::new(),
        prompt_artifacts: Vec::new(),
        generated_images: Vec::new(),
        generated_videos: Vec::new(),
        media_assets: Vec::new(),
        media_provenance: Vec::new(),
        media_bindings: Vec::new(),
        video_segments: Vec::new(),
        assembly_timelines: Vec::new(),
        assembly_clips: Vec::new(),
    };
    let mut maps = VideoStudioProjectImportIdMaps::default();

    super::import_canvas::import_video_studio_canvas_graphs(&db, project.id, &rows, &mut maps, now)
        .await
        .unwrap();

    let graph_rows = canvas_graph::Entity::find()
        .filter(canvas_graph::Column::ProjectId.eq(project.id))
        .all(&db)
        .await
        .unwrap();
    let graph_ids = graph_rows.iter().map(|row| row.id).collect::<Vec<_>>();
    let node_rows = canvas_node::Entity::find()
        .filter(canvas_node::Column::GraphId.is_in(graph_ids.clone()))
        .all(&db)
        .await
        .unwrap();
    let edge_rows = canvas_edge::Entity::find()
        .filter(canvas_edge::Column::GraphId.is_in(graph_ids))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(graph_rows.len(), 20);
    assert_eq!(node_rows.len(), 20);
    assert_eq!(edge_rows.len(), 19);
    assert_eq!(maps.graph_ids.len(), 20);
    assert_eq!(maps.node_ids.len(), 20);

    let specs = (0..20)
        .map(|index| {
            let params_json = json!({
                "source_material": {"source_node_ids": [30_000 + index]}
            });
            generation_spec::Model {
                id: 50_000 + index,
                project_id: 99,
                root_project_id: 99,
                source_context_kind: Some(generation_spec::SOURCE_CONTEXT_FREEFORM.to_string()),
                source_snapshot_id: None,
                source_chapter_id: None,
                source_snapshot_chapter_id: None,
                canvas_text_asset_id: None,
                target_kind: "video".to_string(),
                target_entity_kind: "media_asset".to_string(),
                target_entity_id: Some(70_000 + index),
                status: "completed".to_string(),
                source_version_hash: generation_spec::freeform_source_version_hash(&params_json)
                    .unwrap(),
                params_json,
                created_at: now,
                updated_at: now,
            }
        })
        .collect::<Vec<_>>();
    let deferred = super::import_canvas::import_video_studio_generation_specs(
        &db, project.id, &specs, &mut maps, now,
    )
    .await
    .unwrap();
    assert_eq!(deferred.len(), 20);
    assert_eq!(maps.generation_spec_ids.len(), 20);

    for index in 0..20 {
        maps.media_asset_ids.insert(70_000 + index, 80_000 + index);
    }
    super::import_canvas::update_imported_generation_spec_targets(&db, &deferred, &maps, now)
        .await
        .unwrap();
    super::import_canvas::update_imported_generation_spec_metadata(&db, &specs, &maps, now)
        .await
        .unwrap();

    let stored_specs = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.eq(project.id))
        .order_by_asc(generation_spec::Column::Id)
        .all(&db)
        .await
        .unwrap();
    assert_eq!(stored_specs.len(), specs.len());
    for (index, stored) in stored_specs.iter().enumerate() {
        assert_eq!(stored.target_entity_id, Some(80_000 + index as i64));
        assert_eq!(
            stored.params_json["source_material"]["source_node_ids"],
            json!([maps.node_ids[&(30_000 + index as i64)]])
        );
        assert_eq!(
            stored.source_version_hash,
            generation_spec::freeform_source_version_hash(&stored.params_json).unwrap()
        );
    }

    let artifacts = (0..20)
        .map(|index| prompt_artifact::Model {
            id: 60_000 + index,
            generation_spec_id: 50_000 + index,
            prompt_kind: "video".to_string(),
            prompt_text: format!("提示词 {index}"),
            renderer_version: "test".to_string(),
            edited_by_user: false,
            created_at: now,
            updated_at: now,
        })
        .collect::<Vec<_>>();
    super::import_canvas::import_video_studio_prompt_artifacts(&db, &artifacts, &mut maps, now)
        .await
        .unwrap();
    assert_eq!(maps.prompt_artifact_ids.len(), 20);
}

#[tokio::test]
async fn video_studio_import_batches_generated_video_source_ref_updates() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = crate::project::create_canvas_project_record(
        &db,
        1,
        crate::project::CreateCanvasProjectParams {
            title: "生成视频来源批量回填".to_string(),
            description: String::new(),
        },
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    let create_video = || generated_video::ActiveModel {
        project_id: Set(project.id),
        video_type: Set(a4n_domain::video_workflow::VIDEO_TYPE_CANVAS.to_string()),
        prompt: Set("prompt".to_string()),
        model: Set("model".to_string()),
        width: Set(1),
        height: Set(1),
        duration_secs: Set(1.0),
        fps: Set(24),
        status: Set(a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED.to_string()),
        is_favorite: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };
    let imported_a = create_video().insert(&db).await.unwrap();
    let imported_b = create_video().insert(&db).await.unwrap();
    let mut source_a = imported_a.clone();
    source_a.id = 10;
    source_a.source_video_id = Some(20);
    source_a.source_image_id = Some(30);
    let mut source_b = imported_b.clone();
    source_b.id = 20;
    source_b.source_video_id = None;
    source_b.source_image_id = Some(30);
    let mut maps = VideoStudioProjectImportIdMaps::default();
    maps.video_ids.insert(20, imported_b.id);
    maps.image_ids.insert(30, 300);
    let imported_videos = HashMap::from([(10, imported_a.clone()), (20, imported_b.clone())]);

    super::import_project::update_imported_video_source_refs(
        &db,
        &[source_a, source_b],
        &imported_videos,
        &maps,
        now,
    )
    .await
    .unwrap();

    let rows = generated_video::Entity::find()
        .filter(generated_video::Column::Id.is_in([imported_a.id, imported_b.id]))
        .all(&db)
        .await
        .unwrap();
    let row_a = rows.iter().find(|row| row.id == imported_a.id).unwrap();
    let row_b = rows.iter().find(|row| row.id == imported_b.id).unwrap();
    assert_eq!(row_a.source_video_id, Some(imported_b.id));
    assert_eq!(row_a.source_image_id, Some(300));
    assert_eq!(row_b.source_video_id, None);
    assert_eq!(row_b.source_image_id, Some(300));
}

#[tokio::test]
async fn video_studio_import_batches_video_segments_and_chain_updates() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = crate::project::create_canvas_project_record(
        &db,
        1,
        crate::project::CreateCanvasProjectParams {
            title: "视频片段批量导入".to_string(),
            description: String::new(),
        },
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    let segments = (0..20)
        .map(|index| video_segment::Model {
            id: 10_000 + index,
            project_id: 99,
            root_project_id: 99,
            source_chapter_id: None,
            source_snapshot_chapter_id: None,
            segment_type: "video".to_string(),
            order_index: index as i32,
            title: format!("片段 {index}"),
            duration_secs: 1.0,
            source_generation_spec_id: None,
            result_media_asset_id: None,
            previous_segment_id: (index > 0).then_some(9_999 + index),
            next_segment_id: (index < 19).then_some(10_001 + index),
            created_at: now,
            updated_at: now,
        })
        .collect::<Vec<_>>();
    let mut maps = VideoStudioProjectImportIdMaps::default();

    super::import_media::import_video_studio_video_segments(
        &db, project.id, &segments, &mut maps, now,
    )
    .await
    .unwrap();

    let rows = video_segment::Entity::find()
        .filter(video_segment::Column::ProjectId.eq(project.id))
        .all(&db)
        .await
        .unwrap();
    assert_eq!(rows.len(), segments.len());
    assert_eq!(maps.video_segment_ids.len(), segments.len());
    let first = rows.iter().find(|row| row.order_index == 0).unwrap();
    let second = rows.iter().find(|row| row.order_index == 1).unwrap();
    assert_eq!(first.previous_segment_id, None);
    assert_eq!(first.next_segment_id, Some(second.id));
    assert_eq!(second.previous_segment_id, Some(first.id));
}

#[tokio::test]
async fn video_studio_import_batches_assembly_timelines_and_clips() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project = crate::project::create_canvas_project_record(
        &db,
        1,
        crate::project::CreateCanvasProjectParams {
            title: "成片批量导入".to_string(),
            description: String::new(),
        },
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    let media_asset = media_asset::ActiveModel {
        project_id: Set(project.id),
        root_project_id: Set(project.id),
        media_kind: Set("video".to_string()),
        origin: Set("generated".to_string()),
        file_path: Set(Some("videos/imported.mp4".to_string())),
        status: Set("completed".to_string()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let timeline = assembly_timeline::Model {
        id: 20_000,
        project_id: 99,
        source_chapter_id: None,
        source_snapshot_chapter_id: None,
        version: 1,
        status: "draft".to_string(),
        preview_media_asset_id: None,
        preview_cache_key: None,
        created_at: now,
        updated_at: now,
    };
    let clip = assembly_clip::Model {
        id: 21_000,
        timeline_id: timeline.id,
        media_asset_id: 30_000,
        order_index: 0,
        trim_start_secs: 0.0,
        trim_end_secs: 1.0,
        volume: 1.0,
        transition_kind: "cut".to_string(),
        source_segment_id: None,
    };
    let rows = VideoStudioProjectImportRows {
        project: crate::project::project_record_from_model(project.clone()),
        canvas_graphs: Vec::new(),
        canvas_nodes: Vec::new(),
        canvas_edges: Vec::new(),
        canvas_text_assets: Vec::new(),
        generation_specs: Vec::new(),
        prompt_artifacts: Vec::new(),
        generated_images: Vec::new(),
        generated_videos: Vec::new(),
        media_assets: Vec::new(),
        media_provenance: Vec::new(),
        media_bindings: Vec::new(),
        video_segments: Vec::new(),
        assembly_timelines: vec![timeline],
        assembly_clips: vec![clip],
    };
    let mut maps = VideoStudioProjectImportIdMaps::default();
    maps.media_asset_ids.insert(30_000, media_asset.id);

    super::import_media::import_video_studio_assembly_rows(&db, project.id, &rows, &mut maps, now)
        .await
        .unwrap();

    assert_eq!(maps.assembly_timeline_ids.len(), 1);
    assert_eq!(maps.assembly_clip_ids.len(), 1);
    let stored_clip = assembly_clip::Entity::find()
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored_clip.timeline_id, maps.assembly_timeline_ids[&20_000]);
    assert_eq!(stored_clip.media_asset_id, media_asset.id);
}

fn assert_video_studio_invalid_package_error(
    err: VideoStudioProjectImportError,
    expected_fragments: &[&str],
) {
    let VideoStudioProjectImportError::InvalidPackage(message) = err else {
        panic!("expected invalid package error, got {err:?}");
    };
    for fragment in expected_fragments {
        assert!(
            message.contains(fragment),
            "expected error message to contain {fragment:?}, got {message:?}"
        );
    }
}
