use a4n_error::AppError;
use serde_json::Value;

use a4n_domain::source_spans::{SourceSpan, SourceSpanIndex};

mod entities;
mod events;
mod foreshadowing;
use entities::validate_fact_entity_array;
use events::validate_event_candidates;
use foreshadowing::validate_foreshadowing_candidates;
mod root;
use root::validate_root_fields;

pub fn validate_chapter_fact_extraction(
    facts: &Value,
    source_spans: &[SourceSpan],
) -> Result<(), AppError> {
    let source_span_index = SourceSpanIndex::new(source_spans);
    validate_root_fields(facts)?;
    for key in ["characters", "locations", "items"] {
        validate_fact_entity_array(facts, key, &source_span_index)?;
    }
    validate_event_candidates(facts, &source_span_index)?;

    validate_foreshadowing_candidates(facts, &source_span_index)?;
    if facts
        .get("world_facts")
        .and_then(Value::as_array)
        .is_some_and(|facts| facts.iter().any(|fact| fact.get("evidence").is_some()))
    {
        return Err(AppError::InvalidArgument(
            "世界事实的 evidence 由系统从原文生成，模型只能返回 source_span_ids".into(),
        ));
    }
    let world_facts: Vec<a4n_domain::fact_feedback::WorldFact> =
        serde_json::from_value(facts.get("world_facts").cloned().ok_or_else(|| {
            AppError::InvalidArgument("chapter_fact_extraction.world_facts 必须是数组".into())
        })?)
        .map_err(|error| AppError::InvalidArgument(format!("世界事实结构无效: {error}")))?;
    let mut identities = std::collections::BTreeSet::new();
    for fact in world_facts {
        fact.validate().map_err(AppError::InvalidArgument)?;
        if !identities.insert(fact.identity()) {
            return Err(AppError::InvalidArgument(
                "世界事实包含重复主题，请合并同一主题的本章事实".into(),
            ));
        }
        for id in fact.source_span_ids {
            source_span_index
                .resolve_text(&id)
                .map_err(AppError::InvalidArgument)?;
        }
    }
    Ok(())
}
