use super::super::access_errors::{ensure_scene_number, required_scene_prompt_text};
use crate::actions::llm_request::PACK_KIND_CHAPTER_SCENE_GENERATION_CONTEXT;
use a4n_core::error::AppError;
use a4n_prompt::request::build_json_task_user_message;
#[cfg(test)]
use serde_json::Value;
use serde_json::json;

#[cfg(test)]
pub(in super::super) fn build_scene_generation_user_prompt(
    chapter_number: i32,
    title: &str,
    summary: &str,
    generation_context: Option<&Value>,
    scene_number: i32,
    scene_prompt: &str,
) -> Result<String, AppError> {
    ensure_scene_number(scene_number)?;
    let scene_prompt = required_scene_prompt_text(scene_prompt, "场景引导")?;
    let payload = json!({
    "chapter": {
        "chapter_number": chapter_number,
        "title": title,
        "summary": summary,
        "available_context": generation_context.cloned().unwrap_or_else(|| json!({}))},
    "scene": {
        "scene_number": scene_number,
        "prompt": scene_prompt},
    "output_contract": {
        "format": "plain_text",
        "requirements": [
            "只输出场景正文",
            "不要包含标题、编号、Markdown 或解释文字",
            "正文必须承接 chapter.summary，并落实 scene.prompt"
        ]}});
    build_json_task_user_message(
        "撰写章节场景",
        "只返回 output_contract 要求的场景正文，不要输出其他内容",
        &payload,
    )
}

pub(super) fn build_scene_generation_manifest_user_prompt(
    scene_number: i32,
    scene_prompt: &str,
) -> Result<String, AppError> {
    ensure_scene_number(scene_number)?;
    let scene_prompt = required_scene_prompt_text(scene_prompt, "场景引导")?;
    let payload = json!({
    "task": {
        "name": "chapter_scene_generation",
        "instruction": "基于稳定上下文中的章节摘要、章节正文/大纲和已有场景，撰写目标场景正文"
    },
    "input": {
        "context_refs": {
            "chapter": format!("稳定上下文包 {PACK_KIND_CHAPTER_SCENE_GENERATION_CONTEXT}.chapter"),
            "available_context": format!("稳定上下文包 {PACK_KIND_CHAPTER_SCENE_GENERATION_CONTEXT}.available_context")
        },
        "scene": {
            "scene_number": scene_number,
            "prompt": scene_prompt}},
    "output_contract": {
        "format": "plain_text",
        "requirements": [
            "只输出场景正文",
            "不要包含标题、编号、Markdown 或解释文字",
            "正文必须承接稳定上下文中的 chapter.summary，并落实 scene.prompt"
        ]}});
    build_json_task_user_message(
        "撰写章节场景",
        "只返回 output_contract 要求的场景正文，不要输出其他内容",
        &payload,
    )
}
