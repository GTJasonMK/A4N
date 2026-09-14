use super::*;
use a4n_image_contract::ImageGenerationRequest;
#[cfg(feature = "embedded-media-providers")]
use a4n_image_contract::ImageReference;
use a4n_llm::context::{COMPRESSION_COMPRESSED_SCENE, context_aware_system_user_request};
use a4n_llm::context::{
    CONTEXT_MANIFEST_SCHEMA_VERSION, LlmContextManifest, LlmContextManifestPack,
    LlmContextManifestPackVariant,
};
use serde_json::json;

#[test]
fn model_request_preview_counts_only_ready_steps() {
    let preview = model_request_preview(
        "workflow",
        "title",
        vec![
            status_step("ready", "ready", "llm", None, "ready", None),
            status_step("reused", "reused", "llm", None, "reused", None),
        ],
    );

    assert_eq!(preview.ready_step_count, 1);
}

#[test]
fn provider_request_step_extracts_nested_provider_prompt() {
    let step = provider_request_step(
        "seedance",
        "Seedance 请求",
        "video_generation_seedance2_omni_reference",
        Some("reference_media_video".to_string()),
        "ready",
        None,
        &serde_json::json!({
            "model_id": "seedance",
            "parameters": {
                "prompt": "嵌套 provider prompt"
            }
        }),
    )
    .expect("provider request step should render");

    assert_eq!(step.user_prompt.as_deref(), Some("嵌套 provider prompt"));
}

#[test]
fn chat_request_step_renders_context_aware_preview() {
    let request = context_aware_system_user_request(
        "系统规则",
        LlmContextManifest {
            schema_version: CONTEXT_MANIFEST_SCHEMA_VERSION,
            packs: vec![LlmContextManifestPack {
                scope_kind: "chapter".to_string(),
                pack_kind: "chapter_context".to_string(),
                source_refs_json: None,
                config_hash: "preview:test".to_string(),
                required: true,
                stable_prefix: true,
                variants: vec![LlmContextManifestPackVariant {
                    compression_level: COMPRESSION_COMPRESSED_SCENE.to_string(),
                    canonical_content_json: json!({
                        "summary": "章节上下文",
                        "characters": ["甲", "乙"]
                    }),
                }],
            }],
            dynamic_user_content: "生成当前镜头".to_string(),
            output_contract: Some("只输出 JSON".to_string()),
        },
        Some(0.2),
        Some(1024),
    )
    .expect("context-aware request");

    let step = chat_request_step_with_budget(ChatRequestStepWithBudgetInput {
        step: ChatRequestStepInput {
            key: "step".to_string(),
            title: "上下文预览".to_string(),
            prompt_name: Some("prompt".to_string()),
            state: "ready".to_string(),
            note: None,
            request,
        },
        model_context_tokens: 128_000,
        output_token_reserve: 1024,
        budget_note: Some("测试模型预算".to_string()),
    })
    .expect("preview step");

    let user_prompt = step.user_prompt.as_deref().expect("user prompt");
    assert!(user_prompt.contains("[稳定上下文]"));
    assert!(user_prompt.contains("chapter_context"));
    assert!(user_prompt.contains("[当前任务]"));
    assert!(user_prompt.contains("[输出契约]"));
    assert!(step.context_manifest.is_some());
    assert!(step.context_plan.is_some());
    assert!(step.context_pack_refs.is_some());
    assert_eq!(step.request.as_ref().unwrap()["max_tokens"], json!(1024));
    assert_eq!(
        step.context_plan.as_ref().unwrap()["output_token_reserve"],
        json!(1024)
    );
}

#[test]
fn provider_request_step_redacts_inline_base64_media() {
    let step = provider_request_step(
        "image",
        "图片请求",
        "image_generation",
        None,
        "ready",
        None,
        &serde_json::json!({
            "prompt": "画图",
            "reference_images": [
                {
                    "label": "参考图 1",
                    "data_uri": "data:image/png;base64,QUJDREVGRw=="
                }
            ]
        }),
    )
    .expect("provider request step should render");

    let request = step.request.expect("request should exist").to_string();

    assert!(!request.contains("QUJDREVGRw"));
    assert!(!request.contains("data:image/png;base64"));
    assert!(!request.contains("\"data_uri\""));
    assert!(request.contains("data_uri_omitted"));
    assert!(request.contains("预览已省略完整内联媒体数据"));
    assert!(request.contains("image/png"));
}

#[test]
#[cfg(feature = "embedded-media-providers")]
fn image_provider_request_step_uses_backend_request_body_and_redacts_media() {
    let step = image_provider_request_step(
        "image",
        "图片请求",
        None,
        "ready",
        None,
        "openai-chat-image",
        ImageGenerationRequest {
            prompt: "画一张角色设定图".into(),
            width: 1024,
            height: 1024,
            model: "chat-image".into(),
            reference_images: vec![ImageReference {
                label: "参考图 1".into(),
                data_uri: "data:image/png;base64,QUJDREVGRw==".into(),
            }],
        },
    )
    .expect("image provider preview");

    assert_eq!(step.kind, "image_generation_chat_completions");
    assert_eq!(step.state, "ready");
    assert!(
        step.user_prompt
            .as_deref()
            .unwrap()
            .contains("画一张角色设定图")
    );
    let request = step.request.expect("request should exist").to_string();
    assert!(request.contains("\"messages\""));
    assert!(!request.contains("QUJDREVGRw"));
    assert!(!request.contains("data:image/png;base64"));
    assert!(request.contains("url_omitted"));
}

#[test]
#[cfg(feature = "embedded-media-providers")]
fn image_provider_request_step_rejects_unsupported_reference_images() {
    let error = image_provider_request_step(
        "image",
        "图片请求",
        None,
        "ready",
        Some("调用方备注".to_string()),
        "openai-compatible-image",
        ImageGenerationRequest {
            prompt: "画一张角色设定图".into(),
            width: 1024,
            height: 1024,
            model: "image-model".into(),
            reference_images: vec![ImageReference {
                label: "参考图 1".into(),
                data_uri: "data:image/png;base64,QUJDREVGRw==".into(),
            }],
        },
    )
    .expect_err("unsupported provider request must fail");

    assert!(error.contains("真实请求不支持参考图输入"));
}

#[test]
#[cfg(not(feature = "embedded-media-providers"))]
fn image_provider_request_step_rejects_disabled_provider_preview() {
    let error = image_provider_request_step(
        "image",
        "图片请求",
        None,
        "ready",
        None,
        "openai-chat-image",
        ImageGenerationRequest {
            prompt: "画一张角色设定图".into(),
            width: 1024,
            height: 1024,
            model: "chat-image".into(),
            reference_images: Vec::new(),
        },
    )
    .expect_err("disabled provider preview must fail");

    assert!(error.contains("请使用 image sidecar"));
}
