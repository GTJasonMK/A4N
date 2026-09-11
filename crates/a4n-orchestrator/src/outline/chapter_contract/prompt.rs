use a4n_db_models::models::chapter::part_outline;
use a4n_error::AppError;
use a4n_prompt::request::build_json_task_user_message;
use serde_json::{Value, json};

pub(in crate::outline) fn validate_chapter_outline_generation_request(
    start_chapter: i32,
    count: i32,
) -> Result<i32, AppError> {
    if start_chapter <= 0 {
        return Err(AppError::InvalidArgument(
            "章节大纲生成 start_chapter 必须大于 0".to_string(),
        ));
    }
    if count <= 0 {
        return Err(AppError::InvalidArgument(
            "章节大纲生成 count 必须大于 0".to_string(),
        ));
    }
    start_chapter
        .checked_add(count - 1)
        .ok_or_else(|| AppError::InvalidArgument("章节大纲生成范围溢出".to_string()))
}

pub(in crate::outline) fn chapter_outline_generation_manifest_user_message(
    focus_part: Option<&part_outline::Model>,
    start_chapter: i32,
    count: i32,
    end_chapter: i32,
) -> Result<String, AppError> {
    let payload = json!({
        "context_refs": {
            "novel_blueprint": "稳定上下文包 outline_blueprint_context",
            "part_outlines": "稳定上下文包 outline_part_outlines",
            "active_part_outline": focus_part.map(|part| {
                format!("稳定上下文包 outline_part_outlines 中 part_number={}", part.part_number)
            }),
        },
        "wait_to_generate": {
            "start_chapter": start_chapter,
            "num_chapters": count,
            "end_chapter": end_chapter,
        },
    });
    build_json_task_user_message(
        "生成章节大纲",
        &format!(
            "只输出一个 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段。\
             输出根对象必须且只能包含 chapters。\
             chapters 必须包含 {count} 个对象，数组顺序即第 {start_chapter} 章到第 {end_chapter} 章；不要输出 chapter_number，后端会按顺序补齐。\
             每个 chapters 元素必须包含 title、summary；可额外包含且只能包含 beats、key_scenes、pacing_score、foreshadowing_setup、foreshadowing_payoff、characters_involved、word_count_target。\
             beats、key_scenes、foreshadowing_setup、foreshadowing_payoff、characters_involved 必须是 JSON 数组。\
             pacing_score、word_count_target 必须是 JSON 数字，不能是字符串"
        ),
        &payload,
    )
}

pub(in crate::outline) fn parse_beat_sheet_json(beat_sheet: &str) -> Result<Value, AppError> {
    let trimmed = beat_sheet.trim();
    if trimmed.is_empty() {
        return Err(AppError::InvalidArgument("beat_sheet 不能为空".to_string()));
    }
    let value: Value = serde_json::from_str(trimmed)
        .map_err(|e| AppError::InvalidArgument(format!("beat_sheet 必须是合法 JSON：{e}")))?;
    if !value.is_object() {
        return Err(AppError::InvalidArgument(
            "beat_sheet 必须是 JSON 对象".to_string(),
        ));
    }
    if value.as_object().is_some_and(|object| object.is_empty()) {
        return Err(AppError::InvalidArgument(
            "beat_sheet 必须是非空 JSON 对象".to_string(),
        ));
    }
    Ok(value)
}

pub(in crate::outline) fn beat_mapping_manifest_user_message(
    part: &part_outline::Model,
    beat_sheet: &Value,
    count: i32,
) -> Result<String, AppError> {
    let payload = json!({
        "context_refs": {
            "novel_blueprint": "稳定上下文包 outline_blueprint_context",
            "part_outlines": "稳定上下文包 outline_part_outlines",
            "active_part_outline": format!("稳定上下文包 outline_part_outlines 中 part_number={}", part.part_number),
        },
        "beat_sheet": beat_sheet,
        "wait_to_generate": {
            "start_chapter": part.start_chapter,
            "num_chapters": count,
            "end_chapter": part.end_chapter,
        },
    });
    build_json_task_user_message(
        &format!(
            "把 beat_sheet 中的节拍点映射为第 {} 分部的章节大纲",
            part.part_number
        ),
        &format!(
            "只输出一个 JSON 对象，不要输出 Markdown、代码块、解释文字或额外字段。\
             输出根对象必须且只能包含 chapters。\
             chapters 必须包含 {count} 个对象，数组顺序即第 {} 章到第 {} 章；不要输出 chapter_number，后端会按顺序补齐。\
             每个 chapters 元素必须包含 title、summary；可额外包含且只能包含 beats、key_scenes、pacing_score、foreshadowing_setup、foreshadowing_payoff、characters_involved、word_count_target。\
             不要输出 beat_assignment 或其他未列出的字段。\
             beats、key_scenes、foreshadowing_setup、foreshadowing_payoff、characters_involved 必须是 JSON 数组。\
             pacing_score、word_count_target 必须是 JSON 数字，不能是字符串",
            part.start_chapter, part.end_chapter
        ),
        &payload,
    )
}

pub(in crate::outline) fn chapter_outline_generation_max_tokens(count: i32) -> usize {
    1_200 + count as usize * 900
}
