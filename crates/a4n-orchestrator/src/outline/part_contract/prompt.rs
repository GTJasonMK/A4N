use a4n_core::error::AppError;
use a4n_core::models::chapter::part_outline;
use a4n_prompt::request::build_json_task_user_message;
use serde_json::{Value, json};

pub const PART_GENERATION_OUTPUT_CONTRACT: &str = "只输出一个 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段。\
     输出字段必须且只能包含：title, summary, theme, key_events, character_focus。\
     不要输出 part_number、start_chapter 或 end_chapter；这些字段由后端根据 generation_request 补齐";

pub fn part_generation_manifest_user_message(
    chapters_per_part: i32,
    requested_part_count: i32,
    part_number: i32,
    start_chapter: i32,
    end_chapter: i32,
    previous_parts: &[part_outline::Model],
) -> Result<String, AppError> {
    let payload = json!({
        "context_refs": {
            "novel_blueprint": "稳定上下文包 outline_blueprint_context",
            "previous_parts": if previous_parts.is_empty() {
                Value::Null
            } else {
                json!("稳定上下文包 outline_previous_part_outlines")
            },
        },
        "generation_request": {
            "chapters_per_part": chapters_per_part,
            "requested_part_count": requested_part_count,
            "part_number": part_number,
            "start_chapter": start_chapter,
            "end_chapter": end_chapter,
        },
        "previous_parts": previous_parts
            .iter()
            .map(|part| {
                json!({
                    "part_number": part.part_number,
                    "context_ref": format!("稳定上下文包 outline_previous_part_outlines 中 part_number={}", part.part_number),
                })
            })
            .collect::<Vec<_>>(),
    });
    build_json_task_user_message(
        &format!("生成第 {part_number} 分部大纲"),
        PART_GENERATION_OUTPUT_CONTRACT,
        &payload,
    )
}
