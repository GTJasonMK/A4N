use std::collections::HashMap;

use a4n_api_contract::character::{CharacterConflictCheckInput, CharacterConflictSummary};
use a4n_domain::project::required_text;
use a4n_error::AppError;
use a4n_llm::provider::is_truncated_finish_reason;
use a4n_prompt::rendering::PromptRenderer;
use serde_json::{Value, json};

use crate::{
    OrchestratorContext,
    character::{
        conflict_context_rules::{
            CharacterConflictChapterScope, attribute_event_context,
            character_conflict_allowed_source_ids, character_conflict_chapter_context,
            character_state_context, ensure_character_conflict_context_size,
            ensure_character_conflict_has_comparison_context,
        },
        conflict_output_rules::{
            parse_conflict_output_with_source_ids, restored_character_conflict_checkpoint,
        },
        conflict_rules::character_conflict_call_fingerprint_json,
    },
    workflow::{self, CompletedLlmCallCheckpoint, CompletedLlmCallCheckpointInput},
};

use super::{
    CHARACTER_CONFLICT_KIND, CHARACTER_CONFLICT_PROGRESS_TOTAL,
    contracts::{
        CharacterConflictProgressReporter, PreparedCharacterConflictCheck,
        ValidatedCharacterConflictInput,
    },
    request::{BuildCharacterConflictRequestInput, build_character_conflict_request},
    workflow::character_conflict_workflow_run_input,
};

impl OrchestratorContext {
    pub async fn prepare_character_conflict_request_preview_validated(
        &self,
        input: CharacterConflictCheckInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<a4n_llm::provider::ChatRequest, AppError> {
        let input = validated_character_conflict_input(&input)?;
        let prepared = self
            .prepare_character_conflict_check(input, prompt_renderer)
            .await?;
        Ok(prepared.request)
    }

    pub async fn check_character_conflict_validated(
        &self,
        input: CharacterConflictCheckInput,
        prompt_renderer: &dyn PromptRenderer,
        progress: CharacterConflictProgressReporter<'_>,
        active_task_id: Option<&str>,
    ) -> Result<Vec<CharacterConflictSummary>, AppError> {
        let input = validated_character_conflict_input(&input)?;
        progress(
            0,
            CHARACTER_CONFLICT_PROGRESS_TOTAL,
            "正在读取角色档案和章节内容",
        )?;
        let prepared = self
            .prepare_character_conflict_check(input, prompt_renderer)
            .await?;
        progress(
            1,
            CHARACTER_CONFLICT_PROGRESS_TOTAL,
            "正在构造一致性检查提示词",
        )?;
        progress(
            2,
            CHARACTER_CONFLICT_PROGRESS_TOTAL,
            "正在调用模型检查角色一致性",
        )?;

        let (workflow_input, workflow_fingerprint) = character_conflict_workflow_run_input(
            prepared.input.project_id,
            prepared.profile_id,
            &prepared.input.name,
            prepared.input.chapter_scope,
            &prepared.call_key,
            &prepared.request,
            active_task_id,
        )?;
        let workflow_run = self.begin_workflow_run_validated(workflow_input).await?;
        let result = self
            .run_character_conflict_llm_call(
                workflow_run.id,
                workflow_fingerprint,
                prepared,
                progress,
            )
            .await;
        self.finalize_workflow_result(workflow_run.id, result, "角色冲突检查")
            .await
    }

    async fn prepare_character_conflict_check(
        &self,
        input: ValidatedCharacterConflictInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<PreparedCharacterConflictCheck, AppError> {
        let profile = self
            .profile_by_project_name(input.project_id, &input.name)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!(
                    "角色不存在: project_id={}, name={}",
                    input.project_id, input.name
                ))
            })?;
        let context_rows = self
            .character_conflict_context_rows(
                input.project_id,
                profile.id,
                input.chapter_scope.start_chapter,
                input.chapter_scope.end_chapter,
            )
            .await?;

