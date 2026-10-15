use std::collections::HashMap;

use a4n_api_contract::prompt::{
    AbRenderResult, CategoryInfo, ProjectPromptOverrideSummary, PromptAssemblyInput,
    PromptAssemblyOutput, PromptAssemblySummary, PromptEntry, PromptEntrySummary,
    PromptExecutionContext, PromptVersionSummary, RemoveOverrideInput, RenderInput,
    SavePromptAssemblyInput, SetOverrideInput, UpdatePromptInput, ValidateVarsInput,
    VersionDiffInput,
};
use a4n_error::AppError;
use a4n_prompt::rendering::PromptRenderer;
use async_trait::async_trait;
use serde_json::Value;

use crate::AppState;

pub async fn prompt_list(state: &AppState) -> Result<Vec<PromptEntrySummary>, String> {
    let context = prompt_context(state);
    state
        .prompt_read_application()
        .list_records(&context)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_assembly_list(state: &AppState) -> Result<Vec<PromptAssemblySummary>, String> {
    let context = prompt_context(state);
    state
        .prompt_read_application()
        .list_assembly_records(&context)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_assembly_get(
    state: &AppState,
    user_id: i64,
    input: PromptAssemblyInput,
) -> Result<PromptAssemblyOutput, String> {
    let context = prompt_context(state);
    state
        .prompt_read_application()
        .get_assembly_record(&context, user_id, input)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_assembly_save(
    state: &AppState,
    user_id: i64,
    input: SavePromptAssemblyInput,
) -> Result<(), String> {
    let context = prompt_context(state);
    state
        .prompt_mutation_application()
        .save_assembly_record(&context, user_id, input)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_assembly_reset(
    state: &AppState,
    user_id: i64,
    input: PromptAssemblyInput,
) -> Result<(), String> {
    let context = prompt_context(state);
    state
        .prompt_mutation_application()
        .reset_assembly_record(&context, user_id, input)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_load_defaults(state: &AppState) -> Result<usize, String> {
    let context = prompt_context(state);
    state
        .prompt_mutation_application()
        .load_defaults(&context)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_update(
    state: &AppState,
    user_id: i64,
    input: UpdatePromptInput,
) -> Result<PromptEntrySummary, String> {
    let context = prompt_context(state);
    state
        .prompt_mutation_application()
        .update_record(&context, user_id, input)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_reset(state: &AppState, name: &str) -> Result<PromptEntrySummary, String> {
    let context = prompt_context(state);
    state
        .prompt_mutation_application()
        .reset_record(&context, name)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_version_history(
    state: &AppState,
    name: &str,
) -> Result<Vec<PromptVersionSummary>, String> {
    let context = prompt_context(state);
    state
        .prompt_read_application()
        .version_history(&context, name)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_render(
    state: &AppState,
    user_id: i64,
    input: RenderInput,
) -> Result<String, String> {
    let context = prompt_context(state);
    state
        .prompt_read_application()
        .render_record(&context, user_id, input)
        .await
        .map_err(prompt_error)
}

pub(crate) async fn render_prompt_template(
    state: &AppState,
    template_name: &str,
    variables: HashMap<String, Value>,
    project_id: Option<i64>,
) -> Result<String, String> {
    let context = prompt_context(state);
    state
        .prompt_read_application()
        .render_template(&context, template_name, variables, project_id)
        .await
        .map_err(prompt_error)
}

pub(crate) async fn render_prompt_template_for_call(
    state: &AppState,
    call_key: &str,
    template_name: &str,
    variables: HashMap<String, Value>,
    project_id: Option<i64>,
) -> Result<String, String> {
    let context = prompt_context(state);
    state
        .prompt_read_application()
        .render_template_for_call(&context, call_key, template_name, variables, project_id)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_render_ab(
    state: &AppState,
    user_id: i64,
    input: RenderInput,
) -> Result<AbRenderResult, String> {
    let context = prompt_context(state);
    state
        .prompt_read_application()
        .render_ab(&context, user_id, input)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_set_project_override(
    state: &AppState,
    user_id: i64,
    input: SetOverrideInput,
) -> Result<(), String> {
    let context = prompt_context(state);
    state
        .prompt_mutation_application()
        .set_project_override(&context, user_id, input)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_remove_project_override(
    state: &AppState,
    user_id: i64,
    input: RemoveOverrideInput,
) -> Result<(), String> {
    let context = prompt_context(state);
    state
        .prompt_mutation_application()
        .remove_project_override(&context, user_id, input)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_list_project_overrides(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Vec<ProjectPromptOverrideSummary>, String> {
    let context = prompt_context(state);
    state
        .prompt_read_application()
        .list_project_overrides(&context, user_id, project_id)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_version_diff(
    state: &AppState,
    input: VersionDiffInput,
) -> Result<String, String> {
    let context = prompt_context(state);
    state
        .prompt_read_application()
        .version_diff(&context, input)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_validate_variables(
    state: &AppState,
    input: ValidateVarsInput,
) -> Result<Vec<String>, String> {
    let context = prompt_context(state);
    state
        .prompt_read_application()
        .validate_variables(&context, input)
        .await
        .map_err(prompt_error)
}

pub async fn prompt_list_categories(state: &AppState) -> Result<Vec<CategoryInfo>, String> {
    let context = prompt_context(state);
    state
        .prompt_read_application()
        .list_categories(&context)
        .await
        .map_err(prompt_error)
}

pub(crate) async fn prompt_get_runtime_entry(
    state: &AppState,
    name: &str,
) -> Result<PromptEntry, String> {
    let context = prompt_context(state);
    state
        .prompt_read_application()
        .runtime_entry(&context, name)
        .await
        .map_err(prompt_error)
}

pub(crate) async fn prompt_list_runtime_entries(
    state: &AppState,
) -> Result<Vec<PromptEntry>, String> {
    let context = prompt_context(state);
    state
        .prompt_read_application()
        .runtime_entries(&context)
        .await
        .map_err(prompt_error)
}

pub(crate) async fn prompt_update_many_runtime(
    state: &AppState,
    updates: Vec<(String, String)>,
    author: &str,
    overwrite: bool,
) -> Result<Vec<PromptEntry>, String> {
    let context = prompt_context(state);
    state
        .prompt_mutation_application()
        .update_many_runtime(&context, updates, author, overwrite)
        .await
        .map_err(prompt_error)
}

#[async_trait]
impl a4n_api_contract::prompt::PromptRenderPort for AppState {
    async fn render_prompt(
        &self,
        template_name: &str,
        variables: &HashMap<String, Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        render_prompt_template(self, template_name, variables.clone(), project_id)
            .await
            .map_err(|error| AppError::Internal(anyhow::anyhow!(error)))
    }

    async fn render_prompt_for_call(
        &self,
        call_key: &str,
        template_name: &str,
        variables: &HashMap<String, Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        render_prompt_template_for_call(
            self,
            call_key,
            template_name,
            variables.clone(),
            project_id,
        )
        .await
        .map_err(|error| AppError::Internal(anyhow::anyhow!(error)))
    }
}

#[async_trait]
impl PromptRenderer for AppState {
    async fn render_prompt(
        &self,
        template_name: &str,
        variables: &HashMap<String, Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        <Self as a4n_api_contract::prompt::PromptRenderPort>::render_prompt(
            self,
            template_name,
            variables,
            project_id,
        )
        .await
    }

    async fn render_prompt_for_call(
        &self,
        call_key: &str,
        template_name: &str,
        variables: &HashMap<String, Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        <Self as a4n_api_contract::prompt::PromptRenderPort>::render_prompt_for_call(
            self,
            call_key,
            template_name,
            variables,
            project_id,
        )
        .await
    }
}

pub(crate) fn prompt_context(state: &AppState) -> PromptExecutionContext {
    PromptExecutionContext::new(
        state.runtime_profile().uses_sidecars(),
        crate::runtime_config::current_user_id(),
        crate::runtime_config::redacted_current_snapshot_value(),
    )
}

fn prompt_error(error: AppError) -> String {
    error.to_string()
}
