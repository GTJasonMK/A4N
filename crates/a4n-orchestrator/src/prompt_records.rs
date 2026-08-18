use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

pub use a4n_api_contract::prompt::{
    AbRenderResult, CategoryInfo, ProjectPromptOverrideSummary, PromptEntrySummary,
    PromptVersionSummary, RemoveOverrideInput, RenderInput, SetOverrideInput, UpdatePromptInput,
    ValidateVarsInput, VersionDiffInput,
};
use a4n_capabilities::{PromptClient, sidecar_response_payload};
use a4n_core::error::AppError;
use a4n_prompt::{
    contracts::{
        CountOutput, PromptBatchUpdate, PromptEmptyPayload, PromptHydrateOverride,
        PromptHydrateOverridesPayload, PromptListOutput, PromptNamePayload, PromptRenderAbOutput,
        PromptRenderOutput, PromptRenderPayload, PromptSetProjectOverridePayload,
        PromptUpdateManyPayload, PromptUpdatePayload, PromptValidateOutput, PromptValidatePayload,
        PromptVersionDiffOutput, PromptVersionDiffPayload, PromptVersionHistoryOutput,
    },
    engine::{PromptCategory, PromptEntry, PromptVersion},
};
use a4n_service_protocol::{
    PROMPT_RENDER_CAPABILITY_VERSION, ServiceRequest, ServiceRequestContext, ServiceResponse,
    TraceContext,
};
use serde_json::Value;
use uuid::Uuid;

