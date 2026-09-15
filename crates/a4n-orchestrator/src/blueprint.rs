use std::collections::HashMap;

use a4n_api_contract::{
    blueprint::{
        PreviewBlueprintGenerationInput, PreviewBlueprintRefinementInput,
        StartBlueprintGenerationInput, StartBlueprintRefinementInput,
    },
    generation_tasks::{GenerationStartRequestMeta, GenerationTaskHandle},
};
use a4n_db_models::models::generation::workflow_run;
use a4n_domain::project::{
    BLUEPRINT_GENERATION_KIND, BLUEPRINT_REFINEMENT_WORKFLOW_KIND,
    blueprint_generation_input_fingerprint, blueprint_refinement_input_fingerprint,
    blueprint_stage_call_key, normalized_blueprint_refinement_focus,
    validate_blueprint_chapter_count,
};
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;
use a4n_prompt::blueprint_generation::BlueprintGenerationPreviewStep;
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;
use uuid::Uuid;

use crate::OrchestratorContext;
use a4n_api_contract::workflow::WorkflowResumeRecord;
use generation::{BLUEPRINT_GENERATION_STAGES, preview_generate_request};

mod generation;
mod json;
mod persistence;
mod refine;

/// Host-facing boundary for blueprint generation request previews.
#[async_trait::async_trait]
pub trait BlueprintGenerationPreviewApplicationPort: Send + Sync {
    async fn preview_generation(
        &self,
        user_id: i64,
        input: PreviewBlueprintGenerationInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<BlueprintGenerationPreviewStep, AppError>;
}

/// Host-facing boundary for blueprint refinement request previews.
#[async_trait::async_trait]
pub trait BlueprintRefinementPreviewApplicationPort: Send + Sync {
    async fn preview_refinement(
        &self,
        user_id: i64,
        input: PreviewBlueprintRefinementInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChatRequest, AppError>;
}

/// Host-facing boundary for blueprint generation workers.
///
/// Runtime owns worker scheduling and supplies the prompt renderer; blueprint
/// validation, persistence and terminal workflow state remain here.
#[async_trait::async_trait]
pub trait BlueprintGenerationApplicationPort: Send + Sync {
    fn generation_input_from_workflow_run(
        &self,
        run: &WorkflowResumeRecord,
    ) -> Result<StartBlueprintGenerationInput, AppError>;

    async fn start_generation(
        &self,
        user_id: i64,
        input: &StartBlueprintGenerationInput,
    ) -> Result<crate::GenerationTaskStart, crate::GenerationTaskAccessError>;

    async fn run_generation(
        &self,
        prompt_renderer: &dyn PromptRenderer,
        input: StartBlueprintGenerationInput,
        handle: GenerationTaskHandle,
    ) -> Result<(), String>;
}

/// Host-facing boundary for blueprint refinement workers.
#[async_trait::async_trait]
pub trait BlueprintRefinementApplicationPort: Send + Sync {
    fn refinement_input_from_workflow_run(
        &self,
        run: &WorkflowResumeRecord,
    ) -> Result<StartBlueprintRefinementInput, AppError>;

    async fn start_refinement(
        &self,
        user_id: i64,
        input: &StartBlueprintRefinementInput,
    ) -> Result<crate::GenerationTaskStart, crate::GenerationTaskAccessError>;

