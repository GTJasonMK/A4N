use std::collections::HashSet;

use crate::AppState;

use super::super::{
    CategoryInfo,
    client::{prompt_client, prompt_service_request, prompt_sidecar_error, sidecar_prompt_list},
};

pub async fn prompt_list_categories(state: &AppState) -> Result<Vec<CategoryInfo>, String> {
    if let Some(client) = prompt_client(state)? {
        let response = client
            .categories(prompt_service_request(None, None, serde_json::json!({}))?)
            .await
            .map_err(prompt_sidecar_error)?;
        return crate::sidecar_clients::response_payload(response, "prompt");
    }
    let registry = state.engine.registry();
    let registry = registry.read();
    let mut categories = Vec::with_capacity(registry.categories().len());
    for (key, value) in registry.categories() {
        categories.push(CategoryInfo {
            key: key.clone(),
            label: value.label.clone(),
            order: value
                .order
                .ok_or_else(|| format!("提示词分类 {key} 缺少 order"))?,
            description: value.description.clone(),
        });
    }
    categories.sort_by_key(|category| (category.order, category.key.clone()));
    Ok(categories)
}

pub(in crate::actions::prompt_records) async fn active_prompt_names_for_state(
    state: &AppState,
) -> Result<HashSet<String>, String> {
    let entries = match prompt_client(state)? {
        Some(_) => sidecar_prompt_list(state).await?,
        None => state.engine.list_all(),
    };
    Ok(entries.into_iter().map(|entry| entry.name).collect())
}