        let role_attr = json!({
            "source_id": "profile",
            "base_attributes": profile.base_attributes,
            "current_state": profile.current_state,
        });
        let chapter_context = context_rows
            .chapters
            .iter()
            .map(character_conflict_chapter_context)
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>();
        let state_timeline = context_rows
            .state_timeline
            .iter()
            .map(character_state_context)
            .collect::<Vec<_>>();
        let attribute_events = context_rows
            .attribute_events
            .iter()
            .map(attribute_event_context)
            .collect::<Vec<_>>();
        ensure_character_conflict_has_comparison_context(
            &chapter_context,
            &state_timeline,
            &attribute_events,
        )?;
        ensure_character_conflict_context_size(
            &role_attr,
            &chapter_context,
            &state_timeline,
            &attribute_events,
        )?;
        let source_ids = character_conflict_allowed_source_ids(
            &role_attr,
            &chapter_context,
            &state_timeline,
            &attribute_events,
        );
        let system_prompt = prompt_renderer
            .render_prompt(
                CHARACTER_CONFLICT_KIND,
                &HashMap::new(),
                Some(input.project_id),
            )
            .await?;
        let (call_key, request) =
            build_character_conflict_request(BuildCharacterConflictRequestInput {
                system_prompt,
                project_id: input.project_id,
                profile_id: profile.id,
                profile_name: &input.name,
                chapter_scope: input.chapter_scope,
                role_attr: &role_attr,
                chapter_context: &chapter_context,
                state_timeline: &state_timeline,
                attribute_events: &attribute_events,
            })?;
        Ok(PreparedCharacterConflictCheck {
            input,
            profile_id: profile.id,
            call_key,
            request,
            source_ids,
        })
    }

    async fn run_character_conflict_llm_call(
        &self,
        workflow_run_id: i64,
        workflow_fingerprint: Value,
        mut prepared: PreparedCharacterConflictCheck,
        progress: CharacterConflictProgressReporter<'_>,
    ) -> Result<Vec<CharacterConflictSummary>, AppError> {
        let fingerprint_json = character_conflict_call_fingerprint_json(
            &prepared.call_key,
            &workflow_fingerprint,
            &prepared.request,
        )?;
        workflow::attach_llm_call_checkpoint_trace(
            &mut prepared.request,
            workflow_run_id,
            &prepared.call_key,
            &fingerprint_json,
        )?;
        self.mark_workflow_current_step(workflow_run_id, &prepared.call_key)
            .await?;
        if let Some(checkpoint) = self
            .load_completed_llm_call_checkpoint(
                workflow_run_id,
                &prepared.call_key,
                &fingerprint_json,
            )
            .await?
        {
            progress(
                3,
                CHARACTER_CONFLICT_PROGRESS_TOTAL,
                "角色冲突检查已从断点恢复，正在解析矛盾项",
            )?;
            return restored_character_conflict_checkpoint(
                &checkpoint.parsed_output,
                &checkpoint.response_content,
                &prepared.call_key,
                &prepared.source_ids,
            );
        }

        let response = self.stream_complete_llm(prepared.request).await?;
        if is_truncated_finish_reason(response.finish_reason.as_deref()) {
            return Err(AppError::InvalidArgument(
                "角色冲突检查输出达到当前 max_tokens 上限，返回 JSON 被截断；请在设置页调大“JSON 标准”输出档位，或减少本次检查上下文后重试".to_string(),
            ));
        }
        progress(
            3,
            CHARACTER_CONFLICT_PROGRESS_TOTAL,
            "模型已返回，正在解析矛盾项",
        )?;
        let conflicts =
            parse_conflict_output_with_source_ids(&response.message.content, &prepared.source_ids)?;
        let response_usage_json = response
            .usage
            .as_ref()
            .map(serde_json::to_value)
            .transpose()?;
        self.save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
            workflow_run_id,
            call_key: prepared.call_key,
            call_order: 1,
            input_fingerprint_json: fingerprint_json,
            checkpoint: CompletedLlmCallCheckpoint {
                parsed_output: serde_json::to_value(&conflicts)?,
                response_content: response.message.content,
                usage: response_usage_json,
                finish_reason: response.finish_reason,
                artifacts_json: None,
            },
        })
        .await?;
        Ok(conflicts)
    }
}

#[async_trait::async_trait]
impl super::CharacterConflictApplicationPort for OrchestratorContext {
    async fn preview(
        &self,
        input: CharacterConflictCheckInput,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<a4n_llm::provider::ChatRequest, AppError> {
        self.prepare_character_conflict_request_preview_validated(input, prompt_renderer)
            .await
    }

    async fn check(
        &self,
        input: CharacterConflictCheckInput,
        prompt_renderer: &dyn PromptRenderer,
        progress: CharacterConflictProgressReporter<'_>,
        active_task_id: Option<&str>,
    ) -> Result<Vec<CharacterConflictSummary>, AppError> {
        self.check_character_conflict_validated(input, prompt_renderer, progress, active_task_id)
            .await
    }
}

pub fn validate_character_conflict_input(
    input: &CharacterConflictCheckInput,
) -> Result<(), AppError> {
    validated_character_conflict_input(input).map(drop)
}

pub(super) fn validated_character_conflict_input(
    input: &CharacterConflictCheckInput,
) -> Result<ValidatedCharacterConflictInput, AppError> {
    if input.project_id <= 0 {
        return Err(AppError::InvalidArgument("项目 ID 必须大于 0".to_string()));
    }
    let name = required_text(&input.name, "角色姓名")
        .map_err(AppError::InvalidArgument)?
        .to_string();
    let chapter_scope = CharacterConflictChapterScope::new(input.start_chapter, input.end_chapter)?;
    Ok(ValidatedCharacterConflictInput {
        project_id: input.project_id,
        name,
        chapter_scope,
    })
}
