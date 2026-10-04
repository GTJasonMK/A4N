//! 角色冲突检查上下文构建纯规则。

use a4n_chapter_analysis::summary_text;
use a4n_domain::{
    chapter::ChapterRecord,
    character::{CharacterAttributeEventRecord, CharacterStateIndexRecord},
};
use a4n_error::AppError;
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub const MAX_CONFLICT_TOTAL_CONTEXT_CHARS: usize = 24_000;

/// 角色一致性检查章节范围。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterConflictChapterScope {
    pub start_chapter: Option<i32>,
    pub end_chapter: Option<i32>,
}

impl CharacterConflictChapterScope {
    pub fn new(start_chapter: Option<i32>, end_chapter: Option<i32>) -> Result<Self, AppError> {
        if start_chapter.is_some_and(|value| value <= 0) {
            return Err(AppError::InvalidArgument(
                "角色一致性检查起始章节必须大于 0".to_string(),
            ));
        }
        if end_chapter.is_some_and(|value| value <= 0) {
            return Err(AppError::InvalidArgument(
                "角色一致性检查结束章节必须大于 0".to_string(),
            ));
        }
        if let (Some(start), Some(end)) = (start_chapter, end_chapter)
            && start > end
        {
            return Err(AppError::InvalidArgument(
                "角色一致性检查起始章节不能大于结束章节".to_string(),
            ));
        }
        Ok(Self {
            start_chapter,
            end_chapter,
        })
    }

    pub fn to_context_value(self) -> Value {
        json!({
            "start_chapter": self.start_chapter,
            "end_chapter": self.end_chapter,
        })
    }
}

pub fn character_conflict_chapter_context(
    chapter: &ChapterRecord,
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
    let Some(summary) = chapter
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
    else {
        return Ok(None);
    };
    object.insert("summary".to_string(), json!(summary));
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

pub fn character_state_context(state: &CharacterStateIndexRecord) -> Value {
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

pub fn attribute_event_context(event: &CharacterAttributeEventRecord) -> Value {
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

pub fn character_conflict_allowed_source_ids(
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

pub fn ensure_character_conflict_has_comparison_context(
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

pub fn ensure_character_conflict_context_size(
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
            "角色一致性检查全量上下文过大：当前 {} 字，超过上限 {} 字；请先生成章节分析或章节摘要以减少上下文，或改用更小范围的检查入口后重试",
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn sample_chapter(summary: Option<&str>, content: &str) -> ChapterRecord {
        let now = NaiveDate::from_ymd_opt(2026, 5, 31)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap();
        ChapterRecord {
            id: 1,
            project_id: 9,
            outline_id: None,
            chapter_number: 3,
            title: Some("旧城".to_string()),
            content: content.to_string(),
            word_count: content.chars().count() as i32,
            status: "draft".to_string(),
            summary: summary.map(ToOwned::to_owned),
            style_tags: None,
            analysis_data: None,
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn chapter_context_requires_summary_without_body_fallback() {
        let chapter = sample_chapter(Some("主角进入旧城。"), "完整正文不应重复塞入");
        let context = character_conflict_chapter_context(&chapter)
            .unwrap()
            .expect("chapter should be included");
        assert_eq!(context["source_id"], json!("chapter:3"));
        assert_eq!(context["summary"], json!("主角进入旧城。"));
        assert!(context.get("content").is_none());

        assert!(
            character_conflict_chapter_context(&sample_chapter(
                None,
                "陈恪走进旧城，看见魏征站在雨里。",
            ))
            .unwrap()
            .is_none()
        );
    }

    #[test]
    fn allowed_source_ids_and_size_gates() {
        let role = json!({"source_id": "profile"});
        let chapters = vec![json!({
            "source_id": "chapter:3",
            "analysis": {"key_events": [{"source_id": "legacy_event:1"}]},
        })];
        let ids = character_conflict_allowed_source_ids(
            &role,
            &chapters,
            &[json!({"source_id": "state:3"})],
            &[json!({"source_id": "attribute_event:9"})],
        );
        assert!(ids.contains("profile"));
        assert!(ids.contains("chapter:3"));
        assert!(!ids.contains("legacy_event:1"));

        ensure_character_conflict_has_comparison_context(&[json!({})], &[], &[]).unwrap();
        assert!(ensure_character_conflict_has_comparison_context(&[], &[], &[]).is_err());

        ensure_character_conflict_context_size(
            &json!({"source_id": "profile"}),
            &[json!({"source_id": "chapter:3", "summary": "主角进入旧城。"})],
            &[],
            &[],
        )
        .unwrap();
        let err = ensure_character_conflict_context_size(
            &json!({"source_id": "profile"}),
            &[json!({"summary": "角色状态摘要过长。".repeat(MAX_CONFLICT_TOTAL_CONTEXT_CHARS)})],
            &[],
            &[],
        )
        .unwrap_err();
        assert!(err.to_string().contains("全量上下文过大"));
    }

    #[test]
    fn chapter_scope_validates_range() {
        assert_eq!(
            CharacterConflictChapterScope::new(None, None).unwrap(),
            CharacterConflictChapterScope {
                start_chapter: None,
                end_chapter: None,
            }
        );
        assert_eq!(
            CharacterConflictChapterScope::new(Some(2), Some(5)).unwrap(),
            CharacterConflictChapterScope {
                start_chapter: Some(2),
                end_chapter: Some(5),
            }
        );
        assert!(CharacterConflictChapterScope::new(Some(0), None).is_err());
        assert!(CharacterConflictChapterScope::new(None, Some(-1)).is_err());
        assert!(CharacterConflictChapterScope::new(Some(5), Some(2)).is_err());
    }
}
