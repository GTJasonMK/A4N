use a4n_core::{
    chapter_analysis::CHAPTER_ANALYSIS_STAGE_FORESHADOWING_EVENTS, error::AppError,
    models::chapter::chapter,
};
use a4n_db::rag::RagForeshadowingRow;
use a4n_domain::character::ForeshadowingStatus;
use a4n_llm::provider::ChatRequest;
use serde_json::{Value, json};

use a4n_prompt::rendering::PromptRenderer;

use super::required_fact_field;
use crate::chapter_analysis::owner::{
    prompt::chapter_display_title, requests::context::build_chapter_analysis_stage_request,
};

pub async fn build_chapter_foreshadowing_events_request(
    ch: &chapter::Model,
    prompt_renderer: &dyn PromptRenderer,
    facts: &Value,
    pending_foreshadowing: &Value,
) -> Result<ChatRequest, AppError> {
    let chapter_title = chapter_display_title(ch);
    let event_candidates = required_fact_field(facts, "event_candidates")?;
    let foreshadowing_candidates = required_fact_field(facts, "foreshadowing_candidates")?;
    build_chapter_analysis_stage_request(
        ch,
        prompt_renderer,
        CHAPTER_ANALYSIS_STAGE_FORESHADOWING_EVENTS,
        "从事实候选中筛选最终伏笔和关键事件",
        json!({
            "chapter_title": chapter_title,
            "facts": {
                "event_candidates": event_candidates,
                "foreshadowing_candidates": foreshadowing_candidates,
            },
            "pending_foreshadowing": pending_foreshadowing,
        }),
        json!({
            "format": "json_object",
            "required_fields": ["foreshadowing", "key_events"],
            "source_span_rule": "planted 和 key_events 只能使用 facts 中已有的 source span id",
            "resolution_rule": "resolved 只能引用 pending_foreshadowing 中已有的 id"
        }),
    )
    .await
}

pub fn chapter_foreshadowing_resolution_context(
    chapter_number: i32,
    items: &[RagForeshadowingRow],
) -> Value {
    Value::Array(
        items
            .iter()
            .filter(|item| {
                item.setup_chapter < chapter_number
                    && ((item.payoff_chapter.is_none()
                        && ForeshadowingStatus::parse(&item.status)
                            .is_some_and(ForeshadowingStatus::can_resolve))
                        || (item.payoff_chapter == Some(chapter_number)
                            && item.status == ForeshadowingStatus::Resolved.as_str()))
            })
            .map(|item| {
                json!({
                    "id": item.id.to_string(),
                    "description": item.description,
                    "setup_chapter": item.setup_chapter,
                    "payoff_chapter": item.payoff_chapter,
                    "status": item.status,
                    "importance": item.importance,
                    "related_characters": item.related_characters,
                })
            })
            .collect(),
    )
}

#[cfg(test)]
mod tests {
    use a4n_db::rag::RagForeshadowingRow;

    use super::chapter_foreshadowing_resolution_context;

    fn item(
        id: i64,
        setup_chapter: i32,
        payoff_chapter: Option<i32>,
        status: &str,
    ) -> RagForeshadowingRow {
        RagForeshadowingRow {
            id,
            description: format!("伏笔 {id}"),
            setup_chapter,
            payoff_chapter,
            status: status.to_string(),
            related_characters: None,
            importance: 3,
        }
    }

    #[test]
    fn resolution_context_keeps_pending_and_current_chapter_payoffs_only() {
        let context = chapter_foreshadowing_resolution_context(
            8,
            &[
                item(1, 2, None, "pending"),
                item(2, 3, None, "partially_revealed"),
                item(3, 4, Some(8), "resolved"),
                item(4, 4, Some(7), "resolved"),
                item(5, 8, None, "pending"),
                item(6, 1, None, "abandoned"),
            ],
        );

        let ids = context
            .as_array()
            .expect("context must be an array")
            .iter()
            .map(|item| item["id"].as_str().expect("id").to_string())
            .collect::<Vec<_>>();
        assert_eq!(ids, ["1", "2", "3"]);
    }
}
