use super::assembly::assemble_staged_blueprint_json;
use super::completion::{BlueprintGenerationCompletionInput, persist_generated_blueprint};
use super::contracts::BLUEPRINT_GENERATION_STAGES;
use super::stage::{BlueprintStageRunInput, ensure_not_cancelled, report};
use super::trace::blueprint_stage_trace_requests;
use crate::blueprint::json::parse_blueprint_json;
use crate::blueprint::{BlueprintGenerationInput, OrchestratorContext};
use a4n_core::blueprint::{blueprint_stage_previous_context, build_blueprint_generation_payload};
use a4n_core::error::AppError;
use a4n_core::models::project::novel_blueprint;
use serde_json::Value;

impl OrchestratorContext {
    pub async fn generate_blueprint(
        &self,
        input: BlueprintGenerationInput<'_>,
    ) -> Result<novel_blueprint::Model, AppError> {
        let BlueprintGenerationInput {
            project_id,
            novel_title,
            genre,
            target_length,
            prompt_renderer,
            variables,
            cancel_token,
            progress,
            workflow_run_id,
        } = input;
        report(progress, 1, "整理灵感对话与蓝图提示词")?;
        ensure_not_cancelled(cancel_token)?;
        let prompt_content = prompt_renderer
            .render_prompt("blueprint", variables, Some(project_id))
            .await?;
        let user_payload =
            build_blueprint_generation_payload(novel_title, genre, target_length, variables)?;
        let user_payload_json = serde_json::to_string_pretty(&user_payload)?;

        let mut stage_outputs = Vec::new();
        for (stage_index, spec) in BLUEPRINT_GENERATION_STAGES.iter().enumerate() {
            report(progress, spec.progress_current, spec.progress_message)?;
            let previous_context = blueprint_stage_previous_context(stage_index, &stage_outputs)?;
            let value = self
                .generate_or_resume_blueprint_stage(BlueprintStageRunInput {
                    project_id,
                    spec,
                    progress,
                    cancel_token,
                    prompt_renderer,
                    base_prompt: &prompt_content,
                    user_payload_json: &user_payload_json,
                    previous_context: previous_context.as_ref(),
                    workflow_run_id,
                })
                .await?;
            stage_outputs.push(value);
        }
        let [
            core_output,
            world_output,
            characters_output,
            relationships_output,
        ]: [Value; 4] = stage_outputs
            .try_into()
            .map_err(|_| AppError::InvalidArgument("蓝图生成阶段输出数量不完整".to_string()))?;
        let trace_requests = blueprint_stage_trace_requests(
            prompt_renderer,
            project_id,
            &prompt_content,
            &user_payload_json,
            &[
                core_output.clone(),
                world_output.clone(),
                characters_output.clone(),
                relationships_output.clone(),
            ],
        )
        .await?;

        report(progress, 6, "组合并校验完整蓝图结构")?;
        ensure_not_cancelled(cancel_token)?;
        let assembled = assemble_staged_blueprint_json(
            novel_title,
            genre,
            target_length,
            &core_output,
            &world_output,
            &characters_output,
            &relationships_output,
        );
        let assembled_content = serde_json::to_string(&assembled)?;
        let parsed = parse_blueprint_json(&assembled_content, target_length)?;

        report(progress, 7, "校验章节规划与工作流边界")?;
        report(progress, 8, "保留已有大纲与章节产物")?;
        ensure_not_cancelled(cancel_token)?;

        persist_generated_blueprint(
            self,
            BlueprintGenerationCompletionInput {
                project_id,
                novel_title,
                genre,
                target_length,
                user_payload,
                trace_requests,
                assembled_content,
                parsed,
                progress,
            },
        )
        .await
    }
}
