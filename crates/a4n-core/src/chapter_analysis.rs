use serde_json::Value;

use crate::{error::AppError, models::chapter::chapter, structured_progress::FieldMilestone};

mod extraction;
mod validation;

pub use extraction::{
    AnalysisRagItem, CharacterStateSnapshot, ForeshadowingEntry, analysis_rag_items,
    character_state_snapshots, foreshadowing_entries, priority_to_importance,
};
pub use validation::{validate_chapter_analysis_schema, validate_saved_chapter_analysis};

pub const CHAPTER_ANALYSIS_STAGE_FACTS: &str = "chapter_fact_extraction";
pub const CHAPTER_ANALYSIS_STAGE_SUMMARY: &str = "chapter_summary_synthesis";
pub const CHAPTER_ANALYSIS_STAGE_CHARACTER_STATE: &str = "chapter_character_state";
pub const CHAPTER_ANALYSIS_STAGE_FORESHADOWING_EVENTS: &str = "chapter_foreshadowing_events";
pub const CHAPTER_ANALYSIS_STEP_FACTS: &str = "chapter_analysis.facts";
pub const CHAPTER_ANALYSIS_STEP_SUMMARY: &str = "chapter_analysis.summary";
pub const CHAPTER_ANALYSIS_STEP_CHARACTER_STATES: &str = "chapter_analysis.character_states";
pub const CHAPTER_ANALYSIS_STEP_FORESHADOWING_EVENTS: &str =
    "chapter_analysis.foreshadowing_events";
pub const CHAPTER_ANALYSIS_STEP_FINAL: &str = "chapter_analysis.final";
pub const CHAPTER_ANALYSIS_SERVICE_PROGRESS_TOTAL: u32 = 100;

pub const CHAPTER_ANALYSIS_FACT_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("characters", "角色候选", 1),
    FieldMilestone::new("locations", "地点候选", 1),
    FieldMilestone::new("items", "物品候选", 1),
    FieldMilestone::new("event_candidates", "事件候选", 1),
    FieldMilestone::new("foreshadowing_candidates", "伏笔候选", 1),
];
pub const CHAPTER_ANALYSIS_SUMMARY_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("metadata_additions", "元数据补充", 1),
    FieldMilestone::new("summaries", "章节摘要", 1),
];
pub const CHAPTER_ANALYSIS_CHARACTER_STATE_FIELDS: &[FieldMilestone] =
    &[FieldMilestone::new("character_states", "角色状态", 1)];
pub const CHAPTER_ANALYSIS_FORESHADOWING_FIELDS: &[FieldMilestone] = &[
    FieldMilestone::new("foreshadowing", "伏笔结构", 1),
    FieldMilestone::new("key_events", "关键事件", 1),
];

pub fn ensure_chapter_has_analysis_content(ch: &chapter::Model) -> Result<(), AppError> {
    if ch.has_available_content() {
        return Ok(());
    }
    Err(AppError::InvalidArgument(
        "章节正文为空，不能生成结构化分析".to_string(),
    ))
}

/// Projects the preferred human-readable summary from a normalized chapter
/// analysis document.
pub fn summary_text(analysis: &Value) -> Option<String> {
    let summaries = analysis.get("summaries")?;
    first_scalar_text(summaries, &["compressed", "one_line"]).or_else(|| {
        summaries
            .get("keywords")
            .and_then(Value::as_array)
            .map(|items| {
                items
                    .iter()
                    .filter_map(scalar_text)
                    .collect::<Vec<_>>()
                    .join("、")
            })
            .filter(|text| !text.trim().is_empty())
    })
}

fn first_scalar_text(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .filter_map(|key| value.get(*key))
        .find_map(scalar_text)
}

fn scalar_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => {
            let text = text.trim();
            (!text.is_empty()).then(|| text.to_string())
        }
        Value::Number(number) => Some(number.to_string()),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{
        CHAPTER_ANALYSIS_SERVICE_PROGRESS_TOTAL, CHAPTER_ANALYSIS_STAGE_FACTS,
        CHAPTER_ANALYSIS_STEP_FINAL, summary_text,
    };

    #[test]
    fn workflow_contract_keeps_stable_stage_and_progress_identity() {
        assert_eq!(CHAPTER_ANALYSIS_STAGE_FACTS, "chapter_fact_extraction");
        assert_eq!(CHAPTER_ANALYSIS_STEP_FINAL, "chapter_analysis.final");
        assert_eq!(CHAPTER_ANALYSIS_SERVICE_PROGRESS_TOTAL, 100);
    }

    #[test]
    fn prefers_trimmed_compressed_summary() {
        let analysis = json!({
            "summaries": {
                "compressed": "  压缩摘要  ",
                "one_line": "单行摘要",
                "keywords": ["关键词"]
            }
        });

        assert_eq!(summary_text(&analysis).as_deref(), Some("压缩摘要"));
    }

    #[test]
    fn falls_back_to_one_line_when_compressed_is_empty() {
        let analysis = json!({
            "summaries": {
                "compressed": "   ",
                "one_line": "  单行摘要  "
            }
        });

        assert_eq!(summary_text(&analysis).as_deref(), Some("单行摘要"));
    }

    #[test]
    fn joins_normalized_scalar_keywords() {
        let analysis = json!({
            "summaries": {
                "compressed": null,
                "one_line": null,
                "keywords": ["  转折 ", "", 7, null, {"ignored": true}]
            }
        });

        assert_eq!(summary_text(&analysis).as_deref(), Some("转折、7"));
    }

    #[test]
    fn returns_none_without_non_empty_summary_content() {
        assert_eq!(summary_text(&json!({})), None);
        assert_eq!(
            summary_text(&json!({"summaries": {"keywords": [" ", null]}})),
            None
        );
    }
}
