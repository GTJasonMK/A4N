use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use a4n_api_contract::prompt::{
    AbRenderResult, CategoryInfo, ProjectPromptOverrideSummary, PromptAssemblyBlockKind,
    PromptAssemblyConfig, PromptAssemblyInput, PromptAssemblyOutput, PromptAssemblySnapshot,
    PromptAssemblySource, PromptAssemblySummary, PromptAssemblyTemplateOption, PromptEntrySummary,
    PromptExecutionContext, PromptMutationApplicationPort, PromptReadApplicationPort,
    PromptVersionSummary, RemoveOverrideInput, RenderInput, SavePromptAssemblyInput,
    SetOverrideInput, UpdatePromptInput, ValidateVarsInput, VersionDiffInput,
};
use a4n_error::AppError;
use a4n_prompt::engine::PromptVersion;
use a4n_prompt::rendering::PromptRenderer;
use a4n_prompt_client::PromptClient;
use a4n_prompt_contract::{
    CountOutput, PromptBatchUpdate, PromptCategory, PromptEmptyPayload, PromptEntry,
    PromptHydrateOverride, PromptHydrateOverridesPayload, PromptListOutput, PromptNamePayload,
    PromptRenderAbOutput, PromptRenderAbPayload, PromptRenderOutput, PromptRenderPayload,
    PromptSetProjectOverridePayload, PromptUpdateManyPayload, PromptUpdatePayload,
    PromptValidateOutput, PromptValidatePayload, PromptVersionDiffOutput, PromptVersionDiffPayload,
    PromptVersionHistoryOutput,
};
use a4n_service_client::sidecar_response_payload;
use a4n_service_protocol::{
    PROMPT_RENDER_CAPABILITY_VERSION, ServiceRequest, ServiceRequestContext, ServiceResponse,
    TraceContext,
};
use async_trait::async_trait;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::OrchestratorContext;
use a4n_db::prompt_override::{
    ProjectOverrideIdentity, normalize_project_override_identity, validate_project_override_content,
};
use a4n_domain::collaboration::ProjectAccessRole;

const PROJECT_CONTEXT_ROOT: &str = "project_info";
const PROJECT_CONTEXT_FIELDS: &[&str] = &[
    "id",
    "title",
    "description",
    "project_type",
    "project_kind",
    "source_sync_status",
    "status",
    "target_chapters",
    "current_chapter",
    "total_chapters",
    "chapters_per_part",
    "raw_blueprint",
    "import_analysis_progress",
    "export_config",
];

fn project_context_variables() -> Vec<String> {
    PROJECT_CONTEXT_FIELDS
        .iter()
        .map(|field| format!("{PROJECT_CONTEXT_ROOT}.{field}"))
        .collect()
}

fn variable_root(value: &str) -> &str {
    value.split(['.', '[']).next().unwrap_or(value)
}

fn project_context_access_path_available(path: &[String], project_available: bool) -> bool {
    let Some(root) = path.first() else {
        return true;
    };
    if root != PROJECT_CONTEXT_ROOT {
        return true;
    }
    project_available
        && (path.len() == 1
            || (path.len() == 2 && PROJECT_CONTEXT_FIELDS.contains(&path[1].as_str())))
}

fn project_context_variable_available(value: &str, project_available: bool) -> bool {
    project_available
        && (value == PROJECT_CONTEXT_ROOT
            || project_context_variables().iter().any(|key| key == value))
}

pub(crate) struct PromptRendererAdapter<'a>(
    pub(crate) &'a dyn a4n_api_contract::prompt::PromptRenderPort,
);
#[async_trait]
impl PromptRenderer for PromptRendererAdapter<'_> {
    async fn render_prompt(
        &self,
        template_name: &str,
        variables: &HashMap<String, Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        self.0
            .render_prompt(template_name, variables, project_id)
            .await
    }

    async fn render_prompt_for_call(
        &self,
        call_key: &str,
        template_name: &str,
        variables: &HashMap<String, Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        self.0
            .render_prompt_for_call(call_key, template_name, variables, project_id)
            .await
    }
}