    async fn run_refinement(
        &self,
        prompt_renderer: &dyn PromptRenderer,
        input: StartBlueprintRefinementInput,
        handle: GenerationTaskHandle,
    ) -> Result<(), String>;
}

pub(crate) use json::validate_sparse_blueprint_sections;

impl OrchestratorContext {
    pub(crate) fn blueprint_generation_input_from_workflow_run(
        &self,
        run: &WorkflowResumeRecord,
    ) -> Result<StartBlueprintGenerationInput, AppError> {
        if run.workflow_kind != BLUEPRINT_GENERATION_KIND {
            return Err(AppError::InvalidArgument(format!(
                "工作流类型 {} 不是蓝图生成",
                run.workflow_kind
            )));
        }
        let fingerprint =
            run.input_fingerprint_json
                .as_ref()
                .ok_or_else(|| AppError::InvalidState {
                    current: "blueprint_generation_workflow_fingerprint_missing".to_string(),
                    operation: "恢复蓝图生成".to_string(),
                })?;
        let project_id = required_fingerprint_i64(fingerprint, "project_id")?;
        if run.project_id != Some(project_id) {
            return Err(AppError::InvalidState {
                current: "blueprint_generation_project_scope_mismatch".to_string(),
                operation: "恢复蓝图生成".to_string(),
            });
        }
        let title = required_fingerprint_string(fingerprint, "title")?;
        let genre = required_fingerprint_string(fingerprint, "genre")?;
        let target_length = i32::try_from(required_fingerprint_i64(fingerprint, "target_length")?)
            .map_err(|_| AppError::InvalidArgument("蓝图目标章节数超出范围".to_string()))?;
        validate_blueprint_chapter_count(target_length, "target_length")
            .map_err(AppError::InvalidArgument)?;
        let variables = required_fingerprint_variables(fingerprint)?;
        let expected = blueprint_generation_input_fingerprint(
            project_id,
            &title,
            &genre,
            target_length,
            &variables,
        );
        if fingerprint != &expected {
            return Err(AppError::InvalidState {
                current: "blueprint_generation_workflow_fingerprint_invalid".to_string(),
                operation: "恢复蓝图生成".to_string(),
            });
        }
        Ok(StartBlueprintGenerationInput {
            meta: new_resume_meta(),
            project_id,
            title,
            genre,
            target_length,
            variables: Some(serde_json::from_value(variables).map_err(|error| {
                AppError::InvalidState {
                    current: error.to_string(),
                    operation: "恢复蓝图生成变量".to_string(),
                }
            })?),
        })
    }

    pub(crate) fn blueprint_refinement_input_from_workflow_run(
        &self,
        run: &WorkflowResumeRecord,
    ) -> Result<StartBlueprintRefinementInput, AppError> {
        if run.workflow_kind != BLUEPRINT_REFINEMENT_WORKFLOW_KIND {
            return Err(AppError::InvalidArgument(format!(
                "工作流类型 {} 不是蓝图优化",
                run.workflow_kind
            )));
        }
        let fingerprint =
            run.input_fingerprint_json
                .as_ref()
                .ok_or_else(|| AppError::InvalidState {
                    current: "blueprint_refinement_workflow_fingerprint_missing".to_string(),
                    operation: "恢复蓝图优化".to_string(),
                })?;
        let project_id = required_fingerprint_i64(fingerprint, "project_id")?;
        if run.project_id != Some(project_id) {
            return Err(AppError::InvalidState {
                current: "blueprint_refinement_project_scope_mismatch".to_string(),
                operation: "恢复蓝图优化".to_string(),
            });
        }
        let focus = optional_fingerprint_string(fingerprint, "focus")?;
        let variables = required_fingerprint_variables(fingerprint)?;
        let expected =
            blueprint_refinement_input_fingerprint(project_id, focus.as_deref(), &variables);
        if fingerprint != &expected {
            return Err(AppError::InvalidState {
                current: "blueprint_refinement_workflow_fingerprint_invalid".to_string(),
                operation: "恢复蓝图优化".to_string(),
            });
        }
        Ok(StartBlueprintRefinementInput {
            meta: new_resume_meta(),
            project_id,
            focus,
            variables: Some(serde_json::from_value(variables).map_err(|error| {
                AppError::InvalidState {
                    current: error.to_string(),
                    operation: "恢复蓝图优化变量".to_string(),
                }
            })?),
        })
    }

