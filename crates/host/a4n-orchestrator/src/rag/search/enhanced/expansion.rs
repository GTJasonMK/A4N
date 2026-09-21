use a4n_db::rag::PendingForeshadowingSearchRow;
use a4n_domain::text_location::extract_scene_location;
use a4n_error::AppError;

use crate::OrchestratorContext;
use a4n_rag_core::EnhancedRagQuery;

pub(super) async fn matched_blueprint_character_names(
    context: &OrchestratorContext,
    project_id: i64,
    query: &str,
) -> Result<Vec<String>, AppError> {
    let characters = context
        .blueprint_characters_for_rag_search(project_id)
        .await?;
    let mut names = characters
        .into_iter()
        .map(|name| name.trim().to_string())
        .filter(|name| !name.is_empty() && query.contains(name))
        .collect::<Vec<_>>();
    names.sort();
    names.dedup();
    names.truncate(4);
    Ok(names)
}

pub(super) async fn candidate_foreshadowing(
    context: &OrchestratorContext,
    project_id: i64,
    chapter_number: Option<i32>,
) -> Result<Vec<PendingForeshadowingSearchRow>, AppError> {
    context
        .pending_foreshadowing_search_rows(
            project_id,
            chapter_number.filter(|value| *value > 1),
            12,
        )
        .await
}

impl OrchestratorContext {
    pub async fn prepare_enhanced_rag_queries(
        &self,
        project_id: i64,
        base_query: &str,
        chapter_number: Option<i32>,
    ) -> Result<Vec<EnhancedRagQuery>, AppError> {
        let character_names =
            matched_blueprint_character_names(self, project_id, base_query).await?;
        let foreshadowing = filter_relevant_foreshadowing(
            candidate_foreshadowing(self, project_id, chapter_number).await?,
            base_query,
            &character_names,
        );
        Ok(build_enhanced_rag_queries(
            base_query,
            &character_names,
            &foreshadowing,
        ))
    }
}

pub(super) fn build_enhanced_rag_queries(
    base_query: &str,
    character_names: &[String],
    foreshadowing: &[PendingForeshadowingSearchRow],
) -> Vec<EnhancedRagQuery> {
    let mut queries = Vec::new();
    push_enhanced_query(&mut queries, "主查询", base_query, 1.0);
    for name in character_names.iter().take(3) {
        push_enhanced_query(
            &mut queries,
            "角色",
            format!("角色 {name} 的行动、状态、关系和立场变化"),
            0.85,
        );
    }
    for item in foreshadowing.iter().take(3) {
        push_enhanced_query(
            &mut queries,
            "伏笔",
            format!("伏笔: {}", item.description.trim()),
            0.95,
        );
    }
    if let Some(location) = extract_scene_location(base_query) {
        push_enhanced_query(
            &mut queries,
            "场景",
            format!("场景 {location} 中发生的事件和相关设定"),
            0.75,
        );
    }
    queries
}

pub(super) fn filter_relevant_foreshadowing(
    items: Vec<PendingForeshadowingSearchRow>,
    query: &str,
    character_names: &[String],
) -> Vec<PendingForeshadowingSearchRow> {
    items
        .into_iter()
        .filter(|item| {
            item.importance >= 4
                || text_has_direct_overlap(query, &item.description)
                || item
                    .related_characters
                    .as_ref()
                    .is_some_and(|value| json_contains_any_character_name(value, character_names))
        })
        .collect()
}

fn push_enhanced_query(
    queries: &mut Vec<EnhancedRagQuery>,
    label: &'static str,
    query: impl Into<String>,
    weight: f32,
) {
    let query = query.into().trim().to_string();
    if query.is_empty() || queries.iter().any(|item| item.query == query) {
        return;
    }
    queries.push(EnhancedRagQuery {
        label,
        query,
        weight,
    });
}

fn text_has_direct_overlap(query: &str, text: &str) -> bool {
    let query = query.trim();
    let text = text.trim();
    query.chars().count() >= 2
        && text.chars().count() >= 2
        && (query.contains(text) || text.contains(query))
}

fn json_contains_any_character_name(value: &serde_json::Value, names: &[String]) -> bool {
    match value {
        serde_json::Value::String(text) => names.iter().any(|name| text.contains(name)),
        serde_json::Value::Array(items) => items
            .iter()
            .any(|item| json_contains_any_character_name(item, names)),
        serde_json::Value::Object(map) => map
            .values()
            .any(|item| json_contains_any_character_name(item, names)),
        _ => false,
    }
}
