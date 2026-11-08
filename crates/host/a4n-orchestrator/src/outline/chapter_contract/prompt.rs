use a4n_domain::chapter::PartOutlineRecord;
use a4n_error::AppError;
use a4n_prompt::request::build_json_task_user_message;
use serde_json::{Value, json};

pub(super) const MAX_CHAPTER_OUTLINE_GENERATION_COUNT: i32 = 50;

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
    if count > MAX_CHAPTER_OUTLINE_GENERATION_COUNT {
        return Err(AppError::InvalidArgument(format!(
            "章节大纲生成 count 不能超过 {MAX_CHAPTER_OUTLINE_GENERATION_COUNT}"
        )));
    }
    start_chapter
        .checked_add(count - 1)
        .ok_or_else(|| AppError::InvalidArgument("章节大纲生成范围溢出".to_string()))
}

pub(in crate::outline) fn chapter_outline_generation_manifest_user_message(
    focus_part: Option<&PartOutlineRecord>,
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
             字段、类型和可空约束遵守系统输出合同。"
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
    part: &PartOutlineRecord,
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
             字段、类型和可空约束遵守系统输出合同；不要输出 beat_assignment。",
            part.start_chapter, part.end_chapter
        ),
        &payload,
    )
}
