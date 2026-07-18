use super::{
    remap_freeform_canvas_cache_key, remap_freeform_canvas_node_refs, remap_freeform_metadata,
    remap_freeform_source_external_path,
};
use crate::project::duplicate::ProjectDuplicateError;
use crate::project::duplicate::freeform::{FreeformCopyMaps, FreeformGeneratedImageCopyRecords};
use serde_json::json;

fn invalid_argument_message(error: ProjectDuplicateError) -> String {
    match error {
        ProjectDuplicateError::InvalidArgument(message) => message,
        other => panic!("expected invalid argument, got {other}"),
    }
}

fn invalid_state(error: ProjectDuplicateError) -> (String, String) {
    match error {
        ProjectDuplicateError::InvalidState { current, operation } => (current, operation),
        other => panic!("expected invalid state, got {other}"),
    }
}

#[test]
fn remap_freeform_canvas_legacy_image_ref_updates_id_and_file_path() {
    let mut copied_images = FreeformGeneratedImageCopyRecords::default();
    copied_images.ids.insert(10, 20);
    copied_images
        .file_paths
        .insert(10, "projects/20/media/duplicated/10_image.png".to_string());
    let maps = FreeformCopyMaps {
        images: copied_images,
        ..Default::default()
    };
    let (_, ref_id, config_json) = remap_freeform_canvas_node_refs(
        Some("legacy_generated_image".to_string()),
        Some(10),
        json!({"file_path": "projects/10/images/source.png"}),
        &maps,
        None,
    )
    .expect("legacy generated image ref should be remapped");
    assert_eq!(ref_id, Some(20));
    assert_eq!(
        config_json["file_path"],
        json!("projects/20/media/duplicated/10_image.png")
    );
}

#[test]
fn remap_freeform_canvas_text_asset_ref_requires_mapping() {
    let err = remap_freeform_canvas_node_refs(
        Some("canvas_text_asset".to_string()),
        Some(33),
        json!({}),
        &FreeformCopyMaps::default(),
        None,
    )
    .expect_err("text asset ref must not keep the source project id");
    let (current, operation) = invalid_state(err);
    assert!(current.contains("canvas_text_asset 引用 33 缺少目标映射"));
    assert_eq!(operation, "复制自由视频项目画布节点");
}

#[test]
fn remap_freeform_canvas_canonical_refs_update_id_and_config() {
    let mut maps = FreeformCopyMaps::default();
    maps.media_asset_ids.insert(44, 144);
    maps.video_segment_ids.insert(45, 145);
    maps.assembly_timeline_ids.insert(46, 146);
    let (_, ref_id, config_json) = remap_freeform_canvas_node_refs(
        Some("media_asset".to_string()),
        Some(44),
        json!({"media_asset_id":44,"video_segment_ids":[45],"assembly_timeline_id":46}),
        &maps,
        None,
    )
    .expect("canonical media refs should be remapped");
    assert_eq!(ref_id, Some(144));
    assert_eq!(config_json["media_asset_id"], json!(144));
    assert_eq!(config_json["video_segment_ids"], json!([145]));
    assert_eq!(config_json["assembly_timeline_id"], json!(146));
}

#[test]
fn remap_freeform_metadata_rejects_missing_project_local_id() {
    let err = remap_freeform_metadata(json!({"media_asset_id":999}), &FreeformCopyMaps::default())
        .expect_err("copy metadata must not keep source project ids");
    let (current, operation) = invalid_state(err);
    assert!(current.contains("media_asset_id=999 缺少目标映射"));
    assert_eq!(operation, "复制自由视频项目 JSON 引用");
}

#[test]
fn remap_freeform_metadata_rejects_non_integer_id_array() {
    let mut maps = FreeformCopyMaps::default();
    maps.video_segment_ids.insert(45, 145);
    let err = remap_freeform_metadata(json!({"video_segment_ids":[45,"stale"]}), &maps)
        .expect_err("copy metadata arrays must contain only integer ids");
    let message = invalid_argument_message(err);
    assert!(message.contains("video_segment_ids[1]") && message.contains("整数 ID"));
}

#[test]
fn remap_freeform_metadata_remaps_nested_canvas_ids() {
    let mut maps = FreeformCopyMaps::default();
    maps.media_asset_ids.insert(44, 144);
    maps.graph_ids.insert(50, 150);
    maps.node_ids.insert(51, 151);
    let metadata = remap_freeform_metadata(
        json!({"media_asset_id":44,"canvas":{"graph_id":50,"node_id":51}}),
        &maps,
    )
    .expect("known project-local ids should remap");
    assert_eq!(metadata["media_asset_id"], json!(144));
    assert_eq!(metadata["canvas"]["graph_id"], json!(150));
    assert_eq!(metadata["canvas"]["node_id"], json!(151));
}

#[test]
fn remap_freeform_source_external_path_rejects_missing_generated_ref() {
    let err = remap_freeform_source_external_path(
        Some("generated_video:99"),
        &FreeformCopyMaps::default(),
    )
    .expect_err("source external path must not keep stale generated video ids");
    let (current, operation) = invalid_state(err);
    assert!(current.contains("generated_video:99 缺少目标映射"));
    assert_eq!(operation, "复制媒体来源路径");
}

#[test]
fn remap_freeform_canvas_cache_key_rejects_missing_node_mapping() {
    let mut maps = FreeformCopyMaps::default();
    maps.graph_ids.insert(10, 110);
    let err = remap_freeform_canvas_cache_key(Some("canvas_graph:10:node:20"), &maps)
        .expect_err("cache key must not keep stale canvas node ids");
    let (current, operation) = invalid_state(err);
    assert!(current.contains("preview_cache_key node_id=20 缺少目标映射"));
    assert_eq!(operation, "复制成片时间线缓存键");
}
