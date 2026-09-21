use a4n_error::AppError;
use a4n_llm_provider_contract::provider::is_truncated_finish_reason;
use a4n_prompt::rendering::PromptRenderer;
use serde_json::{Value, json};

use crate::{
    OrchestratorContext,
    character::{
        category_attributes, required_i32,
        sync_projection::{
            apply_character_sync_change_to_state, project_character_attribute_changes,
        },
    },
    structured_llm::ensure_checkpoint_parsed_output_matches,
    workflow::{CompletedLlmCallCheckpoint, CompletedLlmCallCheckpointInput},
};

use super::{
    application::{apply_implicit_classification_to_behavior, set_behavior_classification_results},
    contracts::{CharacterImplicitClassificationInput, CharacterImplicitClassificationOutput},
    output::parse_implicit_classification_output,
    request::{PreparedImplicitClassificationCall, prepare_implicit_classification_call},
};

impl OrchestratorContext {
    pub(crate) async fn classify_character_sync_behaviors_validated(
        &self,
        input: CharacterImplicitClassificationInput<'_>,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<CharacterImplicitClassificationOutput, AppError> {
        let CharacterImplicitClassificationInput {
            project_id,
            workflow_run_id,
            profile_id,
            profile_name,
            changes,
            mut behaviors,
            mut final_attributes,
        } = input;
        if behaviors.is_empty() {
            final_attributes = project_character_attribute_changes(final_attributes, changes)
                .map_err(AppError::InvalidArgument)?;
            return Ok(CharacterImplicitClassificationOutput {
                behaviors,
                final_attributes,
                classification_suggestions: Vec::new(),
            });
        }

        let mut change_index = 0usize;
        let mut suggestions = Vec::new();
        for (behavior_index, behavior) in behaviors.iter_mut().enumerate() {
            let chapter =
                required_i32(behavior, "chapter_number").map_err(AppError::InvalidArgument)?;
            while change_index < changes.len()
                && required_i32(&changes[change_index], "chapter_number")
                    .map_err(AppError::InvalidArgument)?
                    <= chapter
            {
                apply_character_sync_change_to_state(&mut final_attributes, &changes[change_index])
                    .map_err(AppError::InvalidArgument)?;
                change_index += 1;
            }

            let implicit_attributes = category_attributes(&final_attributes, "implicit")
                .map_err(AppError::InvalidArgument)?;
            if implicit_attributes
                .as_object()
                .is_some_and(serde_json::Map::is_empty)
            {
                set_behavior_classification_results(behavior, json!({}))
                    .map_err(AppError::InvalidArgument)?;
                continue;
            }

            let prepared = prepare_implicit_classification_call(
                prompt_renderer,
                project_id,
                workflow_run_id,
                profile_id,
                profile_name,
                behavior_index,
                chapter,
                behavior,
                &implicit_attributes,
            )
            .await?;
            let parsed = self
                .run_implicit_classification_call(prepared, behavior_index, &implicit_attributes)
                .await?;
            suggestions.extend(
                apply_implicit_classification_to_behavior(
                    behavior,
                    &parsed,
                    &mut final_attributes,
                    behavior_index,
                )
                .map_err(AppError::InvalidArgument)?,
            );
        }

        while change_index < changes.len() {
            apply_character_sync_change_to_state(&mut final_attributes, &changes[change_index])
                .map_err(AppError::InvalidArgument)?;
            change_index += 1;
        }

        Ok(CharacterImplicitClassificationOutput {
            behaviors,
            final_attributes,
            classification_suggestions: suggestions,
        })
    }

    async fn run_implicit_classification_call(
        &self,
        prepared: PreparedImplicitClassificationCall,
        behavior_index: usize,
        implicit_attributes: &Value,
    ) -> Result<Value, AppError> {
        let PreparedImplicitClassificationCall {
            request,
            call_key,
            workflow_run_id,
            input_fingerprint_json,
        } = prepared;
        self.mark_workflow_current_step(workflow_run_id, &call_key)
            .await?;
        let (response_text, usage, finish_reason, restored_parsed_output) = if let Some(
            checkpoint,
        ) = self
            .load_completed_llm_call_checkpoint(workflow_run_id, &call_key, &input_fingerprint_json)
            .await?
        {
            (
                checkpoint.response_content,
                checkpoint.usage,
                checkpoint.finish_reason,
                Some(checkpoint.parsed_output),
            )
        } else {
            let response = self.stream_complete_llm(request).await?;
            if is_truncated_finish_reason(response.finish_reason.as_deref()) {
                return Err(AppError::InvalidArgument(
                        "行为隐性属性分类输出达到当前 max_tokens 上限，返回 JSON 被截断；请在模型配置中提高 max_output_tokens，或减少本次分类上下文后重试"
                            .to_string(),
                    ));
            }
            let usage = response
                .usage
                .as_ref()
                .map(serde_json::to_value)
                .transpose()?;
            (
                response.message.content,
                usage,
                response.finish_reason,
                None,
            )
        };

        let parsed = parse_implicit_classification_output(&response_text, implicit_attributes)
            .map_err(|error| {
                AppError::InvalidArgument(format!("行为隐性属性分类输出不符合契约：{error}"))
            })?;
        if let Some(parsed_output) = restored_parsed_output {
            ensure_checkpoint_parsed_output_matches(
                &parsed_output,
                &parsed,
                &format!("行为隐性属性分类 {call_key}"),
            )?;
        } else {
            let call_order = i32::try_from(behavior_index)
                .map_err(|_| {
                    AppError::InvalidArgument(format!(
                        "行为隐性属性分类索引超过 i32 范围：{behavior_index}"
                    ))
                })?
                .checked_add(1000)
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "行为隐性属性分类 call_order 超过 i32 范围：{behavior_index}"
                    ))
                })?;
            self.save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
                workflow_run_id,
                call_key,
                call_order,
                input_fingerprint_json,
                checkpoint: CompletedLlmCallCheckpoint {
                    parsed_output: parsed.clone(),
                    response_content: response_text,
                    usage,
                    finish_reason,
                    artifacts_json: None,
                },
            })
            .await?;
        }
        Ok(parsed)
    }
}
