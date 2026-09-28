use crate::media_authoring::authoring_error;
use a4n_api_contract::galgame::GalgamePipelineStopAfter;
use a4n_db::media_authoring as records;
use a4n_domain::media_authoring::{
    AuthoringDocumentInputs, AuthoringDocumentKind, AuthoringDocumentStatus,
};
use a4n_error::AppError;
use serde_json::Value;

use crate::galgame::{
    GalgameAdaptationProjection, GalgameBranchDesign, GalgamePipelineStartStage,
    branch_design_from_value, galgame_branch_design_session_user_message,
    galgame_branch_design_user_message,
};

use super::super::super::contracts::CHECKPOINT_BRANCH_PLAN;
use super::super::super::llm::GalgameLlmStageInput;
use super::super::super::progress::ensure_galgame_pipeline_not_cancelled;
use super::super::contracts::GalgamePipelineExecution;

pub(super) async fn run_branch_design_stage(
    execution: &GalgamePipelineExecution<'_>,
    scenes_json: &Value,
    scenes: &GalgameAdaptationProjection,
) -> Result<
    (
        Value,
        GalgameBranchDesign,
        a4n_domain::media_authoring::AuthoringDocumentRef,
    ),
    AppError,
> {
    let stage = GalgamePipelineStartStage::BranchDesign;
    ensure_galgame_pipeline_not_cancelled(execution.service, execution.options.workflow_run_id)
        .await?;
    let db = execution.service.database.connection();
    let mut inputs = AuthoringDocumentInputs {
        sources: Vec::new(),
        dependencies: Vec::new(),
        video_reference: None,
    };
    for adaptation in &execution.prompt_context.adaptations {
        let record =
            records::authoring_document(db, execution.project_id, adaptation.adaptation_plan_id)
                .await
                .map_err(authoring_error)?;
        if record.status != AuthoringDocumentStatus::Adopted
            || record.revision != adaptation.adaptation_plan_revision
        {
            return Err(AppError::InvalidArgument(
                "改编计划版本已变化，请重新准备分支阶段".into(),
            ));
        }
        inputs.sources.extend(record.inputs.sources.iter().cloned());
        inputs.dependencies.push(record.reference());
    }
    let previous = records::editable_authoring_document(
        db,
        execution.project_id,
        execution.prompt_context.source_snapshot_id,
        None,
        AuthoringDocumentKind::GalgameBranchPlan,
    )
    .await
    .map_err(authoring_error)?;
    let review_branch =
        execution.options.stop_after == Some(GalgamePipelineStopAfter::BranchDesign);
    if !review_branch
        && execution.options.start_from_stage != Some(stage)
        && let Some(record) = previous.as_ref().filter(|record| record.inputs == inputs)
    {
        let branch = branch_design_from_value(record.document.clone(), "采用已保存分支计划")?;
        crate::galgame::validate_galgame_branch_design(&branch, scenes)?;
        if record.status == AuthoringDocumentStatus::Draft {
            records::adopt_authoring_document(db, execution.project_id, record.id, record.revision)
                .await
                .map_err(authoring_error)?;
        }
        return Ok((record.document.clone(), branch, record.reference()));
    }
    execution
        .service
        .report_pipeline_progress(
            execution.options.workflow_run_id,
            execution.options.task_id.as_deref(),
            execution.options.progress_reporter.as_ref(),
            stage,
            2,
            stage.message(),
        )
        .await?;
    let user_prompt = galgame_branch_design_user_message(scenes_json)?;
    let request_user_prompt = galgame_branch_design_session_user_message()?;
    let output = execution
        .service
        .run_galgame_llm_stage(
            GalgameLlmStageInput {
                prompt_renderer: execution.prompt_renderer,
                project_id: execution.project_id,
                root_project_id: execution.prompt_context.root_project_id,
                workflow_run_id: execution.options.workflow_run_id,
                stage,
                checkpoint_key: CHECKPOINT_BRANCH_PLAN,
                prompt_name: "galgame_branch_design",
                user_prompt,
                request_user_prompt,
                shared_adaptation_projection: scenes_json,
                restore_from_checkpoint: execution.should_restore(stage),
                restore_required: execution.restore_required(stage),
            },
            |value| {
                let parsed = branch_design_from_value(value, "galgame_branch_design")?;
                crate::galgame::pipeline::persist::validate_galgame_branch_design(&parsed, scenes)?;
                Ok(parsed)
            },
        )
        .await?;
    let draft = previous.filter(|record| record.status == AuthoringDocumentStatus::Draft);
    let record = records::save_authoring_document(
        db,
        records::SaveAuthoringDocument {
            project_id: execution.project_id,
            source_snapshot_id: execution.prompt_context.source_snapshot_id,
            source_chapter_id: None,
            kind: AuthoringDocumentKind::GalgameBranchPlan,
            record_id: draft.as_ref().map(|record| record.id),
            expected_revision: draft.map(|record| record.revision),
            inputs,
            document: serde_json::to_value(&output.1)?,
        },
    )
    .await
    .map_err(authoring_error)?;
    if !review_branch {
        records::adopt_authoring_document(db, execution.project_id, record.id, record.revision)
            .await
            .map_err(authoring_error)?;
    }
    execution
        .service
        .report_pipeline_progress(
            execution.options.workflow_run_id,
            execution.options.task_id.as_deref(),
            execution.options.progress_reporter.as_ref(),
            stage,
            2,
            if review_branch {
                "分支计划草稿已保存，可以检查、编辑后采用"
            } else {
                "已采用 Galgame 分支计划"
            },
        )
        .await?;
    Ok((output.0, output.1, record.reference()))
}
