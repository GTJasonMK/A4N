use std::collections::HashMap;

use a4n_db_models::models::chapter::chapter_manga_prompt;
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ResponseFormat;
use a4n_prompt::rendering::PromptRenderer;
use async_trait::async_trait;
use chrono::NaiveDateTime;
use serde_json::{Value, json};

use super::{
    contracts::MangaPromptOptimizationInput,
    execution::{
        OptimizedMangaPromptFields, manga_prompt_optimization_llm_error,
        required_manga_prompt_optimization_input,
    },
    request::build_manga_prompt_optimization_request,
};
use a4n_db_models::models::chapter::chapter_manga_prompt::MANGA_PAGE_PROMPT_TYPE;

struct TestPromptRenderer;

#[async_trait]
impl PromptRenderer for TestPromptRenderer {
    async fn render_prompt(
        &self,
        template_name: &str,
        variables: &HashMap<String, Value>,
        _project_id: Option<i64>,
    ) -> Result<String, AppError> {
        Ok(match template_name {
            "manga_prompt_optimization" => "system prompt".to_string(),
            "manga_prompt_optimization_user" => variables
                .get("payload")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string(),
            other => panic!("unexpected prompt: {other}"),
        })
    }
}

fn page_prompt() -> chapter_manga_prompt::Model {
    let now = NaiveDateTime::parse_from_str("2026-01-01 00:00:00", "%Y-%m-%d %H:%M:%S")
        .expect("valid timestamp");
    chapter_manga_prompt::Model {
        id: 11,
        chapter_id: 22,
        project_id: 33,
        page_number: 1,
        prompt_type: MANGA_PAGE_PROMPT_TYPE.to_string(),
        image_prompt: "旧提示词".to_string(),
        dialogue_text: Some("你好".to_string()),
        characters_in_frame: Some(json!(["主角"])),
        layout: Some(json!({"style": "cinematic"})),
        generated_image_id: Some(44),
        status: "completed".to_string(),
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn input_contract_trims_instruction_and_rejects_invalid_downstream_id() {
    let required = required_manga_prompt_optimization_input(MangaPromptOptimizationInput {
        page_prompt: page_prompt(),
        instruction: "  增强构图  ".to_string(),
        downstream_image_ids: vec![44],
    })
    .expect("valid input");
    assert_eq!(required.instruction, "增强构图");

    let err = required_manga_prompt_optimization_input(MangaPromptOptimizationInput {
        page_prompt: page_prompt(),
        instruction: "增强构图".to_string(),
        downstream_image_ids: vec![0],
    })
    .expect_err("invalid image id should fail");
    assert!(err.to_string().contains("下游图片 ID 必须大于 0"));
}

#[test]
fn prompt_optimization_llm_error_preserves_cancelled() {
    assert!(matches!(
        manga_prompt_optimization_llm_error(AppError::Cancelled),
        AppError::Cancelled
    ));
}

#[test]
fn output_rejects_unknown_fields() {
    let err = serde_json::from_value::<OptimizedMangaPromptFields>(json!({
        "image_prompt": "优化后的漫画整页提示词",
        "legacy_notes": "旧字段",
    }))
    .expect_err("unknown optimized fields should fail");
    assert!(err.to_string().contains("unknown field"));
}

#[tokio::test]
async fn request_sends_one_payload_without_context_manifest() {
    let page_prompt = page_prompt();
    let request =
        build_manga_prompt_optimization_request(&TestPromptRenderer, &page_prompt, "增强构图")
            .await
            .expect("request should build");
    assert_eq!(request.response_format, Some(ResponseFormat::json_object()));
    assert!(request.context_manifest_json.is_none());
    let payload: Value =
        serde_json::from_str(&request.messages[1].content).expect("payload should be JSON");
    assert_eq!(payload["current_prompt"]["image_prompt"], "旧提示词");
    assert_eq!(payload["current_prompt"]["dialogue_text"], "你好");
    assert_eq!(payload["instruction"], "增强构图");
    let trace = request
        .trace_context
        .as_ref()
        .expect("trace context should exist");
    assert_eq!(trace.workflow, "manga_prompt_optimization");
    assert_eq!(
        trace.call_key.as_deref(),
        Some("optimize_manga_prompt:prompt:11")
    );
    assert_eq!(
        trace.input_fingerprint_sha256.as_deref().map(str::len),
        Some(64)
    );
}
