use a4n_error::AppError;
use a4n_prompt::request::{build_json_task_user_message, json_user_message_payload};
use serde_json::{Value, json};

pub(in crate::optimize) fn build_content_optimization_user_message(
    chapter_content: &str,
    context_manifest: &Value,
) -> Result<String, AppError> {
    let chapter_content = chapter_content.trim();
    if chapter_content.is_empty() {
        return Err(AppError::InvalidArgument(
            "内容优化分析缺少章节正文".to_string(),
        ));
    }
    let payload = json!({
        "task": {
            "name": "content_optimization_agent_loop",
            "requirement": "按 system 指定的维度和用户指令分析章节，以实际读取的原文引用支撑 final.issues。",
        },
        "chapter": {
            "content_available_via": "get_project_information(mode=read, source=current_chapter)",
            "content_chars": chapter_content.chars().count(),
        },
        "context_manifest": context_manifest,
    });
    build_json_task_user_message(
        "执行内容优化 Agent Loop",
        "遵守 system 的决策协议，只返回一个严格 JSON 决策对象",
        &payload,
    )
}

pub(in crate::optimize) fn is_content_optimization_analysis_prompt(content: &str) -> bool {
    json_user_message_payload(content)
        .and_then(|payload| serde_json::from_str::<Value>(payload).ok())
        .and_then(|payload| {
            payload
                .get("task")
                .and_then(|task| task.get("name"))
                .and_then(Value::as_str)
                .map(str::to_string)
        })
        .as_deref()
        == Some("content_optimization_agent_loop")
}

pub(in crate::optimize) fn optimization_dimensions(mode: &str) -> Result<&'static str, AppError> {
    match mode {
        "full" => {
            Ok("逻辑连贯性、角色一致性、伏笔呼应、时间线一致性、风格一致性、场景描写、语法与表达")
        }
        "quick" => Ok("阻断阅读或明显影响理解的问题：逻辑断裂、角色矛盾、时间线错误、严重语病"),
        "style_only" => Ok("风格一致性、表达节奏、语气、重复用词、句式单调"),
        "plot_only" => Ok("逻辑连贯性、因果关系、角色动机、伏笔呼应、时间线一致性"),
        other => Err(AppError::InvalidArgument(format!(
            "未知优化分析模式: {other}"
        ))),
    }
}
