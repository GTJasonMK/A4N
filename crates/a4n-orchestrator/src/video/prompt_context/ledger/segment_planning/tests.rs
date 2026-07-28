use a4n_llm::context::{
    CONTEXT_MANIFEST_SCHEMA_VERSION, LlmContextManifest, LlmContextManifestPack,
    LlmContextManifestPackVariant,
};
use serde_json::{Value, json};

use super::VideoSegmentPlanningContextTemplate;

fn pack(
    pack_kind: &str,
    source_refs_json: Option<Value>,
    content: Value,
) -> LlmContextManifestPack {
    LlmContextManifestPack {
        scope_kind: "chapter".to_string(),
        pack_kind: pack_kind.to_string(),
        source_refs_json,
        config_hash: format!("hash:{pack_kind}"),
        required: true,
        stable_prefix: true,
        variants: vec![LlmContextManifestPackVariant {
            compression_level: "full_raw".to_string(),
            canonical_content_json: content,
        }],
    }
}

#[test]
fn checkpoint_identity_preserves_existing_v2_semantics() {
    let chapter = json!({"summary": "章节摘要"});
    let project_context = json!({"characters": ["林清辞"]});
    let prompt_rule_hashes = json!({"prompts": [{"prompt_name": "chapter_video_segment_plan"}]});
    let template = VideoSegmentPlanningContextTemplate::new(LlmContextManifest {
        schema_version: CONTEXT_MANIFEST_SCHEMA_VERSION,
        packs: vec![
            pack("video_segment_planning_chapter", None, chapter.clone()),
            pack(
                "video_previous_output:shooting_script_projection",
                Some(json!({"continuity_sha256": "continuity-hash"})),
                json!({"scenes": []}),
            ),
            pack("video_prompt_rule_hashes", None, prompt_rule_hashes.clone()),
            pack("video_project_context", None, project_context.clone()),
        ],
        dynamic_user_content: String::new(),
        output_contract: None,
    });

    assert_eq!(
        template
            .checkpoint_identity_json()
            .expect("checkpoint identity should build"),
        json!({
            "chapter": chapter,
            "continuity_map_sha256": "continuity-hash",
            "project_context": project_context,
            "prompt_rule_hashes": prompt_rule_hashes
        })
    );
}

#[test]
fn checkpoint_identity_requires_continuity_pack() {
    let template = VideoSegmentPlanningContextTemplate::new(LlmContextManifest {
        schema_version: CONTEXT_MANIFEST_SCHEMA_VERSION,
        packs: vec![
            pack(
                "video_segment_planning_chapter",
                None,
                json!({"summary": "章节摘要"}),
            ),
            pack("video_prompt_rule_hashes", None, json!({"prompts": []})),
        ],
        dynamic_user_content: String::new(),
        output_contract: None,
    });

    assert!(template.checkpoint_identity_json().is_err());
}
