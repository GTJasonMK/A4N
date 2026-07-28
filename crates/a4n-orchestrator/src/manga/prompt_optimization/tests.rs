use std::collections::HashMap;

use a4n_core::{error::AppError, models::chapter::chapter_manga_prompt};
use a4n_llm::provider::ResponseFormat;
use a4n_prompt::rendering::PromptRenderer;
use async_trait::async_trait;
use chrono::NaiveDateTime;
use serde_json::{Value, json};

use super::{
    checkpoint::{OptimizedMangaPromptFields, restored_manga_prompt_optimization_checkpoint},
    contracts::MangaPromptOptimizationInput,
    execution::required_manga_prompt_optimization_input,
    request::{
        build_manga_prompt_optimization_request, manga_prompt_optimization_input_fingerprint,
    },
};
use crate::manga::MANGA_PAGE_PROMPT_TYPE;

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

fn panel() -> chapter_manga_prompt::Model {
    let now = NaiveDateTime::parse_from_str("2026-01-01 00:00:00", "%Y-%m-%d %H:%M:%S")
        .expect("valid timestamp");
    chapter_manga_prompt::Model {
        id: 11,
        chapter_id: 22,
        project_id: 33,
        page_number: 1,
        panel_number: 0,
        prompt_type: MANGA_PAGE_PROMPT_TYPE.to_string(),
        image_prompt: "旧提示词".to_string(),
        dialogue_text: Some("你好".to_string()),
        characters_in_frame: Some(json!(["主角"])),
        layout: Some(json!({"style": "cinematic"})),
        generated_image_id: Some(44),
        camera_angle: Some("俯拍".to_string()),
        dialogue_position: Some("左上".to_string()),
        status: "completed".to_string(),
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn input_contract_trims_instruction_and_rejects_invalid_downstream_id() {
    let required = required_manga_prompt_optimization_input(MangaPromptOptimizationInput {
        panel: panel(),
        instruction: "  增强构图  ".to_string(),
        downstream_image_ids: vec![44],
    })
    .expect("valid input");
    assert_eq!(required.instruction, "增强构图");

    let err = required_manga_prompt_optimization_input(MangaPromptOptimizationInput {
        panel: panel(),
        instruction: "增强构图".to_string(),
        downstream_image_ids: vec![0],
    })
    .expect_err("invalid image id should fail");
    assert!(err.to_string().contains("下游图片 ID 必须大于 0"));
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

#[test]
fn checkpoint_revalidates_output_and_restores_repaired_parsed_output() {
    let checkpoint = json!({
        "output": {"image_prompt": "断点里的提示词"},
        "response_content": r#"{"image_prompt":"响应里的提示词"}"#,
        "parsed_output": {"image_prompt": "  断点里的提示词  "},
        "usage": null,
        "finish_reason": "stop",
    });
    let (output, usage, finish_reason) = restored_manga_prompt_optimization_checkpoint(
        &checkpoint,
        "optimize_manga_prompt:prompt:1",
    )
    .expect("repaired checkpoint should restore parsed_output");
    assert_eq!(output.parsed_image_prompt, "  断点里的提示词  ");
    assert_eq!(output.image_prompt, "断点里的提示词");
    assert_eq!(usage, None);
    assert_eq!(finish_reason.as_deref(), Some("stop"));
}

#[test]
fn checkpoint_rejects_mismatched_output_and_truncated_finish_reason() {
    let mismatch = json!({
        "output": {"image_prompt": "另一份提示词"},
        "response_content": "原始响应",
        "parsed_output": {"image_prompt": "断点里的提示词"},
        "usage": null,
        "finish_reason": "stop",
    });
    let err =
        restored_manga_prompt_optimization_checkpoint(&mismatch, "optimize_manga_prompt:prompt:1")
            .expect_err("mismatched checkpoint should fail");
    assert!(err.to_string().contains("必须等于 parsed_output"));

    let truncated = json!({
        "output": {"image_prompt": "断点里的提示词"},
        "response_content": "原始响应",
        "parsed_output": {"image_prompt": "断点里的提示词"},
        "usage": null,
        "finish_reason": "length",
    });
    let err =
        restored_manga_prompt_optimization_checkpoint(&truncated, "optimize_manga_prompt:prompt:1")
            .expect_err("truncated checkpoint should fail");
    assert!(err.to_string().contains("不能用于恢复"));
}

#[tokio::test]
async fn request_uses_context_manifest_json_mode_and_stable_fingerprint() {
    let panel = panel();
    let request = build_manga_prompt_optimization_request(&TestPromptRenderer, &panel, "增强构图")
        .await
        .expect("request should build");
    assert_eq!(request.response_format, Some(ResponseFormat::json_object()));
    let manifest = request
        .context_manifest_json
        .as_ref()
        .expect("context manifest should exist");
    assert!(manifest.to_string().contains("source_artifact"));
    let trace = request
        .trace_context
        .as_ref()
        .expect("trace context should exist");
    assert_eq!(trace.workflow, "manga_prompt_optimization");
    assert_eq!(
        trace.call_key.as_deref(),
        Some("optimize_manga_prompt:prompt:11")
    );

    let fingerprint = manga_prompt_optimization_input_fingerprint(
        &panel,
        "增强构图",
        json!({"request": "stable"}),
    );
    assert_eq!(fingerprint["source_refs"]["layout"], panel.layout.unwrap());
    assert_eq!(
        fingerprint["source_refs"]["characters_in_frame"],
        panel.characters_in_frame.unwrap()
    );
}