impl OrchestratorContext {
    pub(crate) async fn list_prompt_records_validated(
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

    pub(crate) async fn prompt_runtime_entry_validated(
        &self,
        context: &PromptExecutionContext,
        name: &str,
    ) -> Result<PromptEntry, AppError> {
        self.prompt_entry_for_execution(context, name).await
    }

    pub(crate) async fn prompt_runtime_entries_validated(
        &self,
        context: &PromptExecutionContext,
    ) -> Result<Vec<PromptEntry>, AppError> {
        self.prompt_entries_for_execution(context).await
    }

    fn prompt_assembly_definition(
        &self,
        call_key: &str,
        prompt_name: &str,
        project_id: Option<i64>,
    ) -> Result<(PromptAssemblyConfig, Vec<String>), AppError> {
        self.prompt_entry(prompt_name)?;
        let generic_call_key = format!("prompt:{prompt_name}");
        if call_key != generic_call_key
            && !self
                .prompt_registry_assembly_call_keys(prompt_name)
                .iter()
                .any(|key| key == call_key)
        {
            return Err(AppError::InvalidArgument(format!(
                "提示词 {prompt_name} 未注册调用归属 {call_key}"
            )));
        }
        let variables = self.prompt_template_variables(prompt_name, project_id)?;
        let defaults = self.prompt_default_assembly(prompt_name);
        self.validate_prompt_assembly_config(&defaults, prompt_name, &variables, project_id)
            .map_err(|error| match error {
                AppError::InvalidArgument(reason) => {
                    AppError::ConfigError(format!("{prompt_name} 默认装配无效：{reason}"))
                }
                error => error,
            })?;
        Ok((defaults, variables))
    }

    fn validate_prompt_assembly_config(
        &self,
        config: &PromptAssemblyConfig,
        prompt_name: &str,
        variables: &[String],
        project_id: Option<i64>,
    ) -> Result<(), AppError> {
        config.validate().map_err(AppError::InvalidArgument)?;
        let project_context_available = project_id.is_some();
        let variable_available = |name: &str| {
            name == PROJECT_CONTEXT_ROOT && project_context_available
                || variables.iter().any(|variable| variable == name)
        };
        for block in &config.blocks {
            match block.kind {
                PromptAssemblyBlockKind::Template => {
                    let required = if block.value == prompt_name {
                        variables.to_vec()
                    } else {
                        self.prompt_template_variables(&block.value, project_id)?
                    };
                    let paths =
                        self.prompt_template_variable_access_paths(&block.value, project_id)?;
                    if !required.iter().all(|variable| variable_available(variable))
                        || !paths.iter().all(|path| {
                            project_context_access_path_available(path, project_context_available)
                        })
                    {
                        return Err(AppError::InvalidArgument(format!(
                            "当前调用不可用模板：{}",
                            block.value
                        )));
                    }
                }
                PromptAssemblyBlockKind::Variable => {
                    let root = variable_root(&block.value);
                    let available = if root == PROJECT_CONTEXT_ROOT {
                        project_context_variable_available(&block.value, project_context_available)
                    } else {
                        variable_available(root)
                    };
                    if !available {
                        return Err(AppError::InvalidArgument(format!(
                            "当前调用不可用变量：{}",
                            block.value
                        )));
                    }
                }
                PromptAssemblyBlockKind::Custom => {
                    let required = a4n_prompt::variables::required_template_variables(&block.value)
                        .map_err(|error| {
                            AppError::InvalidArgument(format!("自定义模块模板无效：{error}"))
                        })?;
                    if let Some(variable) = required.iter().find(|name| !variable_available(name)) {
                        return Err(AppError::InvalidArgument(format!(
                            "当前调用不可用变量：{variable}"
                        )));
                    }
                    let paths = a4n_prompt::variables::template_variable_access_paths(&block.value)
                        .map_err(|error| {
                            AppError::InvalidArgument(format!("自定义模块模板无效：{error}"))
                        })?;
                    if !paths.iter().all(|path| {
                        project_context_access_path_available(path, project_context_available)
                    }) {
                        return Err(AppError::InvalidArgument(
                            "自定义装配模块引用了不可用项目资料".into(),
                        ));
                    }
                }
            }
        }
        Ok(())
    }

    pub(crate) fn prompt_default_assembly_call_key(
        &self,
        prompt_name: &str,
    ) -> Result<String, AppError> {
        self.prompt_entry(prompt_name)?;
        Ok(format!("prompt:{prompt_name}"))
    }

    pub(crate) async fn resolve_prompt_assembly_snapshot(
        &self,
        call_key: &str,
        prompt_name: &str,
        project_id: Option<i64>,
    ) -> Result<PromptAssemblySnapshot, AppError> {
        self.resolve_prompt_assembly_snapshot_with_variables(call_key, prompt_name, project_id)
            .await
            .map(|(snapshot, _)| snapshot)
    }

    async fn resolve_prompt_assembly_snapshot_with_variables(
        &self,
        call_key: &str,
        prompt_name: &str,
        project_id: Option<i64>,
    ) -> Result<(PromptAssemblySnapshot, Vec<String>), AppError> {
        let (default, variables) =
            self.prompt_assembly_definition(call_key, prompt_name, project_id)?;
        let project_value = if let Some(project_id) = project_id {
            self.load_prompt_assembly_override(Some(project_id), call_key, prompt_name)
                .await?
        } else {
            None
        };
        if let Some(value) = project_value {
            let config = serde_json::from_value(value).map_err(|error| {
                AppError::InvalidArgument(format!("项目提示词装配配置无效：{error}"))
            })?;
            self.validate_prompt_assembly_config(&config, prompt_name, &variables, project_id)?;
            return Ok((
                PromptAssemblySnapshot {
                    call_key: call_key.into(),
                    prompt_name: prompt_name.into(),
                    config,
                    source: PromptAssemblySource::Project,
                },
                variables,
            ));
        }
        if let Some(value) = self
            .load_prompt_assembly_override(None, call_key, prompt_name)
            .await?
        {
            let config = serde_json::from_value(value).map_err(|error| {
                AppError::InvalidArgument(format!("全局提示词装配配置无效：{error}"))
            })?;
            self.validate_prompt_assembly_config(&config, prompt_name, &variables, project_id)?;
            return Ok((
                PromptAssemblySnapshot {
                    call_key: call_key.into(),
                    prompt_name: prompt_name.into(),
                    config,
                    source: PromptAssemblySource::Global,
                },
                variables,
            ));
        }
        Ok((
            PromptAssemblySnapshot {
                call_key: call_key.into(),
                prompt_name: prompt_name.into(),
                config: default,
                source: PromptAssemblySource::SystemDefault,
            },
            variables,
        ))
    }

    async fn prompt_project_context_value(
        &self,
        project_id: Option<i64>,
    ) -> Result<Value, AppError> {
        let Some(project_id) = project_id else {
            return Ok(json!({}));
        };
        let Some(project) = self.project_by_id(project_id).await? else {
            return Err(AppError::NotFound(format!("项目不存在: {project_id}")));
        };
        Ok(json!({
            "id": project.id,
            "title": project.title,
            "description": project.description,
            "project_type": project.project_type,
            "project_kind": project.project_kind,
            "source_sync_status": project.source_sync_status,
            "status": project.status,
            "target_chapters": project.target_chapters,
            "current_chapter": project.current_chapter,
            "total_chapters": project.total_chapters,
            "chapters_per_part": project.chapters_per_part,
            "raw_blueprint": project.raw_blueprint,
            "import_analysis_progress": project.import_analysis_progress,
            "export_config": project.export_config,
        }))
    }

    async fn prompt_variables_with_project_context(
        &self,
        snapshot: &PromptAssemblySnapshot,
        variables: &HashMap<String, Value>,
        project_id: Option<i64>,
    ) -> Result<HashMap<String, Value>, AppError> {
        let mut variables = variables.clone();
        if project_id.is_some() && self.prompt_assembly_uses_project_info(snapshot, project_id)? {
            variables.insert(
                PROJECT_CONTEXT_ROOT.to_string(),
                self.prompt_project_context_value(project_id).await?,
            );
        }
        Ok(variables)
    }

    fn prompt_assembly_uses_project_info(
        &self,
        snapshot: &PromptAssemblySnapshot,
        project_id: Option<i64>,
    ) -> Result<bool, AppError> {
        for block in snapshot.config.blocks.iter().filter(|block| block.enabled) {
            let uses_project_info = match block.kind {
                PromptAssemblyBlockKind::Template => self
                    .prompt_template_variables(&block.value, project_id)?
                    .iter()
                    .any(|variable| variable == PROJECT_CONTEXT_ROOT),
                PromptAssemblyBlockKind::Variable => {
                    variable_root(&block.value) == PROJECT_CONTEXT_ROOT
                }
                PromptAssemblyBlockKind::Custom => {
                    a4n_prompt::variables::required_template_variables(&block.value)
                        .map_err(|error| {
                            AppError::InvalidArgument(format!(
                                "自定义装配模块 {} 的模板无效：{error}",
                                block.id
                            ))
                        })?
                        .iter()
                        .any(|variable| variable == PROJECT_CONTEXT_ROOT)
                }
            };
            if uses_project_info {
                return Ok(true);
            }
        }
        Ok(false)
    }

    pub(crate) async fn render_prompt_for_call_validated(
        &self,
        context: &PromptExecutionContext,
        call_key: &str,
        template_name: &str,
        variables: &HashMap<String, Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        let snapshot = self
            .resolve_prompt_assembly_snapshot(call_key, template_name, project_id)
            .await?;
        let variables = self
            .prompt_variables_with_project_context(&snapshot, variables, project_id)
            .await?;
        self.render_prompt_snapshot_validated(context, &snapshot, &variables, project_id)
            .await
    }

    async fn render_prompt_snapshot_validated(
        &self,
        context: &PromptExecutionContext,
        snapshot: &PromptAssemblySnapshot,
        variables: &HashMap<String, Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        let Some(client) = self.prompt_client_for_execution(context)? else {
            return self.prompt_render_assembled(snapshot, variables, project_id);
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
                    template_name: snapshot.prompt_name.clone(),
                    variables: variables.clone(),
                    assembly: snapshot.clone(),
                },
            )?)
            .await
            .map_err(prompt_sidecar_error)?;
        Ok(prompt_response_payload::<PromptRenderOutput>(response)?.rendered)
    }

    pub(crate) async fn list_prompt_assembly_summaries(
        &self,
    ) -> Result<Vec<PromptAssemblySummary>, AppError> {
        let prompts = self.prompt_entries();
        let mut prompt_data = HashMap::with_capacity(prompts.len());
        for prompt in &prompts {
            let mut call_keys = self.prompt_registry_assembly_call_keys(&prompt.name);
            let generic_call_key = format!("prompt:{}", prompt.name);
            if !call_keys.contains(&generic_call_key) {
                call_keys.push(generic_call_key);
            }
            let Some(call_key) = call_keys.first() else {
                continue;
            };
            let (_, variables) = self.prompt_assembly_definition(call_key, &prompt.name, None)?;
            prompt_data.insert(prompt.name.clone(), (call_keys, variables));
        }

        let mut summaries = Vec::new();
        for prompt in &prompts {
            let Some((call_keys, variables)) = prompt_data.get(&prompt.name) else {
                continue;
            };
            let available_templates = prompts
                .iter()
                .filter_map(|entry| {
                    let (_, required) = prompt_data.get(&entry.name)?;
                    required
                        .iter()
                        .all(|variable| variables.contains(variable))
                        .then(|| PromptAssemblyTemplateOption {
                            name: entry.name.clone(),
                            title: entry.title.clone(),
                        })
                })
                .collect::<Vec<_>>();
            for call_key in call_keys {
                summaries.push(PromptAssemblySummary {
                    call_key: call_key.clone(),
                    prompt_name: prompt.name.clone(),
                    variables: variables.clone(),
                    project_context_variables: project_context_variables(),
                    available_templates: available_templates.clone(),
                });
            }
        }
        Ok(summaries)
    }

    pub(crate) async fn get_prompt_assembly_output(
        &self,
        context: &PromptExecutionContext,
        user_id: i64,
        input: PromptAssemblyInput,
    ) -> Result<PromptAssemblyOutput, AppError> {
        let PromptAssemblyInput {
            call_key,
            prompt_name,
            project_id,
            preview_config,
        } = input;
        if let Some(project_id) = project_id {
            self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
                .await?;
        }
        let (snapshot, variables) = self
            .resolve_prompt_assembly_snapshot_with_variables(&call_key, &prompt_name, project_id)
            .await?;
        let preview_config = preview_config.unwrap_or_else(|| snapshot.config.clone());
        if preview_config != snapshot.config {
            self.validate_prompt_assembly_config(
                &preview_config,
                &prompt_name,
                &variables,
                project_id,
            )?;
        }
        let preview_snapshot = PromptAssemblySnapshot {
            config: preview_config,
            ..snapshot.clone()
        };
        let project_context = self.prompt_project_context_value(project_id).await?;
        let preview_variables = self.prompt_assembly_preview_variables(
            &preview_snapshot.config,
            &variables,
            project_id,
            &project_context,
        )?;
        let rendered_preview = self
            .render_prompt_snapshot_validated(
                context,
                &preview_snapshot,
                &preview_variables,
                project_id,
            )
            .await?;
        Ok(PromptAssemblyOutput {
            config: snapshot.config,
            source: snapshot.source,
            project_context,
            rendered_preview,
        })
    }

    fn prompt_assembly_preview_variables(
        &self,
        config: &PromptAssemblyConfig,
        variables: &[String],
        project_id: Option<i64>,
        project_context: &Value,
    ) -> Result<HashMap<String, Value>, AppError> {
        let mut access_paths = Vec::new();
        for block in config.blocks.iter().filter(|block| block.enabled) {
            let paths = match block.kind {
                PromptAssemblyBlockKind::Template => {
                    self.prompt_template_variable_access_paths(&block.value, project_id)?
                }
                PromptAssemblyBlockKind::Variable => continue,
                PromptAssemblyBlockKind::Custom => {
                    a4n_prompt::variables::template_variable_access_paths(&block.value).map_err(
                        |error| {
                            AppError::InvalidArgument(format!(
                                "自定义装配模块 {} 的模板无效：{error}",
                                block.id
                            ))
                        },
                    )?
                }
            };
            access_paths.extend(paths);
        }
        let mut preview =
            a4n_prompt::variables::placeholder_template_variables(variables, &access_paths);
        if project_id.is_some() {
            preview.insert(PROJECT_CONTEXT_ROOT.to_string(), project_context.clone());
        }
        Ok(preview)
    }

    pub(crate) async fn save_prompt_assembly_config(
        &self,
        user_id: i64,
        input: SavePromptAssemblyInput,
    ) -> Result<(), AppError> {
        if let Some(project_id) = input.project_id {
            self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
                .await?;
        }
        let (_, variables) =
            self.prompt_assembly_definition(&input.call_key, &input.prompt_name, input.project_id)?;
        self.validate_prompt_assembly_config(
            &input.config,
            &input.prompt_name,
            &variables,
            input.project_id,
        )?;
        self.save_prompt_assembly_override(
            input.project_id,
            &input.call_key,
            &input.prompt_name,
            serde_json::to_value(input.config).map_err(AppError::Serialization)?,
        )
        .await
    }

    pub(crate) async fn reset_prompt_assembly_config(
        &self,
        user_id: i64,
        input: PromptAssemblyInput,
    ) -> Result<(), AppError> {
        if let Some(project_id) = input.project_id {
            self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
                .await?;
        }
        self.prompt_assembly_definition(&input.call_key, &input.prompt_name, input.project_id)?;
        self.delete_prompt_assembly_override(input.project_id, &input.call_key, &input.prompt_name)
            .await
    }

    pub(crate) async fn update_many_prompt_records_validated(
        &self,
        context: &PromptExecutionContext,
        updates: Vec<(String, String)>,
        author: &str,
        overwrite: bool,
    ) -> Result<Vec<PromptEntry>, AppError> {
        let Some(client) = self.prompt_client_for_execution(context)? else {
            return self.prompt_update_entries(updates, author, overwrite).await;
        };
        let payload = PromptUpdateManyPayload {
            updates: updates
                .into_iter()
                .map(|(name, content)| PromptBatchUpdate { name, content })
                .collect(),
            author: author.to_string(),
            overwrite,
        };
        let response = client
            .update_many(prompt_service_request(context, None, None, payload)?)
            .await
            .map_err(prompt_sidecar_error)?;
        prompt_response_payload(response)
    }

    pub(crate) async fn load_default_prompt_records_validated(
        &self,
        context: &PromptExecutionContext,
    ) -> Result<usize, AppError> {
        let (existing_names_result, preflight_result) = futures::join!(
            self.active_prompt_names(context),
            self.preflight_project_override_keys_for_retired_cleanup(),
        );
        let existing_names = existing_names_result?;
        preflight_result?;
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

    pub(crate) async fn update_prompt_record_validated(
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

    pub(crate) async fn reset_prompt_record_validated(
        &self,
        context: &PromptExecutionContext,
        name: &str,
    ) -> Result<PromptEntrySummary, AppError> {
        let Some(client) = self.prompt_client_for_execution(context)? else {
            let entry = self.prompt_reset_entry(name).await?;
            return Ok(prompt_entry_summary(&entry));
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
        let entry = prompt_response_payload(response)?;
        Ok(prompt_entry_summary(&entry))
    }

    pub(crate) async fn prompt_version_history_validated(
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
                        timestamp: version.timestamp.and_utc().to_rfc3339(),
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

    pub(crate) async fn render_prompt_record_validated(
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

    pub(crate) async fn render_prompt_record_template_validated(
        &self,
        context: &PromptExecutionContext,
        template_name: &str,
        variables: HashMap<String, Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        let call_key = self.prompt_default_assembly_call_key(template_name)?;
        self.render_prompt_for_call_validated(
            context,
            &call_key,
            template_name,
            &variables,
            project_id,
        )
        .await
    }

    pub(crate) async fn render_prompt_record_ab_validated(
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
                    PromptRenderAbPayload {
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

    pub(crate) async fn set_prompt_project_override_validated(
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
        let (entry_result, previous_result) = futures::join!(
            self.prompt_entry_for_execution(context, &identity.name),
            self.load_project_override_persisted_content(&identity),
        );
        entry_result?;
        let previous_content = previous_result?;
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

    pub(crate) async fn remove_prompt_project_override_validated(
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

    pub(crate) async fn list_prompt_project_overrides_validated(
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

    pub(crate) async fn prompt_version_diff_validated(
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

    pub(crate) async fn validate_prompt_variables_validated(
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

    pub(crate) async fn list_prompt_categories_validated(
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
        if !context.use_sidecar() {
            return Ok(None);
        }
        self.prompt_client().map(Some).ok_or_else(|| {
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
        let (root_project_result, active_names_result) = futures::join!(
            self.collaboration_root_project(project_id),
            self.active_prompt_names(context),
        );
        let root_project_id = root_project_result?.id;
        let active_names = active_names_result?;
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

#[async_trait]
impl PromptReadApplicationPort for OrchestratorContext {
    async fn list_assembly_records(
        &self,
        _context: &PromptExecutionContext,
    ) -> Result<Vec<PromptAssemblySummary>, AppError> {
        self.list_prompt_assembly_summaries().await
    }

    async fn get_assembly_record(
        &self,
        context: &PromptExecutionContext,
        user_id: i64,
        input: PromptAssemblyInput,
    ) -> Result<PromptAssemblyOutput, AppError> {
        self.get_prompt_assembly_output(context, user_id, input)
            .await
    }

    async fn resolve_assembly_record(
        &self,
        _context: &PromptExecutionContext,
        call_key: &str,
        prompt_name: &str,
        project_id: Option<i64>,
    ) -> Result<PromptAssemblySnapshot, AppError> {
        self.resolve_prompt_assembly_snapshot(call_key, prompt_name, project_id)
            .await
    }

    async fn list_records(
        &self,
        context: &PromptExecutionContext,
    ) -> Result<Vec<PromptEntrySummary>, AppError> {
        self.list_prompt_records_validated(context).await
    }

    async fn version_history(
        &self,
        context: &PromptExecutionContext,
        name: &str,
    ) -> Result<Vec<PromptVersionSummary>, AppError> {
        self.prompt_version_history_validated(context, name).await
    }

    async fn render_record(
        &self,
        context: &PromptExecutionContext,
        user_id: i64,
        input: RenderInput,
    ) -> Result<String, AppError> {
        self.render_prompt_record_validated(context, user_id, input)
            .await
    }

    async fn render_template(
        &self,
        context: &PromptExecutionContext,
        template_name: &str,
        variables: HashMap<String, Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        let call_key = self.prompt_default_assembly_call_key(template_name)?;
        self.render_prompt_for_call_validated(
            context,
            &call_key,
            template_name,
            &variables,
            project_id,
        )
        .await
    }

    async fn render_template_for_call(
        &self,
        context: &PromptExecutionContext,
        call_key: &str,
        template_name: &str,
        variables: HashMap<String, Value>,
        project_id: Option<i64>,
    ) -> Result<String, AppError> {
        self.render_prompt_for_call_validated(
            context,
            call_key,
            template_name,
            &variables,
            project_id,
        )
        .await
    }

    async fn render_ab(
        &self,
        context: &PromptExecutionContext,
        user_id: i64,
        input: RenderInput,
    ) -> Result<AbRenderResult, AppError> {
        self.render_prompt_record_ab_validated(context, user_id, input)
            .await
    }

    async fn list_project_overrides(
        &self,
        context: &PromptExecutionContext,
        user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ProjectPromptOverrideSummary>, AppError> {
        self.list_prompt_project_overrides_validated(context, user_id, project_id)
            .await
    }

    async fn version_diff(
        &self,
        context: &PromptExecutionContext,
        input: VersionDiffInput,
    ) -> Result<String, AppError> {
        self.prompt_version_diff_validated(context, input).await
    }

    async fn validate_variables(
        &self,
        context: &PromptExecutionContext,
        input: ValidateVarsInput,
    ) -> Result<Vec<String>, AppError> {
        self.validate_prompt_variables_validated(context, input)
            .await
    }

    async fn list_categories(
        &self,
        context: &PromptExecutionContext,
    ) -> Result<Vec<CategoryInfo>, AppError> {
        self.list_prompt_categories_validated(context).await
    }

    async fn runtime_entry(
        &self,
        context: &PromptExecutionContext,
        name: &str,
    ) -> Result<PromptEntry, AppError> {
        self.prompt_runtime_entry_validated(context, name).await
    }

    async fn runtime_entries(
        &self,
        context: &PromptExecutionContext,
    ) -> Result<Vec<PromptEntry>, AppError> {
        self.prompt_runtime_entries_validated(context).await
    }
}

#[async_trait]
impl PromptMutationApplicationPort for OrchestratorContext {
    async fn save_assembly_record(
        &self,
        _context: &PromptExecutionContext,
        user_id: i64,
        input: SavePromptAssemblyInput,
    ) -> Result<(), AppError> {
        self.save_prompt_assembly_config(user_id, input).await
    }

    async fn reset_assembly_record(
        &self,
        _context: &PromptExecutionContext,
        user_id: i64,
        input: PromptAssemblyInput,
    ) -> Result<(), AppError> {
        self.reset_prompt_assembly_config(user_id, input).await
    }

    async fn hydrate_overrides(&self) -> Result<(), AppError> {
        self.hydrate_prompt_overrides().await
    }

    async fn load_defaults(&self, context: &PromptExecutionContext) -> Result<usize, AppError> {
        self.load_default_prompt_records_validated(context).await
    }

    async fn update_record(
        &self,
        context: &PromptExecutionContext,
        user_id: i64,
        input: UpdatePromptInput,
    ) -> Result<PromptEntrySummary, AppError> {
        self.update_prompt_record_validated(context, user_id, input)
            .await
    }

    async fn reset_record(
        &self,
        context: &PromptExecutionContext,
        name: &str,
    ) -> Result<PromptEntrySummary, AppError> {
        self.reset_prompt_record_validated(context, name).await
    }

    async fn set_project_override(
        &self,
        context: &PromptExecutionContext,
        user_id: i64,
        input: SetOverrideInput,
    ) -> Result<(), AppError> {
        self.set_prompt_project_override_validated(context, user_id, input)
            .await
    }

    async fn remove_project_override(
        &self,
        context: &PromptExecutionContext,
        user_id: i64,
        input: RemoveOverrideInput,
    ) -> Result<(), AppError> {
        self.remove_prompt_project_override_validated(context, user_id, input)
            .await
    }

    async fn update_many_runtime(
        &self,
        context: &PromptExecutionContext,
        updates: Vec<(String, String)>,
        author: &str,
        overwrite: bool,
    ) -> Result<Vec<PromptEntry>, AppError> {
        self.update_many_prompt_records_validated(context, updates, author, overwrite)
            .await
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
    request.user_id = context.user_id();
    request.project_id = project_id;
    request.root_project_id = root_project_id;
    request.runtime_config_snapshot = context.runtime_config_snapshot().cloned();
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
        timestamp: version.timestamp.and_utc().to_rfc3339(),
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

    #[test]
    fn project_context_access_paths_require_known_fields_and_project_scope() {
        assert!(project_context_access_path_available(
            &["project_info".into(), "title".into()],
            true,
        ));
        assert!(!project_context_access_path_available(
            &["project_info".into(), "missing".into()],
            true,
        ));
        assert!(!project_context_access_path_available(
            &["project_info".into(), "title".into()],
            false,
        ));
        assert!(project_context_access_path_available(
            &["chapter".into(), "title".into()],
            false,
        ));
        assert!(!project_context_variable_available("project_info[0]", true));
        assert!(!project_context_variable_available(
            "project_info.missing",
            true
        ));
        assert!(project_context_variable_available(
            "project_info.title",
            true
        ));
        assert!(!project_context_variable_available(
            "project_info.title",
            false
        ));
    }
}
