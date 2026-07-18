use a4n_prompt::engine::PromptEntry;

use crate::AppState;

use super::super::{
    DependencyGraphOutput, PromptDependencyGraphOutput,
    client::{
        prompt_client, prompt_service_request, prompt_sidecar_error, sidecar_get_prompt,
        sidecar_prompt_list,
    },
};

pub async fn prompt_dependency_graph(state: &AppState) -> Result<DependencyGraphOutput, String> {
    let graph = if let Some(client) = prompt_client(state)? {
        let response = client
            .dependency_graph(prompt_service_request(None, None, serde_json::json!({}))?)
            .await
            .map_err(prompt_sidecar_error)?;
        let output: PromptDependencyGraphOutput =
            crate::sidecar_clients::response_payload(response, "prompt")?;
        output.graph
    } else {
        state.engine.dependency_graph()
    };
    Ok(DependencyGraphOutput {
        nodes: graph
            .nodes
            .iter()
            .map(|node| serde_json::json!({"name": node.name, "category": node.category}))
            .collect(),
        edges: graph
            .edges
            .iter()
            .map(|edge| serde_json::json!({"from": edge.from, "to": edge.to}))
            .collect(),
    })
}

pub async fn prompt_get_dependencies(state: &AppState, name: &str) -> Result<Vec<String>, String> {
    let entry = match prompt_client(state)? {
        Some(_) => sidecar_get_prompt(state, name).await?,
        None => state.engine.get(name).map_err(|err| err.to_string())?,
    };
    Ok(entry.dependencies)
}

pub async fn prompt_get_reverse_dependencies(
    state: &AppState,
    name: &str,
) -> Result<Vec<String>, String> {
    let entries = match prompt_client(state)? {
        Some(_) => sidecar_prompt_list(state).await?,
        None => state.engine.list_all(),
    };
    reverse_dependencies_for_entries(&entries, name)
}

pub(in crate::actions::prompt_records) fn reverse_dependencies_for_entries(
    entries: &[PromptEntry],
    name: &str,
) -> Result<Vec<String>, String> {
    if !entries.iter().any(|entry| entry.name == name) {
        return Err(format!("提示词不存在：{name}"));
    }
    Ok(entries
        .iter()
        .filter(|entry| {
            entry
                .dependencies
                .iter()
                .any(|dependency| dependency == name)
        })
        .map(|entry| entry.name.clone())
        .collect())
}
