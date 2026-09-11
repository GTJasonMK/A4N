use a4n_domain::llm::llm_token_profiles;
use a4n_error::AppError;
use a4n_llm::{
    context::{ContextAwareJsonLlmRequestInput, context_aware_system_user_llm_request},
    provider::ChatRequest,
};
use a4n_prompt::request::build_json_task_user_message;
use serde_json::{Value, json};

use super::context::scene_generation_stable_context;

pub(super) const PACK_KIND_CHAPTER_SCENE_GENERATION_CONTEXT: &str =
    "chapter_scene_generation_context";

pub(super) fn chapter_scene_call_key(chapter_number: i32, scene_number: i32) -> String {
    format!("chapter_scene_generation:{chapter_number}:{scene_number}")
}

pub(super) fn build_scene_generation_manifest_user_prompt(
    scene_number: i32,
    scene_prompt: &str,
) -> Result<String, AppError> {
    let payload = json!({
        "task": {
            "name": "chapter_scene_generation",
            "instruction": "基于稳定上下文中的章节摘要、章节正文/大纲和已有场景，撰写目标场景正文",
        },
        "input": {
            "context_refs": {
                "chapter": format!("稳定上下文包 {PACK_KIND_CHAPTER_SCENE_GENERATION_CONTEXT}.chapter"),
                "available_context": format!("稳定上下文包 {PACK_KIND_CHAPTER_SCENE_GENERATION_CONTEXT}.available_context"),
            },
            "scene": {
                "scene_number": scene_number,
                "prompt": scene_prompt,
            },
        },
        "output_contract": {
            "format": "plain_text",
            "requirements": [
                "只输出场景正文",
                "不要包含标题、编号、Markdown 或解释文字",
                "正文必须承接稳定上下文中的 chapter.summary，并落实 scene.prompt",
            ],
        },
    });
    build_json_task_user_message(
        "撰写章节场景",
        "只返回 output_contract 要求的场景正文，不要输出其他内容",
        &payload,
    )
}

pub(super) struct BuildChapterSceneRequestInput<'a> {
    pub(super) system_prompt: String,
    pub(super) project_id: i64,
    pub(super) chapter_number: i32,
    pub(super) title: &'a str,
    pub(super) summary: &'a str,
    pub(super) generation_context: Option<&'a Value>,
    pub(super) scene_number: i32,
    pub(super) scene_prompt: &'a str,
}

pub(super) fn build_scene_generation_request(
    input: BuildChapterSceneRequestInput<'_>,
) -> Result<ChatRequest, AppError> {
    let user_prompt =
        build_scene_generation_manifest_user_prompt(input.scene_number, input.scene_prompt)?;
    context_aware_system_user_llm_request(
        input.system_prompt,
        ContextAwareJsonLlmRequestInput {
            project_id: input.project_id,
            workflow_run_id: None,
            workflow: "chapter_scene_generation".to_string(),
            step_key: "scene_generation".to_string(),
            scope_kind: "chapter".to_string(),
            pack_kind: PACK_KIND_CHAPTER_SCENE_GENERATION_CONTEXT.to_string(),
            source_refs_json: Some(json!({
                "chapter_number": input.chapter_number,
                "chapter_title": input.title,
                "scene_number": input.scene_number,
            })),
            config_json: json!({
                "ledger_version": "chapter_scene_generation_context_v1",
                "prompt_name": "chapter_scene_generation",
                "chapter_number": input.chapter_number,
            }),
            stable_context_json: scene_generation_stable_context(
                input.chapter_number,
                input.title,
                input.summary,
                input.generation_context,
            ),
            dynamic_user_prompt: user_prompt,
            temperature: Some(0.8),
            max_tokens_profile: llm_token_profiles::STANDARD.to_string(),
            call_key: Some(chapter_scene_call_key(
                input.chapter_number,
                input.scene_number,
            )),
        },
    )
    .map_err(AppError::InvalidArgument)
}