    pub(crate) async fn preview_blueprint_generation_request_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        novel_title: &str,
        genre: &str,
        target_length: i32,
        prompt_renderer: &dyn PromptRenderer,
        variables: &HashMap<String, Value>,
    ) -> Result<BlueprintGenerationPreviewStep, AppError> {
        self.verify_project_permission(
            project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        preview_generate_request(
            project_id,
            novel_title,
            genre,
            target_length,
            prompt_renderer,
            variables,
        )
        .await
    }
}

#[async_trait::async_trait]
impl BlueprintGenerationPreviewApplicationPort for OrchestratorContext {
    async fn preview_generation(
        &self,
        user_id: i64,
        input: PreviewBlueprintGenerationInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<BlueprintGenerationPreviewStep, AppError> {
        let variables = input.variables.unwrap_or_default().into_iter().collect();
        self.preview_blueprint_generation_request_for_user_validated(
            user_id,
            input.project_id,
            &input.title,
            &input.genre,
            input.target_length,
            prompt_renderer,
            &variables,
        )
        .await
    }
}

#[async_trait::async_trait]
impl BlueprintRefinementPreviewApplicationPort for OrchestratorContext {
    async fn preview_refinement(
        &self,
        user_id: i64,
        input: PreviewBlueprintRefinementInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<ChatRequest, AppError> {
        let variables = input.variables.unwrap_or_default().into_iter().collect();
        self.preview_blueprint_refine_request_for_user_validated(
            user_id,
            input.project_id,
            input.focus.as_deref(),
            prompt_renderer,
            &variables,
        )
        .await
    }
}

#[async_trait::async_trait]
impl BlueprintGenerationApplicationPort for OrchestratorContext {
    fn generation_input_from_workflow_run(
        &self,
        run: &WorkflowResumeRecord,
    ) -> Result<StartBlueprintGenerationInput, AppError> {
        OrchestratorContext::blueprint_generation_input_from_workflow_run(self, run)
    }

    async fn start_generation(
        &self,
        user_id: i64,
        input: &StartBlueprintGenerationInput,
    ) -> Result<crate::GenerationTaskStart, crate::GenerationTaskAccessError> {
        OrchestratorContext::start_blueprint_generation_task(self, user_id, input).await
    }

    async fn run_generation(
        &self,
        prompt_renderer: &dyn PromptRenderer,
        input: StartBlueprintGenerationInput,
        handle: GenerationTaskHandle,
    ) -> Result<(), String> {
        OrchestratorContext::run_blueprint_generation_task(self, prompt_renderer, input, handle)
            .await
    }
}

#[async_trait::async_trait]
impl BlueprintRefinementApplicationPort for OrchestratorContext {
    fn refinement_input_from_workflow_run(
        &self,
        run: &WorkflowResumeRecord,
    ) -> Result<StartBlueprintRefinementInput, AppError> {
        OrchestratorContext::blueprint_refinement_input_from_workflow_run(self, run)
    }

    async fn start_refinement(
        &self,
        user_id: i64,
        input: &StartBlueprintRefinementInput,
    ) -> Result<crate::GenerationTaskStart, crate::GenerationTaskAccessError> {
        OrchestratorContext::start_blueprint_refinement_task(self, user_id, input).await
    }

