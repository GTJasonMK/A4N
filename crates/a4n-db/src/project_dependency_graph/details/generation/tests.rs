use a4n_core::models::{
    generation::{generation_spec, prompt_artifact},
    media::{media_asset, media_provenance},
};
use chrono::Utc;
use serde_json::json;

use super::{
    media_assets::media_asset_record, prompt_artifacts::prompt_artifact_record,
    specs::generation_spec_record,
};

fn generation_spec(
    source_chapter_id: Option<i64>,
    source_snapshot_chapter_id: Option<i64>,
) -> generation_spec::Model {
    let now = Utc::now().naive_utc();
    generation_spec::Model {
        id: 11,
        project_id: 1,
        root_project_id: 1,
        source_context_kind: None,
        source_snapshot_id: None,
        source_chapter_id,
        source_snapshot_chapter_id,
        canvas_text_asset_id: None,
        target_kind: "chapter_video".to_string(),
        target_entity_kind: "chapter".to_string(),
        target_entity_id: Some(1),
        status: "completed".to_string(),
        source_version_hash: "hash".to_string(),
        params_json: json!({}),
        created_at: now,
        updated_at: now,
    }
}

fn prompt_artifact() -> prompt_artifact::Model {
    let now = Utc::now().naive_utc();
    prompt_artifact::Model {
        id: 21,
        generation_spec_id: 11,
        prompt_kind: "video_prompt".to_string(),
        prompt_text: "雨夜镜头".to_string(),
        renderer_version: "v1".to_string(),
        edited_by_user: false,
        created_at: now,
        updated_at: now,
    }
}

fn media_asset(media_kind: &str, origin: &str) -> media_asset::Model {
    let now = Utc::now().naive_utc();
    media_asset::Model {
        id: 31,
        project_id: 1,
        root_project_id: 1,
        source_snapshot_id: None,
        source_chapter_id: None,
        source_snapshot_chapter_id: None,
        media_kind: media_kind.to_string(),
        origin: origin.to_string(),
        file_path: Some("media/output.bin".to_string()),
        thumbnail_path: None,
        width: None,
        height: None,
        duration_secs: None,
        fps: None,
        file_size: Some(10),
        checksum: None,
        status: "ready".to_string(),
        error_message: None,
        created_at: now,
        updated_at: now,
    }
}

fn provenance(
    imported_from_user: bool,
    source_media_asset_id: Option<i64>,
    prompt_artifact_id: Option<i64>,
) -> media_provenance::Model {
    media_provenance::Model {
        media_asset_id: 31,
        generation_spec_id: Some(11),
        prompt_artifact_id,
        provider_job_id: None,
        source_media_asset_id,
        source_external_path: None,
        imported_from_user,
        request_snapshot_json: Some(json!({"model": "test"})),
        created_at: Utc::now().naive_utc(),
    }
}

#[test]
fn prompt_and_spec_projection_preserve_missing_source_contracts() {
    let artifact = prompt_artifact();
    let missing_spec = prompt_artifact_record(artifact.clone(), None);
    assert_eq!(missing_spec.input_status, "missing");
    assert!(missing_spec.source_refs.is_empty());
    assert!(missing_spec.warnings[0].contains("generation_spec 不存在"));

    let spec = generation_spec(Some(1), None);
    let complete_artifact = prompt_artifact_record(artifact, Some(&spec));
    assert_eq!(complete_artifact.input_status, "complete");
    assert_eq!(complete_artifact.source_refs[0].id, "generation_spec:11");

    let spec_record = generation_spec_record(spec);
    assert_eq!(spec_record.label, "chapter_video #11");
    assert_eq!(spec_record.input_status, "missing");
    assert_eq!(spec_record.source_refs[0].id, "chapter:1");
    assert!(spec_record.warnings[0].contains("source_snapshot_chapter_id"));
}

#[test]
fn media_projection_preserves_origin_precedence_and_missing_warnings() {
    let artifact = prompt_artifact();
    let cases = [
        (
            media_asset("video", "generated"),
            Some(provenance(true, Some(9), Some(21))),
            Some(&artifact),
            "imported",
        ),
        (
            media_asset("image", "generated"),
            Some(provenance(false, Some(9), Some(21))),
            Some(&artifact),
            "extracted_frame",
        ),
        (
            media_asset("video", "generated"),
            Some(provenance(false, None, Some(21))),
            Some(&artifact),
            "video_generated",
        ),
        (
            media_asset("image", "generated"),
            Some(provenance(false, None, None)),
            None,
            "image_generated",
        ),
        (
            media_asset("image", "manual"),
            Some(provenance(false, None, None)),
            None,
            "manual",
        ),
    ];
    for (asset, provenance, artifact, expected_origin) in cases {
        let mut warnings = Vec::new();
        let record = media_asset_record(asset, provenance.as_ref(), artifact, &mut warnings);
        assert_eq!(record.origin, expected_origin);
        assert!(warnings.is_empty());
    }

    let mut graph_warnings = Vec::new();
    let unknown = media_asset_record(
        media_asset("image", "manual"),
        None,
        None,
        &mut graph_warnings,
    );
    assert_eq!(unknown.origin, "unknown");
    assert_eq!(unknown.input_status, "missing");
    assert!(unknown.warnings[0].contains("未找到媒体来源记录"));
    assert!(graph_warnings[0].contains("缺少 media_provenance"));

    let missing_artifact_provenance = provenance(false, None, Some(21));
    let missing_artifact = media_asset_record(
        media_asset("image", "generated"),
        Some(&missing_artifact_provenance),
        None,
        &mut Vec::new(),
    );
    assert!(missing_artifact.warnings[0].contains("prompt_artifact 不存在"));
}
