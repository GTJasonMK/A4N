use super::super::*;

pub(in crate::project::family::import::tests::remap) fn generation_spec_record()
-> generation_spec::Model {
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

pub(in crate::project::family::import::tests::remap) fn generation_spec_for_source_snapshot(
    snapshot_id: i64,
) -> generation_spec::Model {
    let mut spec = generation_spec_record();
    spec.source_snapshot_id = Some(snapshot_id);
    spec
}
