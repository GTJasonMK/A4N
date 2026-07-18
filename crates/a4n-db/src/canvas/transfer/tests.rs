use super::*;
use crate::canvas::SOURCE_MODE_FREEFORM;
use a4n_core::models::generation::generation_spec;
use chrono::Utc;
use sea_orm::DbErr;
use serde_json::json;

fn generation_spec_with_target(
    target_entity_kind: &str,
    target_entity_id: Option<i64>,
) -> generation_spec::Model {
    let now = Utc::now().naive_utc();
    generation_spec::Model {
        id: 1,
        project_id: 1,
        root_project_id: 1,
        source_context_kind: Some(SOURCE_MODE_FREEFORM.to_string()),
        source_snapshot_id: None,
        source_chapter_id: None,
        source_snapshot_chapter_id: None,
        canvas_text_asset_id: None,
        target_kind: "video".to_string(),
        target_entity_kind: target_entity_kind.to_string(),
        target_entity_id,
        status: "completed".to_string(),
        source_version_hash: "hash".to_string(),
        params_json: json!({}),
        created_at: now,
        updated_at: now,
    }
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

    let metadata = remap_video_studio_canvas_metadata(
        json!({
            "media_asset_id": 10,
            "canvas": {
                "generated_video_id": 20
            }
        }),
        &maps,
    )
    .expect("known ids should remap");

    assert_eq!(metadata["media_asset_id"], json!(110));
    assert_eq!(metadata["canvas"]["generated_video_id"], json!(120));
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
