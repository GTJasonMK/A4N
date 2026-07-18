use a4n_llm::context::{
    COMPRESSION_FULL_RAW, CONTEXT_MANIFEST_SCHEMA_VERSION, LlmContextManifest,
    LlmContextManifestPack, LlmContextManifestPackVariant, context_aware_system_user_request,
    replace_request_user_content_from_manifest,
};
use a4n_llm::provider::{ChatRequest, LlmTraceContext, MessageRole};
use serde_json::json;

use super::{build_full_schema_repair_request, build_schema_field_patch_request};
use crate::structured_llm::schema_repair::{
    MAX_SCHEMA_PATCH_TOKENS,
    contract::{SchemaFieldPatchOperationV1, StructuredJsonDiagnostic, StructuredSchemaReport},
};

fn context_aware_request() -> ChatRequest {
    context_aware_system_user_request(
        "stable system",
        LlmContextManifest {
            schema_version: CONTEXT_MANIFEST_SCHEMA_VERSION,
            packs: vec![LlmContextManifestPack {
                scope_kind: "chapter".into(),
                pack_kind: "chapter_source".into(),
                source_refs_json: None,
                config_hash: "config".into(),
                required: true,
                stable_prefix: true,
                variants: vec![LlmContextManifestPackVariant {
                    compression_level: COMPRESSION_FULL_RAW.into(),
                    canonical_content_json: json!({"chapter": "stable source"}),
                }],
            }],
            dynamic_user_content: "生成当前拍摄设计单元".into(),
            output_contract: Some("输出拍摄设计单元 JSON".into()),
        },
        Some(0.2),
        None,
        Some("shooting_design".into()),
    )
    .expect("context-aware request")
    .with_json_response_format()
}

#[test]
fn field_patch_request_preserves_original_cache_prefix() {
    let request = ChatRequest::system_user(
        "stable system",
        "stable source context",
        Some(0.4),
        Some(32_000),
        Some("shooting_design".into()),
    )
    .with_json_response_format()
    .with_trace_context(LlmTraceContext {
        project_id: Some(1),
        root_project_id: Some(1),
        workflow: "test".into(),
        step_key: "unit".into(),
        generation_run_id: None,
        workflow_run_id: Some(2),
        call_key: Some("unit:1".into()),
        input_fingerprint_sha256: None,
        context_plan_json: None,
        context_pack_refs_json: None,
    });
    let original_messages = request.messages.clone();
    let report = StructuredSchemaReport {
        schema_sha256: "schema".into(),
        field_patch_allowed: true,
        diagnostics: vec![StructuredJsonDiagnostic {
            diagnostic_id: "d1".into(),
            path: "/names".into(),
            operation: Some(SchemaFieldPatchOperationV1::Replace),
            keyword: "type".into(),
            message: "expected array".into(),
            expected_schema: json!({"type": "array"}),
            actual_value: json!("Alice"),
        }],
    };

    let repaired = build_schema_field_patch_request(request, "repair contract", &report)
        .expect("repair request");

    assert_eq!(repaired.messages[..2].len(), original_messages.len());
    for (actual, expected) in repaired.messages[..2].iter().zip(&original_messages) {
        assert!(matches!(
            actual.role,
            MessageRole::System | MessageRole::User
        ));
        assert_eq!(actual.content, expected.content);
    }
    assert_eq!(repaired.max_tokens, Some(MAX_SCHEMA_PATCH_TOKENS));
    assert!(!repaired.messages.last().unwrap().content.contains("shots"));
    assert_eq!(
        repaired.trace_context.unwrap().call_key.as_deref(),
        Some("unit:1:schema_patch")
    );
}

#[test]
fn field_patch_rewrites_manifest_without_adding_chat_history() {
    let report = StructuredSchemaReport {
        schema_sha256: "schema".into(),
        field_patch_allowed: true,
        diagnostics: vec![StructuredJsonDiagnostic {
            diagnostic_id: "d1".into(),
            path: "/names".into(),
            operation: Some(SchemaFieldPatchOperationV1::Replace),
            keyword: "type".into(),
            message: "expected array".into(),
            expected_schema: json!({"type": "array"}),
            actual_value: json!("Alice"),
        }],
    };

    let mut repaired =
        build_schema_field_patch_request(context_aware_request(), "字段补丁契约", &report)
            .expect("manifest field repair request");
    assert_eq!(repaired.messages.len(), 2);
    assert!(matches!(repaired.messages[0].role, MessageRole::System));
    assert!(matches!(repaired.messages[1].role, MessageRole::User));
    let manifest: LlmContextManifest = serde_json::from_value(
        repaired
            .context_manifest_json
            .clone()
            .expect("repair manifest"),
    )
    .expect("repair manifest parses");
    assert!(
        manifest
            .dynamic_user_content
            .contains("生成当前拍摄设计单元")
    );
    assert!(manifest.dynamic_user_content.contains("/names"));
    assert_eq!(manifest.output_contract.as_deref(), Some("字段补丁契约"));
    replace_request_user_content_from_manifest(&mut repaired, &manifest, &[])
        .expect("dynamic LLM manifest rewrite must remain valid");
}

#[test]
fn full_repair_rewrites_manifest_for_trailing_comma_response() {
    let invalid_response = r#"{"scene_index":6,"shots":[],}"#;
    let mut repaired = build_full_schema_repair_request(
        context_aware_request(),
        invalid_response,
        "trailing comma at line 1 column 29",
    )
    .expect("manifest full repair request");

    assert_eq!(repaired.messages.len(), 2);
    assert!(matches!(repaired.messages[0].role, MessageRole::System));
    assert!(matches!(repaired.messages[1].role, MessageRole::User));
    assert!(
        repaired
            .messages
            .iter()
            .all(|message| !matches!(message.role, MessageRole::Assistant))
    );
    let manifest: LlmContextManifest = serde_json::from_value(
        repaired
            .context_manifest_json
            .clone()
            .expect("repair manifest"),
    )
    .expect("repair manifest parses");
    assert!(
        manifest
            .dynamic_user_content
            .contains("生成当前拍摄设计单元")
    );
    assert!(manifest.dynamic_user_content.contains("scene_index"));
    assert!(manifest.dynamic_user_content.contains("trailing comma"));
    let output_contract = manifest.output_contract.as_deref().unwrap_or_default();
    assert!(output_contract.contains("输出拍摄设计单元 JSON"));
    assert!(output_contract.contains("必须重新输出完整 JSON 根对象"));
    replace_request_user_content_from_manifest(&mut repaired, &manifest, &[])
        .expect("dynamic LLM manifest rewrite must remain valid");
}
