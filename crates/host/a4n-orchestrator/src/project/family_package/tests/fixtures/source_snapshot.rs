use super::super::*;
use a4n_db::text_anchor::{EVIDENCE_ROLE_FIRST_SEEN, SOURCE_KIND_CHAPTER_CONTENT};

pub(in crate::project::family_package::tests) fn source_snapshot_record()
-> project_source_snapshot::Model {
    project_source_snapshot::Model {
        id: 901,
        root_project_id: 1,
        source_version_hash: "hash".to_string(),
        blueprint_version_id: None,
        chapter_scope_json: json!({"start": 1, "end": 10}),
        chapter_hashes_json: json!([]),
        profile_hashes_json: json!([]),
        created_at: dt(),
    }
}

pub(in crate::project::family_package::tests) fn source_snapshot_chapter_record(
    chapter_number: i32,
) -> source_snapshot_chapter::Model {
    let content = format!("第{chapter_number}章冻结正文");
    source_snapshot_chapter::Model {
        id: 1000 + chapter_number as i64,
        snapshot_id: 901,
        root_project_id: 1,
        source_chapter_id: 2000 + chapter_number as i64,
        chapter_number,
        title: Some(format!("第{chapter_number}章")),
        content_hash: hash_text(&content),
        content,
        summary: None,
        analysis_data_json: None,
        analysis_hash: None,
    }
}

pub(in crate::project::family_package::tests) fn source_snapshot_chapter_hash_row(
    chapter: &source_snapshot_chapter::Model,
) -> Value {
    json!({
        "source_chapter_id": chapter.source_chapter_id,
        "chapter_number": chapter.chapter_number,
        "content_hash": chapter.content_hash.clone(),
        "analysis_hash": chapter.analysis_hash.clone(),
    })
}

pub(in crate::project::family_package::tests) fn source_snapshot_outline_record(
    outline_kind: &str,
) -> source_snapshot_outline::Model {
    let payload = json!({
        "id": 301,
        "project_id": 1,
        "outline_kind": outline_kind,
        "title": "冻结大纲"
    });
    source_snapshot_outline::Model {
        id: 1201,
        snapshot_id: 901,
        root_project_id: 1,
        source_outline_id: 301,
        outline_kind: outline_kind.to_string(),
        payload_hash: hash_json(&payload),
        payload_json: payload,
    }
}

pub(in crate::project::family_package::tests) fn source_snapshot_profile_record(
    profile_kind: &str,
) -> source_snapshot_profile::Model {
    let payload = json!({
        "id": 401,
        "project_id": 1,
        "profile_kind": profile_kind,
        "name": "冻结档案"
    });
    source_snapshot_profile::Model {
        id: 1301,
        snapshot_id: 901,
        root_project_id: 1,
        profile_kind: profile_kind.to_string(),
        source_profile_id: 401,
        payload_hash: hash_json(&payload),
        payload_json: payload,
    }
}

pub(in crate::project::family_package::tests) fn source_snapshot_profile_hash_row(
    profile: &source_snapshot_profile::Model,
) -> Value {
    json!({
        "kind": profile.profile_kind.clone(),
        "id": profile.source_profile_id,
        "hash": profile.payload_hash.clone(),
    })
}

pub(in crate::project::family_package::tests) fn snapshot_text_anchor_record(
    snapshot_chapter: &source_snapshot_chapter::Model,
) -> text_anchor::Model {
    text_anchor::Model {
        id: 702,
        project_id: 1,
        source_chapter_id: Some(snapshot_chapter.source_chapter_id),
        source_snapshot_id: Some(snapshot_chapter.snapshot_id),
        source_snapshot_chapter_id: Some(snapshot_chapter.id),
        source_kind: SOURCE_KIND_CHAPTER_CONTENT.to_string(),
        source_id: snapshot_chapter.id,
        paragraph_index: 0,
        start_offset: 0,
        end_offset: snapshot_chapter.content.chars().count() as i32,
        quote: snapshot_chapter.content.clone(),
        quote_hash: hash_text(&snapshot_chapter.content),
        source_version_hash: snapshot_chapter.content_hash.clone(),
        created_at: dt(),
    }
}

pub(in crate::project::family_package::tests) fn profile_evidence_record() -> profile_evidence::Model
{
    profile_evidence::Model {
        id: 801,
        profile_kind: "character".to_string(),
        profile_id: 601,
        text_anchor_id: 701,
        evidence_role: EVIDENCE_ROLE_FIRST_SEEN.to_string(),
        created_at: dt(),
    }
}
