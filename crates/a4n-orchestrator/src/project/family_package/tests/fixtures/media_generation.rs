use super::super::*;

pub(in crate::project::family_package::tests) fn media_asset_with_checksum(
    bytes: &[u8],
) -> media_asset::Model {
    media_asset::Model {
        id: 10,
        project_id: 2,
        root_project_id: 1,
        source_snapshot_id: Some(901),
        source_chapter_id: None,
        source_snapshot_chapter_id: None,
        media_kind: "video".to_string(),
        origin: "imported".to_string(),
        file_path: Some("projects/2/media/video/input.mp4".to_string()),
        thumbnail_path: None,
        width: None,
        height: None,
        duration_secs: Some(8.0),
        fps: Some(24.0),
        file_size: Some(bytes.len() as i64),
        checksum: Some(sha256_bytes(bytes)),
        status: "completed".to_string(),
        error_message: None,
        created_at: dt(),
        updated_at: dt(),
    }
}

pub(in crate::project::family_package::tests) fn empty_media_payload() -> FamilyImportMediaPayload {
    FamilyImportMediaPayload {
        entries: HashMap::new(),
        bytes: HashMap::new(),
    }
}

pub(in crate::project::family_package::tests) fn media_payload_for_assets(
    assets: &[media_asset::Model],
) -> FamilyImportMediaPayload {
    let mut payload = empty_media_payload();
    for asset in assets {
        if let Some(file_path) = asset.file_path.as_ref() {
            let package_path = format!("media/{}/file", asset.id);
            payload.entries.insert(
                media_file_key(asset.id, MEDIA_FILE_FIELD_FILE),
                MediaFileEntry {
                    media_asset_id: asset.id,
                    field: MEDIA_FILE_FIELD_FILE.to_string(),
                    original_path: file_path.clone(),
                    package_path: package_path.clone(),
                },
            );
            payload
                .bytes
                .insert(package_path, b"expected bytes".to_vec());
        }
    }
    payload
}

pub(in crate::project::family_package::tests) fn completed_media_asset_with_kind(
    media_kind: &str,
) -> media_asset::Model {
    let mut asset = media_asset_with_checksum(b"expected bytes");
    asset.media_kind = media_kind.to_string();
    asset
}

pub(in crate::project::family_package::tests) fn media_asset_with_status(
    status: &str,
) -> media_asset::Model {
    let mut asset = media_asset_with_checksum(b"expected bytes");
    asset.status = status.to_string();
    asset.file_path = None;
    asset.thumbnail_path = None;
    asset.file_size = None;
    asset.checksum = None;
    asset
}

pub(in crate::project::family_package::tests) fn media_asset_with_kind(
    media_kind: &str,
) -> media_asset::Model {
    let mut asset = media_asset_with_status(MEDIA_STATUS_PENDING);
    asset.media_kind = media_kind.to_string();
    asset
}

pub(in crate::project::family_package::tests) fn media_asset_with_origin(
    origin: &str,
) -> media_asset::Model {
    let mut asset = media_asset_with_status(MEDIA_STATUS_PENDING);
    asset.origin = origin.to_string();
    asset
}

pub(in crate::project::family_package::tests) fn media_binding_record(
    target_kind: &str,
    role: &str,
) -> media_binding::Model {
    media_binding::Model {
        id: 901,
        media_asset_id: 10,
        target_kind: target_kind.to_string(),
        target_id: 201,
        role: role.to_string(),
        created_at: dt(),
    }
}

pub(in crate::project::family_package::tests) fn media_provenance_record(
    request_snapshot_json: Option<Value>,
) -> media_provenance::Model {
    media_provenance::Model {
        media_asset_id: 10,
        generation_spec_id: None,
        prompt_artifact_id: None,
        provider_job_id: None,
        source_media_asset_id: None,
        source_external_path: None,
        imported_from_user: false,
        request_snapshot_json,
        created_at: dt(),
    }
}

pub(in crate::project::family_package::tests) fn generation_spec_record() -> generation_spec::Model
{
    generation_spec::Model {
        id: 301,
        project_id: 1,
        root_project_id: 1,
        source_context_kind: Some("snapshot_backed".to_string()),
        source_snapshot_id: Some(401),
        source_chapter_id: None,
        source_snapshot_chapter_id: None,
        canvas_text_asset_id: None,
        target_kind: "video_segment".to_string(),
        target_entity_kind: "segment".to_string(),
        target_entity_id: None,
        status: "draft".to_string(),
        source_version_hash: "hash".to_string(),
        params_json: json!({}),
        created_at: dt(),
        updated_at: dt(),
    }
}

