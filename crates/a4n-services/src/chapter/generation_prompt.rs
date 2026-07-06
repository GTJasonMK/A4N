use a4n_core::config::app_config::llm_token_profiles;
use a4n_core::error::AppError;
use a4n_engine::llm::provider::{ChatRequest, LlmTraceContext};
use a4n_engine::prompt::request::build_json_task_user_message;
#[cfg(test)]
use a4n_engine::prompt::request::json_user_message_payload;
#[cfg(test)]
use serde_json::Value;
use serde_json::json;

use crate::llm_context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
    LlmContextLedgerPackDraft, LlmContextManifest, context_aware_system_user_request,
};
use crate::source::source_content_sha256;

pub fn build_chapter_generation_request(
    project_id: i64,
    chapter_number: i32,
    system_prompt: String,
    chapter_outline: &str,
    context_before: Option<&str>,
) -> Result<ChatRequest, AppError> {
    let user_content = build_chapter_generation_user_message(chapter_outline, context_before)?;
    let manifest =
        build_chapter_generation_context_manifest(chapter_outline, context_before, user_content)?;
    Ok(context_aware_system_user_request(
        system_prompt,
        manifest,
        Some(0.8),
        None,
        Some(llm_token_profiles::CHAPTER_GENERATION.into()),
    )?
    .with_trace_context(LlmTraceContext {
        project_id: Some(project_id),
        root_project_id: None,
        workflow: "chapter_generation".to_string(),
        step_key: format!("chapter_{chapter_number}"),
        generation_run_id: None,
        workflow_run_id: None,
        call_key: Some(format!("chapter_{chapter_number}")),
        input_fingerprint_sha256: Some(source_content_sha256(&serde_json::to_string(&json!({
            "chapter_number": chapter_number,
            "chapter_outline": required_generation_outline_text(chapter_outline)?,
            "context_before": normalize_context_before_section(context_before),
        }))?)),
        context_plan_json: None,
        context_pack_refs_json: None,
    }))
}

fn build_chapter_generation_user_message(
    chapter_outline: &str,
    context_before: Option<&str>,
) -> Result<String, AppError> {
    required_generation_outline_text(chapter_outline)?;
    let payload = json!({
        "task": {
            "name": "chapter_generation",
            "instruction": "根据章节大纲、前文场景状态和参考上下文生成完整章节正文"
        },
        "context_refs": {
            "chapter_outline": "稳定上下文包 writing_chapter_outline",
            "context_before": if context_before.map(str::trim).filter(|text| !text.is_empty()).is_some() {
                json!("稳定上下文包 writing_context_before")
            } else {
                serde_json::Value::Null
            }
        },
        "output_contract": {
            "format": "plain_text",
            "content": "完整章节正文，必须满足 chapter_outline 中的正文字数要求；纯文本，包含自然段落换行，不含 Markdown/HTML/标题标记",
            "start_rule": "直接从正文第一自然段开始，不要先输出任何标签、对象包装、字段名、说明或摘要",
            "wrapper_rule": "不要输出结构化对象、数组、键值对或代码块；章节元数据由后端维护"
        }
    });
    build_json_task_user_message(
        "生成章节正文",
        "只返回章节正文纯文本，不要输出 JSON、Markdown、代码块、解释文字或额外字段",
        &payload,
    )
}

