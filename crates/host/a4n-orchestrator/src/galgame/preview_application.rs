use std::collections::HashMap;

use a4n_error::AppError;
use a4n_llm_provider_contract::provider::{ChatMessage, ChatRequest};
use a4n_prompt::rendering::PromptRenderer;
use serde_json::Value;

use crate::{
    OrchestratorContext,
    workflow::{ListWorkflowRunsInput, WORKFLOW_STATUS_COMPLETED},
};

use super::{
    GALGAME_PIPELINE_WORKFLOW_KIND, GalgamePipelinePromptContext, GalgamePipelineStartStage,
    adaptation_projection_from_value, branch_design_from_value, character_staging_from_value,
    galgame_adaptation_projection, galgame_branch_design_user_message,
    galgame_character_staging_user_message, galgame_script_adaptation_user_message,
    galgame_source_markers, galgame_visual_planning_user_message, script_adaptation_from_value,
    visual_plan_from_value,
};

const CHECKPOINT_SCENES: &str = "scenes";
const CHECKPOINT_LINES_DRAFT: &str = "lines_draft";
const CHECKPOINT_LINES_STAGED: &str = "lines_staged";
const CHECKPOINT_VISUAL_PLAN: &str = "visual_plan";
const CHECKPOINT_BRANCH_PLAN: &str = "branch_plan";

#[derive(Debug, Clone)]
pub struct GalgamePipelinePreviewRequest {
    pub key: String,
    pub title: String,
    pub prompt_name: String,
    pub request: ChatRequest,
}

#[derive(Default)]
struct PreviewCheckpointInputs {
    scenes: Option<Value>,
    lines_draft: Option<Value>,
    lines_staged: Option<Value>,
    visual_plan: Option<Value>,
    branch_plan: Option<Value>,
    required_prior_stage: Option<GalgamePipelineStartStage>,
}

impl OrchestratorContext {
    pub async fn prepare_galgame_pipeline_preview_validated(
        &self,
        project_id: i64,
        start_from_stage: Option<GalgamePipelineStartStage>,
        resume_from_checkpoint: bool,
        prompt_renderer: &dyn PromptRenderer,
    ) -> Result<Vec<GalgamePipelinePreviewRequest>, AppError> {
        let (context_result, checkpoints_result) = futures::join!(
            self.galgame_pipeline_prompt_context(project_id),
            self.load_galgame_preview_checkpoint_inputs(
                project_id,
                start_from_stage,
                resume_from_checkpoint,
            ),
        );
        let context = context_result?;
        let checkpoints = checkpoints_result?;
        let adaptation_projection = match checkpoints.prior_stage_input(
            GalgamePipelineStartStage::AdaptationPlan,
            checkpoints.scenes.clone(),
        )? {
            Some(value) => value,
            None => serde_json::to_value(galgame_adaptation_projection(&context)?)?,
        };
        let branch_design = checkpoints.prior_stage_input(
            GalgamePipelineStartStage::BranchDesign,
            checkpoints.branch_plan.clone(),
        )?;
        let script_adaptation = checkpoints.prior_stage_input(
            GalgamePipelineStartStage::ScriptAdaptation,
            checkpoints.lines_draft.clone(),
        )?;
        let character_staging = checkpoints.prior_stage_input(
            GalgamePipelineStartStage::CharacterStaging,
            checkpoints.lines_staged.clone(),
        )?;
        self.build_galgame_preview_requests(
            prompt_renderer,
            start_from_stage,
            &context,
            &adaptation_projection,
            branch_design.as_ref(),
            script_adaptation.as_ref(),
            character_staging.as_ref(),
        )
        .await
    }