pub(in crate::project::family_package::tests) fn generation_spec_for_source_snapshot(
    snapshot_id: i64,
) -> generation_spec::Model {
    let mut spec = generation_spec_record();
    spec.source_snapshot_id = Some(snapshot_id);
    spec
}

pub(in crate::project::family_package::tests) fn generation_text_anchor_ref(
    role: &str,
) -> generation_spec_text_anchor::Model {
    generation_spec_text_anchor::Model {
        generation_spec_id: 301,
        text_anchor_id: 501,
        role: role.to_string(),
    }
}

pub(in crate::project::family_package::tests) fn generation_entity_ref(
    entity_kind: &str,
    role: &str,
) -> generation_spec_entity_ref::Model {
    generation_spec_entity_ref::Model {
        generation_spec_id: 301,
        entity_kind: entity_kind.to_string(),
        entity_id: 601,
        role: role.to_string(),
    }
}

pub(in crate::project::family_package::tests) fn generation_media_ref(
    role: &str,
) -> generation_spec_media_ref::Model {
    generation_spec_media_ref::Model {
        generation_spec_id: 301,
        media_asset_id: 10,
        role: role.to_string(),
    }
}

pub(in crate::project::family_package::tests) fn package_with_valid_media_binding_target(
    target_kind: &str,
) -> ProjectFamilyPackage {
    let mut package = match target_kind {
        "galgame_scene" | "galgame_line" => package_with_galgame_project(),
        _ => package_with_derived_project(),
    };
    match target_kind {
        "character_profile" => {
            package.text_core.character_profiles = vec![character_profile_record(201)];
        }
        "object_profile" => {
            package.text_core.object_profiles = vec![object_profile_record(201)];
        }
        "scene_profile" => {
            package.text_core.scene_profiles = vec![scene_profile_record(201)];
        }
        "video_segment" => {
            package.video_segments = vec![video_segment_record(201)];
        }
        "chapter" => {
            package.text_core.chapters = vec![chapter_record(201, 1, None)];
        }
        "assembly_timeline" => {
            package.assembly_timelines = vec![assembly_timeline_record(201)];
        }
        "galgame_scene" => {
            package.galgame_scripts = vec![galgame_script_record()];
            package.galgame_scenes = vec![galgame_scene_record_with_id(201, 1001, "main", None)];
        }
        "galgame_line" => {
            package.galgame_scripts = vec![galgame_script_record()];
            package.galgame_scenes = vec![galgame_scene_record_with_id(2201, 1001, "main", None)];
            package.galgame_lines = vec![galgame_line_record_with_id(201, 2201)];
        }
        _ => {}
    }
    package
}

pub(in crate::project::family_package::tests) fn video_segment_record(
    id: i64,
) -> video_segment::Model {
    video_segment::Model {
        id,
        project_id: 2,
        root_project_id: 1,
        source_chapter_id: None,
        source_snapshot_chapter_id: None,
        segment_type: "shot".to_string(),
        order_index: 1,
        title: "镜头".to_string(),
        duration_secs: 6.0,
        source_generation_spec_id: None,
        result_media_asset_id: None,
        previous_segment_id: None,
        next_segment_id: None,
        created_at: dt(),
        updated_at: dt(),
    }
}

pub(in crate::project::family_package::tests) fn assembly_timeline_record(
    id: i64,
) -> assembly_timeline::Model {
    assembly_timeline::Model {
        id,
        project_id: 2,
        source_chapter_id: None,
        source_snapshot_chapter_id: None,
        version: 1,
        status: "draft".to_string(),
        preview_media_asset_id: None,
        preview_cache_key: None,
        created_at: dt(),
        updated_at: dt(),
    }
}

pub(in crate::project::family_package::tests) fn prompt_artifact_record(
    prompt_kind: &str,
) -> prompt_artifact::Model {
    prompt_artifact::Model {
        id: 701,
        generation_spec_id: 301,
        prompt_kind: prompt_kind.to_string(),
        prompt_text: "prompt text".to_string(),
        renderer_version: "test".to_string(),
        edited_by_user: false,
        created_at: dt(),
        updated_at: dt(),
    }
}
