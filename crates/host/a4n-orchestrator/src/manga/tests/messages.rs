use std::collections::HashMap;

use a4n_domain::manga::{MangaPageAdaptationV1, MangaPageContentV1};
use a4n_error::AppError;
use a4n_prompt::rendering::PromptRenderer;
use async_trait::async_trait;
use serde_json::Value;

use super::*;
use crate::manga::pipeline::prompt::{
    MangaLlmCallInput, manga_page_storyboard_user_message, prepare_manga_llm_call,
};
use crate::source_markers::SourceMarkerMap;
use a4n_llm_context::{
    LlmContextManifest, context_aware_system_user_request, preview_context_aware_chat_request,
};

fn render_manifest(manifest: LlmContextManifest) -> String {
    let mut request = context_aware_system_user_request("system", manifest, None, None).unwrap();
    preview_context_aware_chat_request(&mut request, 1_000_000, 8192).unwrap();
    request.messages[1].content.clone()
}

#[test]
fn manga_pages_reuse_shared_input_without_losing_page_or_output_requirements() {
    let build = |page| {
        build_manga_step_user_message(
        "manga_page_drawing_prompt", &format!("只生成第 {page} 页"),
        json!({"current_page": {"page_number": page}, "page_context": {"cast": ["林晓雨"]}, "reference_assets": ["portrait:1"]}),
        json!({"format": "json_object", "required_fields": ["panels"], "page_number": page}),
    ).unwrap()
    };
    let first = build(1);
    let second = build(2);
    assert_eq!(first.packs, second.packs);
    let rendered = render_manifest(second.clone());
    for required in ["林晓雨", "portrait:1", "panels", "只生成第 2 页"] {
        assert!(rendered.contains(required));
    }
    let payload: Value = serde_json::from_str(
        a4n_prompt::request::json_user_message_payload(&second.dynamic_user_content).unwrap(),
    )
    .unwrap();
    assert_eq!(payload["input"]["current_page"]["page_number"], 2);
    assert_eq!(payload["output_contract"]["page_number"], 2);
    assert!(!second.packs.iter().any(|pack| {
        pack.variants[0]
            .canonical_content_json
            .to_string()
            .contains("current_page")
    }));
}

#[test]
fn manga_page_storyboard_message_requires_null_speaker_for_non_dialogue_layers() {
    let beat_ref = json!({"unit_id": "unit-1", "beat_id": "beat-1"});
    let content: MangaPageContentV1 = serde_json::from_value(json!({
        "summary": "角色发现异常",
        "source_span_ids": [],
        "beat_refs": [beat_ref.clone()],
        "action": "角色停下脚步",
        "reaction": null,
        "dialogue_intent": null,
        "entry_state": "平静",
        "exit_state": "紧张"
    }))
    .expect("page content fixture");
    let adaptation: MangaPageAdaptationV1 = serde_json::from_value(json!({
        "visual_intent": "用留白放大异常感",
        "continuity_requirements": [],
        "retained_details": [],
        "omitted_details": [],
        "beat_refs": [beat_ref]
    }))
    .expect("page adaptation fixture");
    let source_markers = SourceMarkerMap::from_source_spans(&[]).expect("source markers");
    let message = manga_page_storyboard_user_message(
        1,
        1,
        &content,
        &adaptation,
        None,
        None,
        json!({"units": [{"participants": ["林晓雨"]}]}),
        &json!({"characters": {"林晓雨": {"name": "林晓雨"}}}),
        &json!({"character_portraits": []}),
        &source_markers,
    )
    .expect("storyboard message");
    let contract = message
        .packs
        .iter()
        .find_map(|pack| {
            pack.variants[0]
                .canonical_content_json
                .get("output_contract")
        })
        .expect("storyboard output contract");
    assert_eq!(
        contract["text_layer_speaker_rule"],
        "text_layers.kind 为 dialogue 或 thought 时 speaker 必须是 characters 中的稳定角色名；kind 为 narration 或 sfx 时 speaker 必须严格为 JSON null"
    );
    assert_eq!(
        contract["text_layer_placement_rule"],
        "placement 必须严格使用 top_left、top_right、upper_center、center_left、center_right、bottom_left、bottom_right、lower_center；upper_right 等别名无效"
    );
}

struct TestPromptRenderer;

#[async_trait]
impl PromptRenderer for TestPromptRenderer {
    async fn render_prompt(
        &self,
        template_name: &str,
        _variables: &HashMap<String, Value>,
        _project_id: Option<i64>,
    ) -> Result<String, AppError> {
        assert_eq!(template_name, "manga_preprocessing");
        Ok("system prompt".to_string())
    }
}

#[tokio::test]
async fn manga_llm_request_sends_dynamic_payload_once() {
    let marker = "unique-manga-payload-marker";
    let variables = HashMap::new();
    let mut prepared = prepare_manga_llm_call(MangaLlmCallInput {
        prompt_renderer: &TestPromptRenderer,
        prompt_name: "manga_preprocessing",
        project_id: 7,
        workflow_run_id: None,
        call_key: "manga:test".to_string(),
        call_order: 1,
        variables: &variables,
        user_content: build_manga_step_user_message(
            "manga_preprocessing",
            "绑定角色",
            json!({"chapter": marker}),
            json!({"format": "json_object"}),
        )
        .unwrap(),
        temperature: 0.3,
    })
    .await
    .expect("manga request should build");
    preview_context_aware_chat_request(&mut prepared.request, 1_000_000, 8192).unwrap();

    let sent = prepared
        .request
        .messages
        .iter()
        .map(|message| message.content.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(sent.matches(marker).count(), 1);
    assert!(prepared.request.context_manifest_json.is_some());
}

#[test]
fn build_manga_step_user_message_uses_json_payload() {
    let message = build_manga_step_user_message(
        "manga_preprocessing",
        "选择漫画对白与旁白",
        json!({
            "chapter": {"content": "章节正文"}
        }),
        json!({
            "format": "json_object",
            "required_fields": ["speeches"]
        }),
    )
    .expect("manga step message should build");
    let payload: Value = serde_json::from_str(
        a4n_prompt::request::json_user_message_payload(&message.dynamic_user_content).unwrap(),
    )
    .unwrap();
    assert_eq!(payload["task"]["prompt_name"], "manga_preprocessing");
    assert!(payload["input"].is_object());
    assert!(payload["output_contract"].is_object());
    assert!(render_manifest(message).contains("章节正文"));
}
