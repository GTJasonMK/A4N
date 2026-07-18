use super::super::super::*;
use serde_json::json;

#[test]
fn context_pack_draft_uses_stable_canonical_text_and_hash() {
    let first = LlmContextPackDraft::from_json(LlmContextPackDraftInput {
        project_id: 1,
        root_project_id: 1,
        scope_kind: "chapter".to_string(),
        workflow: "video".to_string(),
        pack_kind: "source".to_string(),
        source_refs_json: None,
        config_hash: "cfg".to_string(),
        compression_level: COMPRESSION_FULL_RAW.to_string(),
        canonical_content_json: json!({"b": 2, "a": 1}),
    })
    .expect("first pack");
    let second = LlmContextPackDraft::from_json(LlmContextPackDraftInput {
        project_id: 1,
        root_project_id: 1,
        scope_kind: "chapter".to_string(),
        workflow: "video".to_string(),
        pack_kind: "source".to_string(),
        source_refs_json: None,
        config_hash: "cfg".to_string(),
        compression_level: COMPRESSION_FULL_RAW.to_string(),
        canonical_content_json: json!({"a": 1, "b": 2}),
    })
    .expect("second pack");

    assert_eq!(first.canonical_content_text, second.canonical_content_text);
    assert_eq!(first.content_sha256, second.content_sha256);
}
