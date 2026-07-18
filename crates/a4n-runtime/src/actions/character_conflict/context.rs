use a4n_core::{
    chapter_analysis::summary_text,
    error::AppError,
    models::{
        chapter::chapter,
        character::{attribute_event, character_state_index},
    },
};
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub(super) const MAX_CONFLICT_FALLBACK_CONTENT_CHARS: usize = 4_000;
pub(super) const MAX_CONFLICT_TOTAL_CONTEXT_CHARS: usize = 24_000;

pub(super) fn character_conflict_chapter_context(
    chapter: &chapter::Model,
) -> Result<Option<Value>, AppError> {
    let mut object = serde_json::Map::new();
    object.insert(
        "source_id".to_string(),
        json!(format!("chapter:{}", chapter.chapter_number)),
    );
    object.insert("chapter_number".to_string(), json!(chapter.chapter_number));
    if let Some(title) = chapter
        .title
        .as_deref()
        .map(str::trim)
        .filter(|title| !title.is_empty())
    {
        object.insert("title".to_string(), json!(title));
    }
    if let Some(summary) = chapter
        .analysis_data
        .as_ref()
        .and_then(summary_text)
        .or_else(|| {
            chapter
                .summary
                .as_deref()
                .map(str::trim)
                .filter(|summary| !summary.is_empty())
                .map(ToOwned::to_owned)
        })
    {
        object.insert("summary".to_string(), json!(summary));
    } else if !chapter.content.trim().is_empty() {
        let content = chapter.content.trim();
        let content_chars = content.chars().count();
        if content_chars > MAX_CONFLICT_FALLBACK_CONTENT_CHARS {
            return Err(AppError::InvalidArgument(format!(
                "第 {} 章缺少结构化分析摘要和章节摘要，且正文长度 {} 字超过角色一致性检查兜底上限 {} 字；请先生成章节分析或补充章节摘要后重试",
                chapter.chapter_number, content_chars, MAX_CONFLICT_FALLBACK_CONTENT_CHARS
            )));
        }
        object.insert("content".to_string(), json!(content));
    } else {
        return Ok(None);
    }
    if let Some(analysis) = chapter.analysis_data.as_ref() {
        let mut analysis_context = serde_json::Map::new();
        if let Some(key_events) = analysis.get("key_events") {
            analysis_context.insert("key_events".to_string(), key_events.clone());
        }
        if let Some(metadata) = analysis.get("metadata") {
            analysis_context.insert("metadata".to_string(), metadata.clone());
        }
        if !analysis_context.is_empty() {
            object.insert("analysis".to_string(), Value::Object(analysis_context));
        }
    }
    Ok(Some(Value::Object(object)))
}

pub(super) fn character_state_context(state: &character_state_index::Model) -> Value {
    json!({
        "source_id": format!("state:{}", state.chapter_number),
        "chapter_number": state.chapter_number,
        "state_summary": state.state_summary,
        "key_attributes": state.key_attributes,
        "emotional_state": state.emotional_state.as_deref().map(str::trim).filter(|value| !value.is_empty()),
        "current_location": state.current_location.as_deref().map(str::trim).filter(|value| !value.is_empty()),
        "current_goal": state.current_goal.as_deref().map(str::trim).filter(|value| !value.is_empty()),
    })
}

pub(super) fn attribute_event_context(event: &attribute_event::Model) -> Value {
    json!({
        "source_id": format!("attribute_event:{}", event.id),
        "chapter_number": event.chapter_number,
        "category": event.category,
        "key": event.key,
        "operation": event.operation,
        "change_description": event.change_description,
        "old_value": event.old_value,
        "new_value": event.new_value,
        "cause": event.cause,
        "evidence": event.evidence,
    })
}

pub(super) fn character_conflict_allowed_source_ids(
    role_attr: &Value,
    chapter_context: &[Value],
    state_timeline: &[Value],
    attribute_events: &[Value],
) -> BTreeSet<String> {
    let mut source_ids = BTreeSet::new();
    collect_top_level_source_id(role_attr, &mut source_ids);
    for source in chapter_context
        .iter()
        .chain(state_timeline)
        .chain(attribute_events)
    {
        collect_top_level_source_id(source, &mut source_ids);
    }
    source_ids
}

pub(super) fn ensure_character_conflict_has_comparison_context(
    chapter_context: &[Value],
    state_timeline: &[Value],
    attribute_events: &[Value],
) -> Result<(), AppError> {
    if chapter_context.is_empty() && state_timeline.is_empty() && attribute_events.is_empty() {
        Err(AppError::InvalidArgument(
            "角色一致性检查缺少可对照上下文：没有可用章节上下文、角色状态时间线或属性事件；请先生成章节分析、同步角色档案或补充可检查上下文后重试"
                .to_string(),
        ))
    } else {
        Ok(())
    }
}

pub(super) fn ensure_character_conflict_context_size(
    role_attr: &Value,
    chapter_context: &[Value],
    state_timeline: &[Value],
    attribute_events: &[Value],
) -> Result<(), AppError> {
    let total_chars = json!({
        "character_profile": role_attr,
        "chapter_context": chapter_context,
        "character_state_timeline": state_timeline,
        "attribute_events": attribute_events,
    })
    .to_string()
    .chars()
    .count();
    if total_chars > MAX_CONFLICT_TOTAL_CONTEXT_CHARS {
        Err(AppError::InvalidArgument(format!(
            "角色一致性检查全量上下文过大：当前 {} 字，超过上限 {} 字；请先生成章节分析或章节摘要以减少正文兜底，或改用更小范围的检查入口后重试",
            total_chars, MAX_CONFLICT_TOTAL_CONTEXT_CHARS
        )))
    } else {
        Ok(())
    }
}

fn collect_top_level_source_id(value: &Value, out: &mut BTreeSet<String>) {
    if let Some(source_id) = value
        .get("source_id")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        out.insert(source_id.to_string());
    }
}