use crate::{OrchestratorContext, collaboration::ProjectAccessRole};
use a4n_db::prompt_override::{
    ProjectOverrideIdentity, normalize_project_override_identity, validate_project_override_content,
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
                    PromptEmptyPayload::default(),
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
        user_id: i64,
        input: UpdatePromptInput,
    ) -> Result<PromptEntrySummary, AppError> {
        let UpdatePromptInput { name, content } = input;
        let author = self.auth_user_by_id(user_id).await?.username;
        let entry = if let Some(client) = self.prompt_client_for_execution(context)? {
            let response = client
                .update(prompt_service_request(
                    context,
                    None,
                    None,
                    PromptUpdatePayload {
                        name,
                        content,
                        author,
                    },
                )?)
                .await
                .map_err(prompt_sidecar_error)?;
            prompt_response_payload(response)?
        } else {
            self.prompt_update_entry(&name, content, &author).await?
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
        prompt_response_payload(response)
    }

    pub async fn prompt_version_history_validated(
        &self,
        context: &PromptExecutionContext,
        name: &str,
    ) -> Result<Vec<PromptVersionSummary>, AppError> {
        if let Some(client) = self.prompt_client_for_execution(context)? {
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
            return Ok(
                prompt_response_payload::<PromptVersionHistoryOutput>(response)?
                    .versions
                    .into_iter()
                    .map(|version| PromptVersionSummary {
                        version: version.version,
                        timestamp: version.timestamp.to_string(),
                        author: version.author,
                        message: version.message,
                    })
                    .collect(),
            );
        }
        Ok(self
            .prompt_version_history(name)?
            .iter()
            .map(prompt_version_summary)
            .collect())
    }

    pub async fn render_prompt_record_validated(
        &self,
        context: &PromptExecutionContext,
        user_id: i64,
        input: RenderInput,
    ) -> Result<String, AppError> {
        let RenderInput {
            template_name,
            variables,
            project_id,
        } = input;
        if let Some(project_id) = project_id {
            self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
                .await?;
        }
        self.render_prompt_record_template_validated(context, &template_name, variables, project_id)
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
        input: RenderInput,
    ) -> Result<AbRenderResult, AppError> {
        let RenderInput {
            template_name,
            variables,
            project_id,
        } = input;
        if let Some(project_id) = project_id {
            self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
                .await?;
        }
        if let Some(client) = self.prompt_client_for_execution(context)? {
            let root_project_id = self
                .hydrate_prompt_project_overrides(context, client.clone(), project_id)
                .await?;
            let response = client
                .render_ab(prompt_service_request(
                    context,
                    project_id,
                    root_project_id,
                    PromptRenderPayload {
                        template_name,
                        variables,
                    },
                )?)
                .await
                .map_err(prompt_sidecar_error)?;
            let output: PromptRenderAbOutput = prompt_response_payload(response)?;
            return Ok(AbRenderResult {
                current: output.current,
                original: output.original,
            });
        }
        let (current, original) = self.prompt_render_ab(&template_name, &variables, project_id)?;
        Ok(AbRenderResult { current, original })
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
        Ok(self
            .project_override_summaries(project_id, &active_names)
            .await?
            .into_iter()
            .map(|item| ProjectPromptOverrideSummary {
                prompt_name: item.prompt_name,
                updated_at: item.updated_at,
            })
            .collect())
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

    pub async fn list_prompt_categories_validated(
        &self,
        context: &PromptExecutionContext,
    ) -> Result<Vec<CategoryInfo>, AppError> {
        let categories: Vec<PromptCategory> =
            if let Some(client) = self.prompt_client_for_execution(context)? {
                let response = client
                    .categories(prompt_service_request(
                        context,
                        None,
                        None,
                        PromptEmptyPayload::default(),
                    )?)
                    .await
                    .map_err(prompt_sidecar_error)?;
                prompt_response_payload(response)?
            } else {
                self.prompt_categories()?
            };
        Ok(categories
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
                PromptEmptyPayload::default(),
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
            .map(|item| PromptHydrateOverride {
                name: item.prompt_name,
                content: item.content,
            })
            .collect();
        let response = client
            .hydrate_project_overrides(prompt_service_request(
                context,
                Some(project_id),
                Some(root_project_id),
                PromptHydrateOverridesPayload { overrides },
            )?)
            .await
            .map_err(prompt_sidecar_error)?;
        prompt_response_payload(response)?;
        Ok(Some(root_project_id))
    }

    async fn set_prompt_override_for_execution(
        &self,
        context: &PromptExecutionContext,
        identity: &ProjectOverrideIdentity,
        content: String,
    ) -> Result<(), AppError> {
        let Some(client) = self.prompt_client_for_execution(context)? else {
            return self.prompt_set_project_override(identity.project_id, &identity.name, content);
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
                PromptSetProjectOverridePayload {
                    name: identity.name.clone(),
                    content,
                },
            )?)
            .await
            .map_err(prompt_sidecar_error)?;
        prompt_response_payload(response)
    }

    async fn remove_prompt_override_for_execution(
        &self,
        context: &PromptExecutionContext,
        identity: &ProjectOverrideIdentity,
    ) -> Result<(), AppError> {
        let Some(client) = self.prompt_client_for_execution(context)? else {
            return self.prompt_remove_project_override(identity.project_id, &identity.name);
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
                PromptNamePayload {
                    name: identity.name.clone(),
                },
            )?)
            .await
            .map_err(prompt_sidecar_error)?;
        prompt_response_payload(response)
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

fn prompt_service_request<T>(
    context: &PromptExecutionContext,
    project_id: Option<i64>,
    root_project_id: Option<i64>,
    payload: T,
) -> Result<ServiceRequest<T>, AppError> {
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
    let mut request = ServiceRequestContext::deny_all(
        request_id.clone(),
        TraceContext {
            trace_id: request_id,
            parent_span_id: None,
        },
    );
    request.user_id = context.user_id;
    request.project_id = project_id;
    request.root_project_id = root_project_id;
    request.runtime_config_snapshot = context.runtime_config_snapshot.clone();
    request
        .with_payload(PROMPT_RENDER_CAPABILITY_VERSION, payload)
        .map_err(prompt_sidecar_error)
}

fn prompt_response_payload<T>(response: ServiceResponse<T>) -> Result<T, AppError> {
    sidecar_response_payload(response, "prompt").map_err(prompt_sidecar_error)
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_service_protocol::{SecretForwardingPolicy, ServiceStatus};

    #[test]
    fn prompt_request_rejects_incomplete_project_scope() {
        let context = PromptExecutionContext::new(true, Some(7), None);
        assert!(
            prompt_service_request(&context, Some(42), None, PromptEmptyPayload::default())
                .unwrap_err()
                .to_string()
                .contains("缺少 root_project_id")
        );
        let request = prompt_service_request(
            &context,
            Some(42),
            Some(3),
            PromptSetProjectOverridePayload {
                name: "writing".to_string(),
                content: "content".to_string(),
            },
        )
        .unwrap();
        assert_eq!(request.project_id, Some(42));
        assert_eq!(request.root_project_id, Some(3));
        assert_eq!(request.secret_policy, SecretForwardingPolicy::deny_all());
    }

    #[test]
    fn prompt_response_rejects_inconsistent_status_payload_pairs() {
        let response = ServiceResponse {
            status: ServiceStatus::Ok,
            trace_id: Some("trace-1".to_string()),
            usage: None,
            artifact_refs: Vec::new(),
            payload: Some("payload".to_string()),
            error: Some(a4n_service_protocol::ServiceError::non_retryable(
                a4n_service_protocol::ServiceErrorCode::Internal,
                "unexpected",
            )),
        };
        assert!(
            prompt_response_payload(response)
                .unwrap_err()
                .to_string()
                .contains("同时携带 error")
        );
    }
}