    async fn load_galgame_preview_checkpoint_inputs(
        &self,
        project_id: i64,
        start_from_stage: Option<GalgamePipelineStartStage>,
        resume_from_checkpoint: bool,
    ) -> Result<PreviewCheckpointInputs, AppError> {
        if start_from_stage.is_none() && !resume_from_checkpoint {
            return Ok(PreviewCheckpointInputs::default());
        }
        let mut inputs = PreviewCheckpointInputs {
            required_prior_stage: start_from_stage,
            ..Default::default()
        };
        let Some(run) = self
            .workflow_run_models(ListWorkflowRunsInput {
                project_id: Some(project_id),
                chapter_id: None,
                chapter_ids: None,
                workflow_kind: Some(GALGAME_PIPELINE_WORKFLOW_KIND.to_string()),
                status: None,
                resumable_only: None,
                limit: Some(1),
            })
            .await?
            .into_iter()
            .next()
        else {
            return Ok(inputs);
        };
        for checkpoint in self.workflow_checkpoint_summaries(run.id).await? {
            if checkpoint.status != WORKFLOW_STATUS_COMPLETED {
                continue;
            }
            let output = galgame_preview_checkpoint_output(
                &checkpoint.step_key,
                checkpoint.checkpoint_json.as_ref(),
            )?;
            match checkpoint.step_key.as_str() {
                CHECKPOINT_SCENES => inputs.scenes = Some(output),
                CHECKPOINT_LINES_DRAFT => inputs.lines_draft = Some(output),
                CHECKPOINT_LINES_STAGED => inputs.lines_staged = Some(output),
                CHECKPOINT_VISUAL_PLAN => inputs.visual_plan = Some(output),
                CHECKPOINT_BRANCH_PLAN => inputs.branch_plan = Some(output),
                other => {
                    return Err(AppError::InvalidState {
                        current: "galgame_preview_checkpoint".to_string(),
                        operation: format!("Galgame 管线存在未知 completed checkpoint: {other}"),
                    });
                }
            }
        }
        inputs.validate_required_prior_checkpoints()?;
        Ok(inputs)
    }

    #[allow(clippy::too_many_arguments)]
    async fn build_galgame_preview_requests(
        &self,
        prompt_renderer: &dyn PromptRenderer,
        start_from_stage: Option<GalgamePipelineStartStage>,
        context: &GalgamePipelinePromptContext,
        adaptation_projection: &Value,
        branch_design: Option<&Value>,
        script_adaptation: Option<&Value>,
        character_staging: Option<&Value>,
    ) -> Result<Vec<GalgamePipelinePreviewRequest>, AppError> {
        let source_markers = galgame_source_markers(&context.chapters)?;
        let stage_filter = |stage: GalgamePipelineStartStage| {
            start_from_stage
                .map(|start| stage.order() >= start.order())
                .unwrap_or(true)
        };
        let mut pending = Vec::new();
        if stage_filter(GalgamePipelineStartStage::BranchDesign) {
            pending.push((
                "branch_design",
                "分支设计",
                "galgame_branch_design",
                galgame_branch_design_user_message(adaptation_projection, &source_markers)?,
            ));
        }
        if stage_filter(GalgamePipelineStartStage::ScriptAdaptation)
            && let Some(branch_design) = branch_design
        {
            pending.push((
                "script_adaptation",
                "剧本行改编",
                "galgame_script_adaptation",
                galgame_script_adaptation_user_message(
                    &context.chapters,
                    adaptation_projection,
                    branch_design,
                    &source_markers,
                )?,
            ));
        }
        if stage_filter(GalgamePipelineStartStage::CharacterStaging)
            && let Some(script_adaptation) = script_adaptation
        {
            pending.push((
                "character_staging",
                "角色演出标注",
                "galgame_character_staging",
                galgame_character_staging_user_message(
                    script_adaptation,
                    &context.speaker_profiles,
                    &source_markers,
                )?,
            ));
        }
        if stage_filter(GalgamePipelineStartStage::VisualPlanning)
            && let Some(character_staging) = character_staging
        {
            pending.push((
                "visual_planning",
                "视觉资产规划",
                "galgame_visual_planning",
                galgame_visual_planning_user_message(
                    adaptation_projection,
                    character_staging,
                    &source_markers,
                )?,
            ));
        }
        futures::future::join_all(pending.into_iter().map(
            |(key, title, prompt_name, user_prompt)| {
                build_preview_request(prompt_renderer, key, title, prompt_name, user_prompt)
            },
        ))
        .await
        .into_iter()
        .collect()
    }
}

impl PreviewCheckpointInputs {
    fn prior_stage_input(
        &self,
        stage: GalgamePipelineStartStage,
        value: Option<Value>,
    ) -> Result<Option<Value>, AppError> {
        if self
            .required_prior_stage
            .is_some_and(|start| stage.order() < start.order())
        {
            return value.map(Some).ok_or_else(|| AppError::InvalidState {
                current: "galgame_preview_checkpoint".to_string(),
                operation: format!(
                    "Galgame 请求预览缺少上游 checkpoint: start_from_stage={}, required_stage={}",
                    start_stage_key(self.required_prior_stage),
                    stage.key()
                ),
            });
        }
        Ok(value)
    }

