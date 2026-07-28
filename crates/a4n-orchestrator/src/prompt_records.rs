use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use a4n_capabilities::{JsonRequest, JsonResponse, PromptClient};
use a4n_core::error::AppError;
use a4n_prompt::{
    engine::{DependencyGraph, PromptEntry, PromptVersion},
    variables::parse_template_variables,
};
use a4n_service_protocol::{
    PROMPT_RENDER_CAPABILITY_VERSION, SchemaVersion, SecretForwardingPolicy, ServiceRequest,
    ServiceStatus, TraceContext, sanitize_service_response, validate_service_request_context,
};
use serde::{Deserialize, Serialize, de::DeserializeOwned};
use serde_json::Value;
use uuid::Uuid;

use crate::{
    OrchestratorContext,
    collaboration::ProjectAccessRole,
    prompt_override::{
        ProjectOverrideIdentity, ProjectPromptOverrideSummary, normalize_project_override_identity,
        validate_project_override_content,
    },
};

#[derive(Debug, Clone)]
pub struct PromptExecutionContext {
    use_sidecar: bool,
    user_id: Option<i64>,
    runtime_config_snapshot: Option<Value>,
}

impl PromptExecutionContext {
    pub fn new(
        use_sidecar: bool,
        user_id: Option<i64>,
        runtime_config_snapshot: Option<Value>,
    ) -> Self {
        Self {
            use_sidecar,
            user_id,
            runtime_config_snapshot,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptEntrySummary {
    pub name: String,
    pub title: String,
    pub content: String,
    pub description: Option<String>,
    pub tags: Vec<String>,
    pub category: String,
    pub dependencies: Vec<String>,
    pub is_modified: bool,
    pub version: String,
    pub modified_at: String,
    #[serde(default)]
    pub used_by: Vec<String>,
    #[serde(default)]
    pub path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PromptVersionSummary {
    pub version: String,
    pub timestamp: String,
    pub author: String,
    pub message: String,
    pub content_snapshot: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DependencyGraphOutput {
    pub nodes: Vec<Value>,
    pub edges: Vec<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AbRenderResult {
    pub version_a: String,
    pub version_b: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RenderInput {
    pub template_name: String,
    pub variables: Option<Value>,
    pub project_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UpdatePromptInput {
    pub name: String,
    pub content: String,
    pub author: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetOverrideInput {
    pub project_id: i64,
    pub name: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemoveOverrideInput {
    pub project_id: i64,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct VersionDiffInput {
    pub name: String,
    pub version_a: String,
    pub version_b: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ValidateVarsInput {
    pub template_name: String,
    pub variables: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AbRenderInput {
    pub template_name: String,
    pub variables: Option<Value>,
    pub project_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CategoryInfo {
    pub key: String,
    pub label: Option<String>,
    pub order: i32,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct PromptBatchUpdate {
    name: String,
    content: String,
}

#[derive(Debug, Clone, Serialize)]
struct PromptUpdateManyPayload {
    updates: Vec<PromptBatchUpdate>,
    author: String,
}

#[derive(Debug, Clone, Serialize)]
struct PromptRenderPayload {
    template_name: String,
    variables: HashMap<String, Value>,
    project_id: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PromptRenderOutput {
    rendered: String,
}

#[derive(Debug, Clone, Serialize)]
struct PromptNamePayload {
    name: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PromptListOutput {
    prompts: Vec<PromptEntry>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PromptVersionHistoryOutput {
    versions: Vec<PromptVersion>,
}

#[derive(Debug, Clone, Serialize)]
struct PromptVersionDiffPayload {
    name: String,
    version_a: String,
    version_b: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PromptVersionDiffOutput {
    diff: String,
}

#[derive(Debug, Clone, Serialize)]
struct PromptValidatePayload {
    template_name: String,
    variables: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PromptValidateOutput {
    missing: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PromptRenderAbOutput {
    version_a: String,
    version_b: String,
}

#[derive(Debug, Clone, Serialize)]
struct PromptProjectOverridePayload {
    project_id: i64,
    name: String,
    content: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct PromptHydrateOverridesPayload {
    overrides: Vec<PromptHydrateOverride>,
}

#[derive(Debug, Clone, Serialize)]
struct PromptHydrateOverride {
    project_id: i64,
    name: String,
    content: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct CountOutput {
    count: usize,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct PromptDependencyGraphOutput {
    graph: DependencyGraph,
}

impl OrchestratorContext {
    pub async fn list_prompt_records_validated(
        &self,
        context: &PromptExecutionContext,
    ) -> Result<Vec<PromptEntrySummary>, AppError> {
        Ok(self
            .prompt_entries_for_execution(context)
            .await?
            .iter()
            .map(prompt_entry_summary)
            .collect())
    }

    pub async fn prompt_runtime_entry_validated(
        &self,
        context: &PromptExecutionContext,
        name: &str,
    ) -> Result<PromptEntry, AppError> {
        self.prompt_entry_for_execution(context, name).await
    }

    pub async fn prompt_runtime_entries_validated(
        &self,
        context: &PromptExecutionContext,
    ) -> Result<Vec<PromptEntry>, AppError> {
        self.prompt_entries_for_execution(context).await
    }

    pub async fn update_many_prompt_records_validated(
        &self,
        context: &PromptExecutionContext,
        updates: Vec<(String, String)>,
        author: &str,
    ) -> Result<Vec<PromptEntry>, AppError> {
        let Some(client) = self.prompt_client_for_execution(context)? else {
            return self.prompt_update_entries(updates, author).await;
        };
        let payload = PromptUpdateManyPayload {
            updates: updates
                .into_iter()
                .map(|(name, content)| PromptBatchUpdate { name, content })
                .collect(),
            author: author.to_string(),
        };
        let response = client
            .update_many(prompt_service_request(context, None, None, payload)?)
            .await
            .map_err(prompt_sidecar_error)?;
        prompt_response_payload(response)
    }

    pub async fn load_default_prompt_records_validated(
        &self,
        context: &PromptExecutionContext,
    ) -> Result<usize, AppError> {
        let existing_names = self.active_prompt_names(context).await?;
        self.preflight_project_override_keys_for_retired_cleanup()
            .await?;
        let loaded = if let Some(client) = self.prompt_client_for_execution(context)? {
            let response = client
                .load_defaults(prompt_service_request(
                    context,
                    None,
                    None,
                    serde_json::json!({}),
                )?)
                .await
                .map_err(prompt_sidecar_error)?;
            prompt_response_payload::<CountOutput>(response)?.count
        } else {
            self.prompt_load_defaults().await?
        };
        let active_names = self.active_prompt_names(context).await?;
        let retired_names = existing_names
            .difference(&active_names)
            .cloned()
            .collect::<HashSet<_>>();
        if !retired_names.is_empty()
            && let Err(error) = self
                .delete_project_overrides_for_prompt_names(&retired_names)
                .await
        {
            return Err(AppError::InvalidState {
                current: "prompt_defaults_reloaded".to_string(),
                operation: format!("清理退役提示词项目覆盖失败：{error}"),
            });
        }
        Ok(loaded)
    }

    pub async fn update_prompt_record_validated(
        &self,
        context: &PromptExecutionContext,
        input: UpdatePromptInput,
    ) -> Result<PromptEntrySummary, AppError> {
        let entry = if let Some(client) = self.prompt_client_for_execution(context)? {
            let response = client
                .update(prompt_service_request(context, None, None, input)?)
                .await
                .map_err(prompt_sidecar_error)?;
            prompt_response_payload(response)?
        } else {
            self.prompt_update_entry(&input.name, input.content, &input.author)
                .await?
        };
        Ok(prompt_entry_summary(&entry))
    }

    pub async fn reset_prompt_record_validated(
        &self,
        context: &PromptExecutionContext,
        name: &str,
    ) -> Result<(), AppError> {
        let Some(client) = self.prompt_client_for_execution(context)? else {
            return self.prompt_reset_entry(name).await;
        };
        let response = client
            .reset(prompt_service_request(
                context,
                None,
                None,
                PromptNamePayload {
                    name: name.to_string(),
                },
            )?)
            .await
            .map_err(prompt_sidecar_error)?;
        let _: Value = prompt_response_payload(response)?;
        Ok(())
    }

    pub async fn prompt_version_history_validated(
        &self,
        context: &PromptExecutionContext,
        name: &str,
    ) -> Result<Vec<PromptVersionSummary>, AppError> {
        let versions = if let Some(client) = self.prompt_client_for_execution(context)? {
            let response = client
                .version_history(prompt_service_request(
                    context,
                    None,
                    None,
                    PromptNamePayload {
                        name: name.to_string(),
                    },
                )?)
                .await
                .map_err(prompt_sidecar_error)?;
            prompt_response_payload::<PromptVersionHistoryOutput>(response)?.versions
        } else {
            self.prompt_version_history(name)?
        };
        Ok(versions.iter().map(prompt_version_summary).collect())
    }

    pub async fn render_prompt_record_validated(
        &self,
        context: &PromptExecutionContext,
        user_id: i64,
        input: RenderInput,
    ) -> Result<String, AppError> {
        if let Some(project_id) = input.project_id {
            self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
                .await?;
        }
        let variables = parse_template_variables(input.variables.as_ref())
            .map_err(AppError::InvalidArgument)?;
        self.render_prompt_record_template_validated(
            context,
            &input.template_name,
            variables,
            input.project_id,
        )
        .await
    }

    pub async fn render_prompt_record_template_validated(
        &self,
        context: &PromptExecutionContext,
        template_name: &str,
        variables: HashMap<String, Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        let Some(client) = self.prompt_client_for_execution(context)? else {
            return self.prompt_render_template(template_name, &variables, project_id);
        };
        let root_project_id = self
            .hydrate_prompt_project_overrides(context, client.clone(), project_id)
            .await?;
        let response = client
            .render(prompt_service_request(
                context,
                project_id,
                root_project_id,
                PromptRenderPayload {
                    template_name: template_name.to_string(),
                    variables,
                    project_id,
                },
            )?)
            .await
            .map_err(prompt_sidecar_error)?;
        Ok(prompt_response_payload::<PromptRenderOutput>(response)?.rendered)
    }

    pub async fn render_prompt_record_ab_validated(
        &self,
        context: &PromptExecutionContext,
        user_id: i64,
        input: AbRenderInput,
    ) -> Result<AbRenderResult, AppError> {
        if let Some(project_id) = input.project_id {
            self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
                .await?;
        }
        let variables = parse_template_variables(input.variables.as_ref())
            .map_err(AppError::InvalidArgument)?;
        if let Some(client) = self.prompt_client_for_execution(context)? {
            let root_project_id = self
                .hydrate_prompt_project_overrides(context, client.clone(), input.project_id)
                .await?;
            let response = client
                .render_ab(prompt_service_request(
                    context,
                    input.project_id,
                    root_project_id,
                    PromptRenderPayload {
                        template_name: input.template_name,
                        variables,
                        project_id: input.project_id,
                    },
                )?)
                .await
                .map_err(prompt_sidecar_error)?;
            let output: PromptRenderAbOutput = prompt_response_payload(response)?;
            return Ok(AbRenderResult {
                version_a: output.version_a,
                version_b: output.version_b,
            });
        }
        let (version_a, version_b) =
            self.prompt_render_ab(&input.template_name, &variables, input.project_id)?;
        Ok(AbRenderResult {
            version_a,
            version_b,
        })
    }

    pub async fn set_prompt_project_override_validated(
        &self,
        context: &PromptExecutionContext,
        user_id: i64,
        input: SetOverrideInput,
    ) -> Result<(), AppError> {
        self.verify_project_permission(input.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let identity = normalize_project_override_identity(input.project_id, &input.name)
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        validate_project_override_content(&input.content)
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        self.prompt_entry_for_execution(context, &identity.name)
            .await?;
        let previous_content = self
            .load_project_override_persisted_content(&identity)
            .await?;
        self.set_prompt_override_for_execution(context, &identity, input.content.clone())
            .await?;
        if let Err(error) = self
            .save_project_override(identity.project_id, &identity.name, &input.content)
            .await
        {
            return Err(self
                .prompt_override_persistence_error(
                    context,
                    &identity,
                    previous_content,
                    format!("提示词覆盖持久化失败：{error}"),
                    "已回滚运行时覆盖",
                    "运行时覆盖已更新但回滚失败：",
                )
                .await);
        }
        Ok(())
    }

    pub async fn remove_prompt_project_override_validated(
        &self,
        context: &PromptExecutionContext,
        user_id: i64,
        input: RemoveOverrideInput,
    ) -> Result<(), AppError> {
        self.verify_project_permission(input.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let identity = normalize_project_override_identity(input.project_id, &input.name)
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        let previous_content = self
            .load_project_override_persisted_content(&identity)
            .await?;
        self.remove_prompt_override_for_execution(context, &identity)
            .await?;
        if let Err(error) = self.delete_project_override(&identity).await {
            return Err(self
                .prompt_override_persistence_error(
                    context,
                    &identity,
                    previous_content,
                    format!("提示词覆盖删除持久化失败：{error}"),
                    "已恢复运行时覆盖",
                    "运行时覆盖已移除但恢复失败：",
                )
                .await);
        }
        Ok(())
    }

    pub async fn list_prompt_project_overrides_validated(
        &self,
        context: &PromptExecutionContext,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ProjectPromptOverrideSummary>, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let active_names = self.active_prompt_names(context).await?;
        self.project_override_summaries(project_id, &active_names)
            .await
    }

    pub async fn prompt_version_diff_validated(
        &self,
        context: &PromptExecutionContext,
        input: VersionDiffInput,
    ) -> Result<String, AppError> {
        if let Some(client) = self.prompt_client_for_execution(context)? {
            let response = client
                .diff(prompt_service_request(
                    context,
                    None,
                    None,
                    PromptVersionDiffPayload {
                        name: input.name,
                        version_a: input.version_a,
                        version_b: input.version_b,
                    },
                )?)
                .await
                .map_err(prompt_sidecar_error)?;
            return Ok(prompt_response_payload::<PromptVersionDiffOutput>(response)?.diff);
        }
        self.prompt_diff_versions(&input.name, &input.version_a, &input.version_b)
    }

    pub async fn validate_prompt_variables_validated(
        &self,
        context: &PromptExecutionContext,
        input: ValidateVarsInput,
    ) -> Result<Vec<String>, AppError> {
        if let Some(client) = self.prompt_client_for_execution(context)? {
            let response = client
                .validate(prompt_service_request(
                    context,
                    None,
                    None,
                    PromptValidatePayload {
                        template_name: input.template_name,
                        variables: input.variables,
                    },
                )?)
                .await
                .map_err(prompt_sidecar_error)?;
            return Ok(prompt_response_payload::<PromptValidateOutput>(response)?.missing);
        }
        self.prompt_validate_variables(
            &input.template_name,
            &input.variables.into_iter().collect::<HashSet<_>>(),
        )
    }

    pub async fn prompt_dependency_graph_validated(
        &self,
        context: &PromptExecutionContext,
    ) -> Result<DependencyGraphOutput, AppError> {
        let graph = if let Some(client) = self.prompt_client_for_execution(context)? {
            let response = client
                .dependency_graph(prompt_service_request(
                    context,
                    None,
                    None,
                    serde_json::json!({}),
                )?)
                .await
                .map_err(prompt_sidecar_error)?;
            prompt_response_payload::<PromptDependencyGraphOutput>(response)?.graph
        } else {
            self.prompt_dependency_graph()
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

    pub async fn prompt_dependencies_validated(
        &self,
        context: &PromptExecutionContext,
        name: &str,
    ) -> Result<Vec<String>, AppError> {
        Ok(self
            .prompt_entry_for_execution(context, name)
            .await?
            .dependencies)
    }

    pub async fn prompt_reverse_dependencies_validated(
        &self,
        context: &PromptExecutionContext,
        name: &str,
    ) -> Result<Vec<String>, AppError> {
        reverse_dependencies(&self.prompt_entries_for_execution(context).await?, name)
            .map_err(AppError::NotFound)
    }

    pub async fn list_prompt_categories_validated(
        &self,
        context: &PromptExecutionContext,
    ) -> Result<Vec<CategoryInfo>, AppError> {
        if let Some(client) = self.prompt_client_for_execution(context)? {
            let response = client
                .categories(prompt_service_request(
                    context,
                    None,
                    None,
                    serde_json::json!({}),
                )?)
                .await
                .map_err(prompt_sidecar_error)?;
            return prompt_response_payload(response);
        }
        Ok(self
            .prompt_registry_categories()?
            .into_iter()
            .map(|category| CategoryInfo {
                key: category.key,
                label: category.label,
                order: category.order,
                description: category.description,
            })
            .collect())
    }

    fn prompt_client_for_execution(
        &self,
        context: &PromptExecutionContext,
    ) -> Result<Option<Arc<dyn PromptClient>>, AppError> {
        if !context.use_sidecar {
            return Ok(None);
        }
        self.capabilities().prompt.clone().map(Some).ok_or_else(|| {
            AppError::ConfigError(
                "prompt sidecar capability 未配置，不能回退到 embedded prompt engine".to_string(),
            )
        })
    }

    async fn prompt_entries_for_execution(
        &self,
        context: &PromptExecutionContext,
    ) -> Result<Vec<PromptEntry>, AppError> {
        let Some(client) = self.prompt_client_for_execution(context)? else {
            return Ok(self.prompt_entries());
        };
        let response = client
            .list(prompt_service_request(
                context,
                None,
                None,
                serde_json::json!({}),
            )?)
            .await
            .map_err(prompt_sidecar_error)?;
        Ok(prompt_response_payload::<PromptListOutput>(response)?.prompts)
    }

    async fn prompt_entry_for_execution(
        &self,
        context: &PromptExecutionContext,
        name: &str,
    ) -> Result<PromptEntry, AppError> {
        let Some(client) = self.prompt_client_for_execution(context)? else {
            return self.prompt_entry(name);
        };
        let response = client
            .get(prompt_service_request(
                context,
                None,
                None,
                PromptNamePayload {
                    name: name.to_string(),
                },
            )?)
            .await
            .map_err(prompt_sidecar_error)?;
        prompt_response_payload(response)
    }

    async fn active_prompt_names(
        &self,
        context: &PromptExecutionContext,
    ) -> Result<HashSet<String>, AppError> {
        Ok(self
            .prompt_entries_for_execution(context)
            .await?
            .into_iter()
            .map(|entry| entry.name)
            .collect())
    }

    async fn hydrate_prompt_project_overrides(
        &self,
        context: &PromptExecutionContext,
        client: Arc<dyn PromptClient>,
        project_id: Option<i64>,
    ) -> Result<Option<i64>, AppError> {
        let Some(project_id) = project_id else {
            return Ok(None);
        };
        let root_project_id = self.collaboration_root_project(project_id).await?.id;
        let active_names = self.active_prompt_names(context).await?;
        let overrides = self
            .project_override_summaries(project_id, &active_names)
            .await?
            .into_iter()
            .map(|item| {
                let content = item
                    .content
                    .as_str()
                    .ok_or_else(|| AppError::InvalidState {
                        current: "prompt_override_record".to_string(),
                        operation: format!(
                            "提示词覆盖 value 必须是字符串：project_id={}, prompt={}",
                            item.project_id, item.prompt_name
                        ),
                    })?;
                Ok(PromptHydrateOverride {
                    project_id: item.project_id,
                    name: item.prompt_name,
                    content: content.to_string(),
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        let response = client
            .hydrate_project_overrides(prompt_service_request(
                context,
                Some(project_id),
                Some(root_project_id),
                PromptHydrateOverridesPayload { overrides },
            )?)
            .await
            .map_err(prompt_sidecar_error)?;
        let _: CountOutput = prompt_response_payload(response)?;
        Ok(Some(root_project_id))
    }

    async fn set_prompt_override_for_execution(
        &self,
        context: &PromptExecutionContext,
        identity: &ProjectOverrideIdentity,
        content: String,
    ) -> Result<(), AppError> {
        let Some(client) = self.prompt_client_for_execution(context)? else {
            return self
                .prompt_set_project_override(identity.project_id, &identity.name, content)
                .await;
        };
        let root_project_id = self
            .collaboration_root_project(identity.project_id)
            .await?
            .id;
        let response = client
            .set_project_override(prompt_service_request(
                context,
                Some(identity.project_id),
                Some(root_project_id),
                PromptProjectOverridePayload {
                    project_id: identity.project_id,
                    name: identity.name.clone(),
                    content: Some(content),
                },
            )?)
            .await
            .map_err(prompt_sidecar_error)?;
        let _: Value = prompt_response_payload(response)?;
        Ok(())
    }

    async fn remove_prompt_override_for_execution(
        &self,
        context: &PromptExecutionContext,
        identity: &ProjectOverrideIdentity,
    ) -> Result<(), AppError> {
        let Some(client) = self.prompt_client_for_execution(context)? else {
            return self
                .prompt_remove_project_override(identity.project_id, &identity.name)
                .await;
        };
        let root_project_id = self
            .collaboration_root_project(identity.project_id)
            .await?
            .id;
        let response = client
            .remove_project_override(prompt_service_request(
                context,
                Some(identity.project_id),
                Some(root_project_id),
                PromptProjectOverridePayload {
                    project_id: identity.project_id,
                    name: identity.name.clone(),
                    content: None,
                },
            )?)
            .await
            .map_err(prompt_sidecar_error)?;
        let _: Value = prompt_response_payload(response)?;
        Ok(())
    }

    async fn prompt_override_persistence_error(
        &self,
        context: &PromptExecutionContext,
        identity: &ProjectOverrideIdentity,
        previous_content: Option<String>,
        operation_error: String,
        rollback_success_message: &str,
        rollback_failure_prefix: &str,
    ) -> AppError {
        let rollback_result = match previous_content {
            Some(content) => {
                self.set_prompt_override_for_execution(context, identity, content)
                    .await
            }
            None => {
                self.remove_prompt_override_for_execution(context, identity)
                    .await
            }
        };
        let operation = match rollback_result {
            Ok(()) => format!("{operation_error}；{rollback_success_message}"),
            Err(error) => format!("{operation_error}；{rollback_failure_prefix}{error}"),
        };
        AppError::InvalidState {
            current: "prompt_override_persistence".to_string(),
            operation,
        }
    }
}

fn prompt_service_request(
    context: &PromptExecutionContext,
    project_id: Option<i64>,
    root_project_id: Option<i64>,
    payload: impl Serialize,
) -> Result<JsonRequest, AppError> {
    match (project_id, root_project_id) {
        (None, None) | (Some(_), Some(_)) => {}
        (Some(project_id), None) => {
            return Err(AppError::InvalidState {
                current: "prompt_sidecar_scope".to_string(),
                operation: format!("项目请求缺少 root_project_id：project_id={project_id}"),
            });
        }
        (None, Some(root_project_id)) => {
            return Err(AppError::InvalidState {
                current: "prompt_sidecar_scope".to_string(),
                operation: format!("全局请求不能单独携带 root_project_id={root_project_id}"),
            });
        }
    }
    let request_id = Uuid::new_v4().to_string();
    let request = ServiceRequest {
        request_id: request_id.clone(),
        trace_context: TraceContext {
            trace_id: request_id,
            parent_span_id: None,
        },
        schema_version: SchemaVersion::protocol_only(),
        user_id: context.user_id,
        project_id,
        root_project_id,
        runtime_config_snapshot: context.runtime_config_snapshot.clone(),
        secret_policy: SecretForwardingPolicy::deny_all(),
        capability_version: PROMPT_RENDER_CAPABILITY_VERSION.to_string(),
        payload: serde_json::to_value(payload)?,
    };
    validate_service_request_context(&request)
        .map_err(|error| AppError::ConfigError(error.message))?;
    Ok(request)
}

fn prompt_response_payload<T: DeserializeOwned>(mut response: JsonResponse) -> Result<T, AppError> {
    sanitize_service_response(&mut response).map_err(|error| {
        AppError::ConfigError(format!("prompt sidecar 返回非法 response：{error}"))
    })?;
    match response.status {
        ServiceStatus::Ok => serde_json::from_value(response.payload.ok_or_else(|| {
            AppError::ConfigError("prompt sidecar 返回成功但缺少 payload".to_string())
        })?)
        .map_err(|error| {
            AppError::ConfigError(format!("prompt sidecar 响应 payload 解析失败：{error}"))
        }),
        ServiceStatus::Error => Err(AppError::ConfigError(
            response
                .error
                .map(|error| error.message)
                .unwrap_or_else(|| "prompt sidecar 返回未知错误".to_string()),
        )),
    }
}

fn prompt_sidecar_error(error: a4n_service_protocol::ServiceError) -> AppError {
    AppError::ConfigError(error.message)
}

fn prompt_entry_summary(entry: &PromptEntry) -> PromptEntrySummary {
    PromptEntrySummary {
        name: entry.name.clone(),
        title: entry.title.clone(),
        content: entry.content.clone(),
        description: entry.description.clone(),
        tags: entry.tags.clone(),
        category: entry.category.clone(),
        dependencies: entry.dependencies.clone(),
        is_modified: entry.is_modified,
        version: entry.version.clone(),
        modified_at: entry.modified_at.to_string(),
        used_by: entry.used_by.clone(),
        path: entry.path.clone(),
    }
}

fn prompt_version_summary(version: &PromptVersion) -> PromptVersionSummary {
    PromptVersionSummary {
        version: version.version.clone(),
        timestamp: version.timestamp.to_string(),
        author: version.author.clone(),
        message: version.message.clone(),
        content_snapshot: version.content_snapshot.clone(),
    }
}

fn reverse_dependencies(entries: &[PromptEntry], name: &str) -> Result<Vec<String>, String> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn prompt_entry(name: &str, dependencies: &[&str]) -> PromptEntry {
        PromptEntry {
            name: name.to_string(),
            title: name.to_string(),
            content: name.to_string(),
            description: None,
            tags: Vec::new(),
            category: "writing".to_string(),
            dependencies: dependencies.iter().map(|value| value.to_string()).collect(),
            is_modified: false,
            version: "1.0".to_string(),
            modified_at: Utc::now().naive_utc(),
            used_by: Vec::new(),
            path: None,
        }
    }

    #[test]
    fn reverse_dependencies_require_existing_prompt_and_return_dependents() {
        let entries = vec![
            prompt_entry("outline", &["blueprint"]),
            prompt_entry("writing", &["outline"]),
            prompt_entry("blueprint", &[]),
        ];
        assert_eq!(
            reverse_dependencies(&entries, "outline").unwrap(),
            vec!["writing".to_string()]
        );
        assert!(
            reverse_dependencies(&entries, "missing")
                .unwrap_err()
                .contains("提示词不存在")
        );
    }

    #[test]
    fn prompt_request_rejects_incomplete_project_scope() {
        let context = PromptExecutionContext::new(true, Some(7), None);
        assert!(
            prompt_service_request(&context, Some(42), None, serde_json::json!({}))
                .unwrap_err()
                .to_string()
                .contains("缺少 root_project_id")
        );
        let request = prompt_service_request(
            &context,
            Some(42),
            Some(3),
            serde_json::json!({"project_id": 42}),
        )
        .unwrap();
        assert_eq!(request.project_id, Some(42));
        assert_eq!(request.root_project_id, Some(3));
        assert_eq!(request.secret_policy, SecretForwardingPolicy::deny_all());
    }
}
