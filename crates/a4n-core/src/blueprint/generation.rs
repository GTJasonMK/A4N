use std::collections::HashMap;

use a4n_domain::project::{
    BLUEPRINT_MAX_TOTAL_CHAPTERS, BLUEPRINT_MIN_GENERATION_CHAPTERS,
    is_valid_blueprint_generation_chapters,
};
use serde_json::{Value, json};

use crate::error::AppError;

pub const BLUEPRINT_GENERATE_PROGRESS_TOTAL: u32 = 12;
pub const BLUEPRINT_GENERATION_KIND: &str = "blueprint_generation";
pub const BLUEPRINT_BASE_PROMPT: &str = "blueprint";
pub const BLUEPRINT_STAGE_SYSTEM_PROMPT: &str = "blueprint_stage_system";
pub const BLUEPRINT_STAGE_USER_PROMPT: &str = "blueprint_stage_user";
pub const BLUEPRINT_GENERATION_PROMPTS: [&str; 3] = [
    BLUEPRINT_BASE_PROMPT,
    BLUEPRINT_STAGE_SYSTEM_PROMPT,
    BLUEPRINT_STAGE_USER_PROMPT,
];
pub const BLUEPRINT_REFINEMENT_TASK_KIND: &str = "blueprint_refinement";
pub const BLUEPRINT_REFINEMENT_WORKFLOW_KIND: &str = "blueprint_refine";
pub const BLUEPRINT_REFINEMENT_MODE: &str = "refine";
pub const BLUEPRINT_REFINEMENT_PROMPT: &str = "blueprint_optimization";
pub const BLUEPRINT_REFINEMENT_USER_PROMPT: &str = "blueprint_optimization_user";
pub const BLUEPRINT_REFINEMENT_PROMPTS: [&str; 2] = [
    BLUEPRINT_REFINEMENT_PROMPT,
    BLUEPRINT_REFINEMENT_USER_PROMPT,
];
pub const BLUEPRINT_REFINEMENT_FIELDS: &[&str] = &[
    "style",
    "full_synopsis",
    "world_setting",
    "characters",
    "relationships",
];
pub const BLUEPRINT_RAW_JSON_STREAM_ID: &str = "raw_json";

pub fn blueprint_stage_call_key(stage_key: &str) -> String {
    format!("{BLUEPRINT_GENERATION_KIND}.{stage_key}")
}

pub fn blueprint_refinement_business_key(project_id: i64) -> String {
    format!("{BLUEPRINT_REFINEMENT_WORKFLOW_KIND}:{project_id}")
}

pub fn blueprint_refinement_call_key(project_id: i64) -> String {
    format!("{BLUEPRINT_REFINEMENT_WORKFLOW_KIND}:project:{project_id}")
}

/// 蓝图任务与 workflow 共用的业务输入指纹。
///
/// 任务 reservation 和 workflow resume 必须使用完全相同的 JSON；否则失败后
/// 重新提交会创建新的 workflow，已有阶段 checkpoint 就无法命中。
pub fn blueprint_generation_input_fingerprint(
    project_id: i64,
    title: &str,
    genre: &str,
    target_length: i32,
    variables: &Value,
) -> Value {
    json!({
        "version": 1,
        "task_kind": BLUEPRINT_GENERATION_KIND,
        "project_id": project_id,
        "title": title,
        "genre": genre,
        "target_length": target_length,
        "variables": variables,
    })
}

pub fn blueprint_refinement_input_fingerprint(
    project_id: i64,
    focus: Option<&str>,
    variables: &Value,
) -> Value {
    json!({
        "version": 1,
        "task_kind": BLUEPRINT_REFINEMENT_TASK_KIND,
        "project_id": project_id,
        "focus": normalized_blueprint_refinement_focus(focus),
        "variables": variables,
    })
}

pub fn normalized_blueprint_refinement_focus(focus: Option<&str>) -> Option<&str> {
    focus.map(str::trim).filter(|value| !value.is_empty())
}

pub fn validate_blueprint_chapter_count(value: i32, context: &str) -> Result<(), AppError> {
    if is_valid_blueprint_generation_chapters(value) {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(format!(
            "{context} 必须在 {BLUEPRINT_MIN_GENERATION_CHAPTERS} 到 {BLUEPRINT_MAX_TOTAL_CHAPTERS} 之间"
        )))
    }
}

pub fn build_blueprint_generation_payload(
    novel_title: &str,
    genre: &str,
    target_chapters: i32,
    variables: &HashMap<String, Value>,
) -> Result<Value, AppError> {
    validate_blueprint_chapter_count(target_chapters, "requested_chapter_count")?;

    let mut payload = serde_json::Map::new();
    payload.insert(
        "novel_title".to_string(),
        Value::String(novel_title.to_string()),
    );
    payload.insert("genre".to_string(), Value::String(genre.to_string()));
    payload.insert(
        "requested_chapter_count".to_string(),
        Value::from(target_chapters),
    );
    for key in [
        "inspiration_transcript",
        "generation_instruction",
        "conversation_state",
        "progress_summary",
        "next_question",
        "project",
    ] {
        if let Some(value) = variables.get(key) {
            payload.insert(key.to_string(), value.clone());
        }
    }
    Ok(Value::Object(payload))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blueprint_resume_fingerprints_are_canonical() {
        let variables = json!({"genre_hint": "悬疑"});
        assert_eq!(
            blueprint_generation_input_fingerprint(7, "雾城", "悬疑", 20, &variables),
            json!({
                "version": 1,
                "task_kind": BLUEPRINT_GENERATION_KIND,
                "project_id": 7,
                "title": "雾城",
                "genre": "悬疑",
                "target_length": 20,
                "variables": {"genre_hint": "悬疑"},
            })
        );
        assert_eq!(
            blueprint_refinement_input_fingerprint(7, Some("  世界观  "), &variables)["focus"],
            "世界观"
        );
    }
}