    fn validate_required_prior_checkpoints(&self) -> Result<(), AppError> {
        let Some(start) = self.required_prior_stage else {
            return Ok(());
        };
        for (stage, value) in [
            (GalgamePipelineStartStage::AdaptationPlan, &self.scenes),
            (GalgamePipelineStartStage::BranchDesign, &self.branch_plan),
            (
                GalgamePipelineStartStage::ScriptAdaptation,
                &self.lines_draft,
            ),
            (
                GalgamePipelineStartStage::CharacterStaging,
                &self.lines_staged,
            ),
            (GalgamePipelineStartStage::VisualPlanning, &self.visual_plan),
        ] {
            if stage.order() < start.order() && value.is_none() {
                return Err(AppError::InvalidState {
                    current: "galgame_preview_checkpoint".to_string(),
                    operation: format!(
                        "Galgame 请求预览缺少上游 checkpoint: start_from_stage={}, required_stage={}",
                        start.key(),
                        stage.key()
                    ),
                });
            }
        }
        Ok(())
    }
}

async fn build_preview_request(
    prompt_renderer: &dyn PromptRenderer,
    key: &str,
    title: &str,
    prompt_name: &str,
    user_prompt: String,
) -> Result<GalgamePipelinePreviewRequest, AppError> {
    let system_prompt = prompt_renderer
        .render_prompt(prompt_name, &HashMap::new(), None)
        .await
        .map_err(|error| AppError::InvalidState {
            current: "galgame_preview_prompt".to_string(),
            operation: error.to_string(),
        })?;
    Ok(GalgamePipelinePreviewRequest {
        key: key.to_string(),
        title: title.to_string(),
        prompt_name: prompt_name.to_string(),
        request: ChatRequest::new(
            vec![
                ChatMessage::system(system_prompt),
                ChatMessage::user(user_prompt),
            ],
            Some(0.25),
            None,
        )
        .with_json_response_format(),
    })
}

fn galgame_preview_checkpoint_output(
    checkpoint_key: &str,
    checkpoint_json: Option<&Value>,
) -> Result<Value, AppError> {
    let object = checkpoint_json.and_then(Value::as_object).ok_or_else(|| {
        invalid_checkpoint(format!(
            "Galgame checkpoint {checkpoint_key} 缺少 JSON 对象"
        ))
    })?;
    let output = object
        .get("output")
        .ok_or_else(|| {
            invalid_checkpoint(format!("Galgame checkpoint {checkpoint_key} 缺少 output"))
        })?
        .clone();
    if let Some(parsed_output) = object.get("parsed_output")
        && parsed_output != &output
    {
        return Err(invalid_checkpoint(format!(
            "Galgame checkpoint {checkpoint_key} 的 output 与 parsed_output 不一致"
        )));
    }
    match checkpoint_key {
        CHECKPOINT_SCENES => {
            adaptation_projection_from_value(output.clone(), checkpoint_key)?;
        }
        CHECKPOINT_LINES_DRAFT => {
            script_adaptation_from_value(output.clone(), checkpoint_key)?;
        }
        CHECKPOINT_LINES_STAGED => {
            character_staging_from_value(output.clone(), checkpoint_key)?;
        }
        CHECKPOINT_VISUAL_PLAN => {
            visual_plan_from_value(output.clone(), checkpoint_key)?;
        }
        CHECKPOINT_BRANCH_PLAN => {
            branch_design_from_value(output.clone(), checkpoint_key)?;
        }
        other => {
            return Err(invalid_checkpoint(format!(
                "Galgame checkpoint key 不支持: {other}"
            )));
        }
    }
    Ok(output)
}

fn invalid_checkpoint(operation: String) -> AppError {
    AppError::InvalidState {
        current: "galgame_preview_checkpoint".to_string(),
        operation,
    }
}

fn start_stage_key(stage: Option<GalgamePipelineStartStage>) -> &'static str {
    stage
        .map(GalgamePipelineStartStage::key)
        .unwrap_or("adaptation_projection")
}
