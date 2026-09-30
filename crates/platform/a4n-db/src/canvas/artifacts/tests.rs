use super::*;
use crate::canvas::CanvasDbError;
use crate::canvas::{CanvasGraphRecordInput, CanvasNodeRecordInput, save_canvas_graph_record};
use crate::generation::{
    GENERATION_PROMPT_KIND_CANVAS_PROMPT, GENERATION_PROMPT_KIND_CANVAS_SHOT_SCRIPT,
    GENERATION_SPEC_TARGET_ENTITY_KIND_CANVAS_NODE, GENERATION_SPEC_TARGET_KIND_CANVAS_VIDEO,
};
use a4n_db_models::models::{
    canvas::canvas_node,
    generation::{generation_spec, prompt_artifact},
    media::{media_asset, media_binding, media_provenance},
    video_project::{assembly_clip, assembly_timeline, video_segment},
};
use a4n_domain::canvas::{
    CanvasAssemblyContractSyncInput, CanvasAssemblySourceVideoInput,
    CanvasPromptArtifactUpsertInput, FreeformCanvasGenerationSpecUpsertInput,
};
use a4n_domain::image::IMAGE_TYPE_CANVAS;
use a4n_domain::video_workflow::{WORKFLOW_MODE_CANVAS_ASSEMBLY, WORKFLOW_MODE_CANVAS_VIDEO};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
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
async fn canvas_prompt_artifact_keeps_user_edits() {
    let db = setup_db().await;
    let project = crate::project::create_freeform_video_project_record(
        &db,
        1,
        crate::project::CreateFreeformVideoProjectParams {
            title: "提示词人工编辑保留".to_string(),
            description: String::new(),
        },
    )
    .await
    .unwrap();
    let spec = upsert_freeform_canvas_generation_spec_record(
        &db,
        FreeformCanvasGenerationSpecUpsertInput {
            project_id: project.id,
            target_entity_id: 2,
            canvas_text_asset_id: None,
            target_kind: GENERATION_SPEC_TARGET_KIND_CANVAS_VIDEO.to_string(),
            target_entity_kind: GENERATION_SPEC_TARGET_ENTITY_KIND_CANVAS_NODE.to_string(),
            status: "running".to_string(),
            params_json: json!({"source_material": {"prompt": "初稿", "source_node_ids": []}}),
        },
    )
    .await
    .unwrap();
    let now = chrono::Utc::now().naive_utc();
    let existing = prompt_artifact::ActiveModel {
        generation_spec_id: Set(spec.id),
        prompt_kind: Set(GENERATION_PROMPT_KIND_CANVAS_PROMPT.to_string()),
        prompt_text: Set("人工改写".to_string()),
        renderer_version: Set("manual".to_string()),
        edited_by_user: Set(true),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();

    let record = upsert_canvas_prompt_artifact_record(
        &db,
        CanvasPromptArtifactUpsertInput {
            generation_spec_id: spec.id,
            prompt_kind: GENERATION_PROMPT_KIND_CANVAS_SHOT_SCRIPT.to_string(),
            prompt_text: "模型文本".to_string(),
        },
    )
    .await
    .unwrap();
    let stored = prompt_artifact::Entity::find_by_id(record.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(record.id, existing.id);
    assert_eq!(record.prompt_kind, GENERATION_PROMPT_KIND_CANVAS_PROMPT);
    assert_eq!(record.prompt_text, "人工改写");
    assert_eq!(stored.prompt_text, "人工改写");
    assert!(stored.edited_by_user);
    assert_eq!(stored.renderer_version, "manual");
}

#[tokio::test]
async fn canvas_node_artifact_update_rejects_a_changed_graph_version() {
    let db = setup_db().await;
    let project = crate::project::create_freeform_video_project_record(
        &db,
        1,
        crate::project::CreateFreeformVideoProjectParams {
            title: "画布节点版本保护".to_string(),
            description: String::new(),
        },
    )
    .await
    .unwrap();
    let graph = save_canvas_graph_record(
        &db,
        project.id,
        CanvasGraphRecordInput {
            id: None,
            expected_version: None,
            name: Some("版本保护".to_string()),
            graph_kind: Some("video_studio".to_string()),
            viewport_json: None,
            nodes: vec![CanvasNodeRecordInput {
                client_id: "source".to_string(),
                node_kind: "LlmTextNode".to_string(),
                ref_type: None,
                ref_id: None,
                config_json: Some(json!({"instruction": "before"})),
                position_json: json!({"x": 0, "y": 0}),
                state: Some("ready".to_string()),
            }],
            edges: Vec::new(),
        },
        chrono::Utc::now().naive_utc(),
    )
    .await
    .unwrap()
    .document;
    let node = graph.nodes.first().unwrap();
    let saved = save_canvas_graph_record(
        &db,
        project.id,
        CanvasGraphRecordInput {
            id: Some(graph.graph.id),
            expected_version: Some(graph.graph.version),
            name: None,
            graph_kind: Some("video_studio".to_string()),
            viewport_json: None,
            nodes: vec![CanvasNodeRecordInput {
                client_id: node.id.to_string(),
                node_kind: node.node_kind.clone(),
                ref_type: node.ref_type.clone(),
                ref_id: node.ref_id,
                config_json: Some(json!({"instruction": "after"})),
                position_json: node.position_json.clone(),
                state: Some("ready".to_string()),
            }],
            edges: Vec::new(),
        },
        chrono::Utc::now().naive_utc(),
    )
    .await
    .unwrap()
    .document;

    let error = update_canvas_node_artifact_record(
        &db,
        CanvasNodeArtifactUpdateInput {
            node_id: node.id,
            expected_graph_version: graph.graph.version,
            ref_type: "prompt_artifact".to_string(),
            ref_id: 99,
            config_patch: json!({"canvas_runtime_result": true, "text": "stale"}),
        },
    )
    .await
    .expect_err("a result from the old graph revision must not be attached");

    assert!(matches!(
        error,
        CanvasDbError::VersionConflict {
            graph_id,
            expected_version,
            current_version,
        } if graph_id == saved.graph.id
            && expected_version == graph.graph.version
            && current_version == saved.graph.version
    ));
    let persisted = canvas_node::Entity::find_by_id(node.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(persisted.config_json["instruction"], "after");
    assert!(
        !persisted
            .config_json
            .as_object()
            .unwrap()
            .contains_key("canvas_runtime_result")
    );
    assert_eq!(persisted.state, "ready");
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
            a4n_domain::media::CreateGeneratedVideoRecordDraftInput {
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
            &a4n_domain::media::VideoFileProperties {
                file_size: 100,
                width: video.width,
                height: video.height,
                duration_secs: video.duration_secs,
                fps: video.fps,
            },
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
        a4n_domain::media::CreateGeneratedVideoRecordDraftInput {
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
        &a4n_domain::media::VideoFileProperties {
            file_size: 300,
            width: assembly.width,
            height: assembly.height,
            duration_secs: assembly.duration_secs,
            fps: assembly.fps,
        },
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