    async fn run_refinement(
        &self,
        prompt_renderer: &dyn PromptRenderer,
        input: StartBlueprintRefinementInput,
        handle: GenerationTaskHandle,
    ) -> Result<(), String> {
        OrchestratorContext::run_blueprint_refinement_task(self, prompt_renderer, input, handle)
            .await
    }
}

pub(crate) fn blueprint_db_error(error: a4n_db::blueprint::BlueprintDbError) -> AppError {
    match error {
        a4n_db::blueprint::BlueprintDbError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        a4n_db::blueprint::BlueprintDbError::NotFound(message) => AppError::NotFound(message),
        a4n_db::blueprint::BlueprintDbError::Database(error) => AppError::Database(error),
    }
}

fn blueprint_sync_error(error: a4n_db::project::BlueprintSyncError) -> AppError {
    match error {
        a4n_db::project::BlueprintSyncError::InvalidArgument(message) => {
            AppError::InvalidArgument(message)
        }
        a4n_db::project::BlueprintSyncError::Database(error) => AppError::Database(error),
    }
}

fn character_profile_seed_error(error: a4n_db::character::CharacterProfileSeedError) -> AppError {
    match error {
        a4n_db::character::CharacterProfileSeedError::Database(error) => AppError::Database(error),
        a4n_db::character::CharacterProfileSeedError::Internal(message) => {
            AppError::Internal(anyhow::anyhow!(message))
        }
    }
}

use crate::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};

impl OrchestratorContext {
    /// 蓝图生成：校验后创建 workflow_run（RUNNING）。
    async fn begin_blueprint_generation_workflow_validated(
        &self,
        project_id: i64,
        title: &str,
        genre: &str,
        target_length: i32,
        variables: &serde_json::Value,
        task_id: Option<String>,
    ) -> Result<workflow_run::Model, AppError> {
        if project_id <= 0 {
            return Err(AppError::InvalidArgument("项目 ID 必须大于 0".to_string()));
        }
        let fingerprint = blueprint_generation_input_fingerprint(
            project_id,
            title,
            genre,
            target_length,
            variables,
        );
        let business_key = format!("{BLUEPRINT_GENERATION_KIND}:project:{project_id}");
        if let Some(task_id) = task_id.as_deref()
            && let Some(existing) = self
                .latest_workflow_run_by_business_key(&business_key)
                .await?
            && existing.status == WORKFLOW_STATUS_RUNNING
            && existing.active_task_id.as_deref() == Some(task_id)
        {
            if existing.workflow_kind != BLUEPRINT_GENERATION_KIND
                || existing.project_id != Some(project_id)
                || existing.input_fingerprint_json.as_ref() != Some(&fingerprint)
            {
                return Err(AppError::InvalidState {
                    current: "blueprint_generation_prebound_workflow_mismatch".to_string(),
                    operation: "启动已绑定的蓝图生成任务".to_string(),
                });
            }
            return Ok(existing);
        }
        self.begin_or_resume_workflow_run_validated(UpsertWorkflowRunInput {
            business_key,
            workflow_kind: BLUEPRINT_GENERATION_KIND.to_string(),
            project_id: Some(project_id),
            root_project_id: Some(project_id),
            source_snapshot_id: None,
            scope_table: Some("project".to_string()),
            scope_id: Some(project_id),
            chapter_id: None,
            mode: Some("generate".to_string()),
            input_fingerprint_json: Some(fingerprint),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some(blueprint_stage_call_key(BLUEPRINT_GENERATION_STAGES[0].key)),
            active_task_id: task_id,
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        })
        .await
    }
}

fn new_resume_meta() -> GenerationStartRequestMeta {
    GenerationStartRequestMeta {
        start_request_id: Uuid::new_v4().to_string(),
        retry_of_task_id: None,
    }
}

fn required_fingerprint_i64(value: &Value, key: &str) -> Result<i64, AppError> {
    value
        .get(key)
        .and_then(Value::as_i64)
        .filter(|value| *value > 0)
        .ok_or_else(|| AppError::InvalidState {
            current: format!("blueprint_workflow_fingerprint_missing_{key}"),
            operation: "恢复蓝图工作流".to_string(),
        })
}

fn required_fingerprint_string(value: &Value, key: &str) -> Result<String, AppError> {
    value
        .get(key)
        .and_then(Value::as_str)
        // The fingerprint is built from the exact request values. Trim only
        // for the non-empty check; changing the value here makes a valid
        // request impossible to reconstruct on resume.
        .filter(|value| !value.trim().is_empty())
        .map(ToString::to_string)
        .ok_or_else(|| AppError::InvalidState {
            current: format!("blueprint_workflow_fingerprint_missing_{key}"),
            operation: "恢复蓝图工作流".to_string(),
        })
}

fn optional_fingerprint_string(value: &Value, key: &str) -> Result<Option<String>, AppError> {
    match value.get(key) {
        Some(Value::Null) => Ok(None),
        Some(Value::String(value)) => {
            Ok(normalized_blueprint_refinement_focus(Some(value)).map(ToString::to_string))
        }
        _ => Err(AppError::InvalidState {
            current: format!("blueprint_workflow_fingerprint_invalid_{key}"),
            operation: "恢复蓝图工作流".to_string(),
        }),
    }
}

fn required_fingerprint_variables(value: &Value) -> Result<Value, AppError> {
    value
        .get("variables")
        .filter(|value| value.is_object())
        .cloned()
        .ok_or_else(|| AppError::InvalidState {
            current: "blueprint_workflow_fingerprint_invalid_variables".to_string(),
            operation: "恢复蓝图工作流".to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn required_fingerprint_string_preserves_request_value() {
        let fingerprint =
            blueprint_generation_input_fingerprint(7, "  雾城  ", "  悬疑  ", 20, &json!({}));

        assert_eq!(
            required_fingerprint_string(&fingerprint, "title").unwrap(),
            "  雾城  "
        );
        assert_eq!(
            required_fingerprint_string(&fingerprint, "genre").unwrap(),
            "  悬疑  "
        );
    }

    #[test]
    fn required_fingerprint_string_rejects_whitespace_only_value() {
        let fingerprint = json!({"title": " \n\t "});

        assert!(required_fingerprint_string(&fingerprint, "title").is_err());
    }
}
