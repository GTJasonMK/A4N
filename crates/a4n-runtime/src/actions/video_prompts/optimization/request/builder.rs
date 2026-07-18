use super::{
    super::contracts::{
        CHAPTER_VIDEO_PROMPT_OPTIMIZATION_PROMPT_NAME,
        CHAPTER_VIDEO_PROMPT_OPTIMIZATION_USER_PROMPT_NAME,
    },
    payload::video_prompt_optimization_source_artifact,
};
use a4n_core::{
    models::chapter::chapter_video_prompt, source_spans::source_text_hash as source_content_sha256,
};
use a4n_llm::context::{
    COMPRESSION_FOCUSED_RAW, COMPRESSION_FULL_RAW, COMPRESSION_MINIMAL, LlmContextLedger,
    LlmContextLedgerPackDraft, context_aware_system_user_request,
};
use a4n_llm::provider::{ChatRequest, LlmTraceContext};
use a4n_orchestrator::video::PromptOutputLanguage;
use serde_json::json;

pub(super) fn build_video_prompt_optimization_request_with_prompts(
    system_prompt: String,
    user_prompt: String,
    prompt: &chapter_video_prompt::Model,
    instruction: &str,
    prompt_output_language: PromptOutputLanguage,
) -> Result<ChatRequest, String> {
    let current_prompt = video_prompt_optimization_source_artifact(prompt);
    let source_hash = source_content_sha256(
        &serde_json::to_string(&current_prompt)
            .map_err(|err| format!("视频提示词优化 source artifact 序列化失败：{err}"))?,
    );
    let mut ledger = LlmContextLedger::new();
    ledger
        .add_pack(a4n_llm::context::LlmContextLedgerPackInput {
            scope_kind: ("prompt_optimization").into(),
            pack_kind: ("source_artifact").into(),
            source_refs_json: Some(json!({
                "source": "chapter_video_prompt",
                "prompt_id": prompt.id,
                "content_sha256": source_hash})),
            config_identity_json: json!({
                "ledger_version": "video_prompt_optimization_context_v1",
                "artifact_kind": "chapter_video_prompt"}),
            required: true,
            stable_prefix: true,
            variants: vec![
                LlmContextLedgerPackDraft::new(COMPRESSION_FULL_RAW, current_prompt.clone()),
                LlmContextLedgerPackDraft::new(COMPRESSION_FOCUSED_RAW, current_prompt.clone()),
                LlmContextLedgerPackDraft::new(
                    COMPRESSION_MINIMAL,
                    json!({
                        "artifact_kind": "chapter_video_prompt",
                        "prompt_id": prompt.id,
                        "prompt_type": prompt.prompt_type.as_str(),
                        "workflow_mode": prompt.workflow_mode.as_str(),
                        "shot_number": prompt.shot_number,
                        "title": prompt.title.as_deref(),
                        "visual_prompt": &prompt.visual_prompt,
                        "motion_prompt": prompt.motion_prompt.as_deref(),
                        "negative_prompt": prompt.negative_prompt.as_deref()}),
                ),
            ],
        })
        .map_err(|err| err.to_string())?;
    let manifest = ledger.into_manifest(
        user_prompt,
        Some("视频提示词优化必须保留稳定上下文包 source_artifact 中的事实、结构和素材约束，只按动态输入中的 instruction 修改表达。".to_string()),
    );
    let fingerprint = source_content_sha256(
        &serde_json::to_string(&json!({
        "prompt_id": prompt.id,
        "source_artifact_sha256": source_hash,
        "instruction": instruction,
        "prompt_output_language": prompt_output_language.as_str(),
        "prompt_names": [
            CHAPTER_VIDEO_PROMPT_OPTIMIZATION_PROMPT_NAME,
            CHAPTER_VIDEO_PROMPT_OPTIMIZATION_USER_PROMPT_NAME,
        ]}))
        .map_err(|err| format!("视频提示词优化 fingerprint 序列化失败：{err}"))?,
    );
    let request = context_aware_system_user_request(
        system_prompt,
        manifest,
        Some(0.25),
        None,
        Some("media_prompt".to_string()),
    )
    .map_err(|err| err.to_string())?
    .with_json_response_format()
    .with_trace_context(LlmTraceContext {
        project_id: Some(prompt.project_id),
        root_project_id: None,
        workflow: "video_prompt_optimization".to_string(),
        step_key: "optimize_chapter_video_prompt".to_string(),
        generation_run_id: None,
        workflow_run_id: None,
        call_key: Some(format!(
            "optimize_chapter_video_prompt:prompt:{}",
            prompt.id
        )),
        input_fingerprint_sha256: Some(fingerprint),
        context_plan_json: None,
        context_pack_refs_json: None,
    });
    Ok(request)
}