fn build_chapter_generation_context_manifest(
    chapter_outline: &str,
    context_before: Option<&str>,
    dynamic_user_content: String,
) -> Result<LlmContextManifest, AppError> {
    let outline = required_generation_outline_text(chapter_outline)?;
    let mut ledger = LlmContextLedger::new();
    let outline_hash = source_content_sha256(outline);
    ledger.add_pack(crate::llm_context::LlmContextLedgerPackInput {
        scope_kind: ("chapter").into(),
        pack_kind: ("writing_chapter_outline").into(),
        source_refs_json: Some(json!({
            "source": "chapter_outline",
            "content_sha256": outline_hash,
        })),
        config_identity_json: json!({
            "ledger_version": "chapter_writing_context_v1",
            "kind": "chapter_outline",
        }),
        required: true,
        stable_prefix: true,
        variants: vec![
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                json!({
                    "kind": "chapter_outline",
                    "chapter_outline": outline,
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_FOCUSED_RAW,
                json!({
                    "kind": "chapter_outline",
                    "chapter_outline": outline,
                }),
            ),
            LlmContextLedgerPackDraft::new(
                COMPRESSION_MINIMAL,
                json!({
                    "kind": "chapter_outline",
                    "content_sha256": outline_hash,
                    "char_count": outline.chars().count(),
                }),
            ),
        ],
    })?;

    if let Some(context_before) = normalize_context_before_section(context_before) {
        let context_hash = source_content_sha256(&context_before);
        ledger.add_pack(crate::llm_context::LlmContextLedgerPackInput {
            scope_kind: ("chapter").into(),
            pack_kind: ("writing_context_before").into(),
            source_refs_json: Some(json!({
                "source": "chapter_generation_context_before",
                "content_sha256": context_hash,
            })),
            config_identity_json: json!({
                "ledger_version": "chapter_writing_context_v1",
                "kind": "context_before",
            }),
            required: true,
            stable_prefix: true,
            variants: vec![
                LlmContextLedgerPackDraft::new(
                    COMPRESSION_FULL_RAW,
                    json!({
                        "kind": "context_before",
                        "context_before": context_before,
                    }),
                ),
                LlmContextLedgerPackDraft::new(
                    COMPRESSION_FOCUSED_RAW,
                    json!({
                        "kind": "context_before",
                        "context_before": context_before,
                    }),
                ),
                LlmContextLedgerPackDraft::new(
                    COMPRESSION_MINIMAL,
                    json!({
                        "kind": "context_before",
                        "content_sha256": context_hash,
                        "char_count": context_before.chars().count(),
                    }),
                ),
            ],
        })?;
    }

    Ok(ledger.into_manifest(
        dynamic_user_content,
        Some(
            "章节生成必须只返回章节正文纯文本，不要输出 JSON、Markdown、代码块、解释文字或标题标记。"
                .to_string(),
        ),
    ))
}

fn required_generation_outline_text(chapter_outline: &str) -> Result<&str, AppError> {
    let outline = chapter_outline.trim();
    if outline.is_empty() {
        return Err(AppError::InvalidArgument(
            "章节生成缺少章节大纲，未调用 LLM".to_string(),
        ));
    }
    Ok(outline)
}

fn normalize_context_before_section(context_before: Option<&str>) -> Option<String> {
    match context_before
        .map(str::trim)
        .filter(|text| !text.is_empty())
    {
        Some(text) if text.starts_with('#') => Some(text.to_string()),
        Some(text) => Some(format!("## 场景状态\n{text}")),
        None => None,
    }
}

#[cfg(test)]
fn parse_chapter_generation_user_payload(content: &str) -> Result<Value, AppError> {
    let payload = json_user_message_payload(content).ok_or_else(|| {
        AppError::InvalidArgument("章节生成 user message 缺少结构化 JSON 输入前缀".to_string())
    })?;
    serde_json::from_str(payload).map_err(|e| {
        AppError::InvalidArgument(format!("章节生成 user message 不是合法 JSON payload: {e}"))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builds_generation_user_message_with_scene_state_for_plain_context() {
        let message =
            build_chapter_generation_user_message("## 当前任务\n第2章", Some("上一章结尾"))
                .expect("valid generation outline");
        let payload = parse_chapter_generation_user_payload(&message).expect("payload");

        assert_eq!(payload["task"]["name"], json!("chapter_generation"));
        assert_eq!(
            payload["context_refs"]["chapter_outline"],
            json!("稳定上下文包 writing_chapter_outline")
        );
        assert_eq!(
            payload["context_refs"]["context_before"],
            json!("稳定上下文包 writing_context_before")
        );
        assert_eq!(payload["output_contract"]["format"], json!("plain_text"));
        assert!(payload["output_contract"].get("forbidden_fields").is_none());
        assert!(!message.contains("full_content"));
        assert!(!message.contains("上一章结尾"));
    }

    #[test]
    fn preserves_structured_context_sections_in_generation_user_message() {
        let message = build_chapter_generation_user_message(
            "## 当前任务\n第3章",
            Some("## 前文摘要\n第1章：开端\n\n## 上一章结尾\n尾段"),
        )
        .expect("valid generation outline");
        let payload = parse_chapter_generation_user_payload(&message).expect("payload");

        assert_eq!(
            payload["context_refs"]["chapter_outline"],
            json!("稳定上下文包 writing_chapter_outline")
        );
        assert_eq!(
            payload["context_refs"]["context_before"],
            json!("稳定上下文包 writing_context_before")
        );
        assert!(!message.contains("第1章：开端"));
        assert!(!message.contains("上一章结尾"));
    }

    #[test]
    fn generation_user_message_omits_missing_context_section() {
        let message = build_chapter_generation_user_message("## 当前任务\n第1章", None)
            .expect("valid generation outline");
        let payload = parse_chapter_generation_user_payload(&message).expect("payload");

        assert_eq!(
            payload["context_refs"]["chapter_outline"],
            json!("稳定上下文包 writing_chapter_outline")
        );
        assert!(payload["context_refs"]["context_before"].is_null());
    }

    #[test]
    fn generation_user_message_rejects_missing_outline() {
        let err = build_chapter_generation_user_message("  ", Some("上一章结尾"))
            .expect_err("blank generation outline should fail");

        assert!(matches!(err, AppError::InvalidArgument(_)));
        assert!(err.to_string().contains("缺少章节大纲"));
    }

    #[test]
    fn chapter_generation_request_moves_outline_and_context_into_manifest() {
        let request = build_chapter_generation_request(
            42,
            3,
            "写作系统提示词".to_string(),
            "## 当前任务\n第3章：追踪线索",
            Some("上一章结尾：主角发现旧信封"),
        )
        .expect("chapter generation request should build");

        let manifest = request
            .context_manifest_json
            .as_ref()
            .expect("context manifest should be attached");
        let pack_kinds = manifest["packs"]
            .as_array()
            .expect("packs should be array")
            .iter()
            .filter_map(|pack| pack["pack_kind"].as_str())
            .collect::<Vec<_>>();
        assert!(pack_kinds.contains(&"writing_chapter_outline"));
        assert!(pack_kinds.contains(&"writing_context_before"));

        let user_message = request
            .messages
            .iter()
            .find(|message| matches!(message.role, a4n_engine::llm::provider::MessageRole::User))
            .expect("user message should exist");
        assert!(user_message.content.contains("context_refs"));
        assert!(user_message.content.contains("writing_chapter_outline"));
        assert!(user_message.content.contains("writing_context_before"));
        assert!(!user_message.content.contains("追踪线索"));
        assert!(!user_message.content.contains("旧信封"));

        let trace_context = request
            .trace_context
            .as_ref()
            .expect("trace context should be attached");
        assert_eq!(trace_context.workflow, "chapter_generation");
        assert_eq!(trace_context.step_key, "chapter_3");
        assert_eq!(trace_context.call_key.as_deref(), Some("chapter_3"));
    }
}
