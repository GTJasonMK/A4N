use crate::AppState;
use a4n_core::models::{
    ai::{generated_image, generated_video},
    canvas::{canvas_graph, canvas_node},
    generation::{generation_spec, prompt_artifact},
    media::media_asset,
};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde_json::Value;
use std::collections::BTreeSet;

const EXTERNAL_IMPORT_MODEL: &str = "external-import";
const CANVAS_IMAGE_TYPE: &str = "canvas_image";
const CANVAS_VIDEO_TYPE: &str = "canvas_video";
const MEDIA_ORIGIN_IMPORTED: &str = "imported";

#[derive(Debug, Default)]
pub(super) struct FreeformGeneratedResetTargets {
    pub image_ids: Vec<i64>,
    pub video_ids: Vec<i64>,
    pub media_asset_ids: Vec<i64>,
    pub generation_spec_ids: Vec<i64>,
    pub prompt_artifact_ids: Vec<i64>,
    pub media_paths: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct MissingCanvasRef {
    pub ref_type: &'static str,
    pub ref_id: i64,
    pub clear_node_ref: bool,
}

const RESET_IMAGE_REF_CONFIG_KEYS: &[&str] = &[
    "generated_image_id",
    "legacy_generated_image_id",
    "source_image_id",
    "first_frame_image_id",
];
const RESET_VIDEO_REF_CONFIG_KEYS: &[&str] = &[
    "generated_video_id",
    "legacy_generated_video_id",
    "source_video_id",
    "video_id",
];
const RESET_GENERATION_SPEC_REF_CONFIG_KEYS: &[&str] = &["generation_spec_id"];
const RESET_PROMPT_ARTIFACT_REF_CONFIG_KEYS: &[&str] = &["prompt_artifact_id"];

pub(super) async fn collect_freeform_generated_reset_targets(
    state: &AppState,
    project_id: i64,
) -> Result<FreeformGeneratedResetTargets, String> {
    let mut targets = FreeformGeneratedResetTargets::default();
    let mut paths = BTreeSet::new();
    for image in generated_image::Entity::find()
        .filter(generated_image::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
    {
        if is_freeform_imported_canvas_image(&image) {
            continue;
        }
        paths.insert(image.file_path.clone());
        targets.image_ids.push(image.id);
    }
    for video in generated_video::Entity::find()
        .filter(generated_video::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
    {
        if is_freeform_imported_canvas_video(&video) {
            continue;
        }
        if let Some(path) = video.file_path.as_ref() {
            paths.insert(path.clone());
        }
        if let Some(path) = video.thumbnail_path.as_ref() {
            paths.insert(path.clone());
        }
        targets.video_ids.push(video.id);
    }
    for asset in media_asset::Entity::find()
        .filter(media_asset::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
    {
        if !is_freeform_generated_media_asset(&asset) {
            continue;
        }
        if let Some(path) = asset.file_path.as_ref() {
            paths.insert(path.clone());
        }
        if let Some(path) = asset.thumbnail_path.as_ref() {
            paths.insert(path.clone());
        }
        targets.media_asset_ids.push(asset.id);
    }
    targets.generation_spec_ids = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.eq(project_id))
        .all(state.db.as_ref())
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|row| row.id)
        .collect::<Vec<_>>();
    if !targets.generation_spec_ids.is_empty() {
        targets.prompt_artifact_ids = prompt_artifact::Entity::find()
            .filter(
                prompt_artifact::Column::GenerationSpecId
                    .is_in(targets.generation_spec_ids.clone()),
            )
            .all(state.db.as_ref())
            .await
            .map_err(|e| e.to_string())?
            .into_iter()
            .map(|row| row.id)
            .collect::<Vec<_>>();
    }
    targets.media_paths = paths.into_iter().collect();
    Ok(targets)
}

pub(super) async fn mark_freeform_canvas_generated_refs_missing<C>(
    db: &C,
    project_id: i64,
    targets: &FreeformGeneratedResetTargets,
) -> Result<(), String>
where
    C: sea_orm::ConnectionTrait,
{
    if targets.image_ids.is_empty()
        && targets.video_ids.is_empty()
        && targets.media_asset_ids.is_empty()
        && targets.generation_spec_ids.is_empty()
        && targets.prompt_artifact_ids.is_empty()
    {
        return Ok(());
    }
    let graph_ids = canvas_graph::Entity::find()
        .filter(canvas_graph::Column::ProjectId.eq(project_id))
        .all(db)
        .await
        .map_err(|e| e.to_string())?
        .into_iter()
        .map(|graph| graph.id)
        .collect::<Vec<_>>();
    if graph_ids.is_empty() {
        return Ok(());
    }
    let nodes = canvas_node::Entity::find()
        .filter(canvas_node::Column::GraphId.is_in(graph_ids))
        .all(db)
        .await
        .map_err(|e| e.to_string())?;
    let now = chrono::Utc::now().naive_utc();
    for node in nodes {
        let Some(missing_ref) = reset_missing_canvas_ref_for_node(&node, targets) else {
            continue;
        };
        let mut config = node.config_json.clone();
        if let Some(object) = config.as_object_mut() {
            object.insert(
                "missing_reason".to_string(),
                Value::String("referenced generated freeform asset was reset".to_string()),
            );
            object.insert(
                "missing_ref_type".to_string(),
                Value::String(missing_ref.ref_type.to_string()),
            );
            object.insert(
                "missing_ref_id".to_string(),
                Value::from(missing_ref.ref_id),
            );
            clear_reset_generated_ref_payload(object, targets);
        }
        let mut active: canvas_node::ActiveModel = node.into();
        if missing_ref.clear_node_ref {
            active.ref_type = Set(None);
            active.ref_id = Set(None);
        }
        active.state = Set("missing".to_string());
        active.config_json = Set(config);
        active.updated_at = Set(now);
        active.update(db).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}

pub(super) fn reset_missing_canvas_ref_for_node(
    node: &canvas_node::Model,
    targets: &FreeformGeneratedResetTargets,
) -> Option<MissingCanvasRef> {
    if node.ref_type.as_deref() == Some("media_asset")
        && let Some(ref_id) = node.ref_id
        && targets.media_asset_ids.contains(&ref_id)
    {
        return Some(MissingCanvasRef {
            ref_type: "media_asset",
            ref_id,
            clear_node_ref: true,
        });
    }
    if node.ref_type.as_deref() == Some("generation_spec")
        && let Some(ref_id) = node.ref_id
        && targets.generation_spec_ids.contains(&ref_id)
    {
        return Some(MissingCanvasRef {
            ref_type: "generation_spec",
            ref_id,
            clear_node_ref: true,
        });
    }
    if node.ref_type.as_deref() == Some("prompt_artifact")
        && let Some(ref_id) = node.ref_id
        && targets.prompt_artifact_ids.contains(&ref_id)
    {
        return Some(MissingCanvasRef {
            ref_type: "prompt_artifact",
            ref_id,
            clear_node_ref: true,
        });
    }
    if node.ref_type.as_deref() == Some("legacy_generated_image")
        && let Some(ref_id) = node.ref_id
        && targets.image_ids.contains(&ref_id)
    {
        return Some(MissingCanvasRef {
            ref_type: "generated_image",
            ref_id,
            clear_node_ref: true,
        });
    }
    if node.ref_type.as_deref() == Some("legacy_generated_video")
        && let Some(ref_id) = node.ref_id
        && targets.video_ids.contains(&ref_id)
    {
        return Some(MissingCanvasRef {
            ref_type: "generated_video",
            ref_id,
            clear_node_ref: true,
        });
    }
    if let Some(ref_id) = first_config_id_in(
        &node.config_json,
        RESET_IMAGE_REF_CONFIG_KEYS,
        &targets.image_ids,
    )
    .or_else(|| config_array_id_in(&node.config_json, "reference_image_ids", &targets.image_ids))
    {
        return Some(MissingCanvasRef {
            ref_type: "generated_image",
            ref_id,
            clear_node_ref: false,
        });
    }
    if let Some(ref_id) = first_config_id_in(
        &node.config_json,
        RESET_VIDEO_REF_CONFIG_KEYS,
        &targets.video_ids,
    )
    .or_else(|| config_array_id_in(&node.config_json, "video_ids", &targets.video_ids))
    {
        return Some(MissingCanvasRef {
            ref_type: "generated_video",
            ref_id,
            clear_node_ref: false,
        });
    }
    if let Some(ref_id) = first_config_id_in(
        &node.config_json,
        RESET_GENERATION_SPEC_REF_CONFIG_KEYS,
        &targets.generation_spec_ids,
    ) {
        return Some(MissingCanvasRef {
            ref_type: "generation_spec",
            ref_id,
            clear_node_ref: false,
        });
    }
    if let Some(ref_id) = first_config_id_in(
        &node.config_json,
        RESET_PROMPT_ARTIFACT_REF_CONFIG_KEYS,
        &targets.prompt_artifact_ids,
    ) {
        return Some(MissingCanvasRef {
            ref_type: "prompt_artifact",
            ref_id,
            clear_node_ref: false,
        });
    }
    first_config_id_in(
        &node.config_json,
        &["media_asset_id"],
        &targets.media_asset_ids,
    )
    .map(|ref_id| MissingCanvasRef {
        ref_type: "media_asset",
        ref_id,
        clear_node_ref: false,
    })
}

pub(super) fn clear_reset_generated_ref_payload(
    object: &mut serde_json::Map<String, Value>,
    targets: &FreeformGeneratedResetTargets,
) {
    remove_config_i64_refs(object, RESET_IMAGE_REF_CONFIG_KEYS, &targets.image_ids);
    remove_config_i64_array_refs(object, "reference_image_ids", &targets.image_ids);
    remove_config_i64_refs(object, RESET_VIDEO_REF_CONFIG_KEYS, &targets.video_ids);
    remove_config_i64_array_refs(object, "video_ids", &targets.video_ids);
    remove_config_i64_refs(
        object,
        RESET_GENERATION_SPEC_REF_CONFIG_KEYS,
        &targets.generation_spec_ids,
    );
    remove_config_i64_array_refs(object, "generation_spec_ids", &targets.generation_spec_ids);
    remove_config_i64_refs(
        object,
        RESET_PROMPT_ARTIFACT_REF_CONFIG_KEYS,
        &targets.prompt_artifact_ids,
    );
    remove_config_i64_array_refs(object, "prompt_artifact_ids", &targets.prompt_artifact_ids);
    remove_config_i64_refs(object, &["media_asset_id"], &targets.media_asset_ids);
    remove_config_i64_array_refs(object, "media_asset_ids", &targets.media_asset_ids);
}

fn first_config_id_in(config: &Value, keys: &[&str], ids: &[i64]) -> Option<i64> {
    keys.iter()
        .filter_map(|key| config.get(*key).and_then(Value::as_i64))
        .find(|id| ids.contains(id))
}

fn config_array_id_in(config: &Value, key: &str, ids: &[i64]) -> Option<i64> {
    config
        .get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_i64)
        .find(|id| ids.contains(id))
}

fn remove_config_i64_refs(object: &mut serde_json::Map<String, Value>, keys: &[&str], ids: &[i64]) {
    for key in keys {
        if object
            .get(*key)
            .and_then(Value::as_i64)
            .is_some_and(|id| ids.contains(&id))
        {
            object.remove(*key);
        }
    }
}

fn remove_config_i64_array_refs(
    object: &mut serde_json::Map<String, Value>,
    key: &str,
    ids: &[i64],
) {
    let Some(values) = object.get(key).and_then(Value::as_array) else {
        return;
    };
    let retained = values
        .iter()
        .filter(|value| value.as_i64().is_none_or(|id| !ids.contains(&id)))
        .cloned()
        .collect::<Vec<_>>();
    if retained.is_empty() {
        object.remove(key);
    } else {
        object.insert(key.to_string(), Value::Array(retained));
    }
}

pub(super) fn is_freeform_imported_canvas_image(image: &generated_image::Model) -> bool {
    image.image_type == CANVAS_IMAGE_TYPE
        && image.model == EXTERNAL_IMPORT_MODEL
        && image
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("canvas_import"))
            .and_then(|value| value.as_bool())
            .unwrap_or(false)
}

pub(super) fn is_freeform_imported_canvas_video(video: &generated_video::Model) -> bool {
    video.video_type == CANVAS_VIDEO_TYPE
        && video.workflow_mode.as_deref() == Some(CANVAS_VIDEO_TYPE)
        && video.model == EXTERNAL_IMPORT_MODEL
        && video
            .metadata_json
            .as_ref()
            .and_then(|metadata| metadata.get("canvas_import"))
            .is_some()
}

pub(super) fn is_freeform_generated_media_asset(asset: &media_asset::Model) -> bool {
    asset.origin != MEDIA_ORIGIN_IMPORTED
}
