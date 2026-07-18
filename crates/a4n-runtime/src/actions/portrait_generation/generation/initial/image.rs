mod completion;

use a4n_core::models::character::protagonist_profile;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use super::super::super::image_pipeline::{
    PersistGeneratedPortraitInput, persist_generated_portrait, run_portrait_image_generation,
    to_summary,
};
use super::super::super::workflow::load_completed_portrait_final_checkpoint;
use super::super::super::{AppState, GeneratePortraitInput, PortraitProgress, PortraitSummary};
use super::super::{check_cancelled, report_progress};
use super::prompt::InitialPortraitPrompt;
use completion::{InitialPortraitCompletionInput, complete_initial_portrait};

pub(super) struct InitialPortraitImageContext<'a> {
    pub(super) input: &'a GeneratePortraitInput,
    pub(super) profile: &'a protagonist_profile::Model,
    pub(super) existing_count: u64,
    pub(super) expression: &'a str,
    pub(super) workflow_run_id: i64,
    pub(super) workflow_fingerprint: Value,
    pub(super) prompt: InitialPortraitPrompt,
}

pub(super) async fn generate_and_persist_initial_portrait(
    state: &AppState,
    context: InitialPortraitImageContext<'_>,
    progress: &mut Option<PortraitProgress<'_>>,
    cancel_token: &CancellationToken,
) -> Result<PortraitSummary, String> {
    let InitialPortraitImageContext {
        input,
        profile,
        existing_count,
        expression,
        workflow_run_id,
        workflow_fingerprint,
        prompt,
    } = context;
    report_progress(progress, 3, 6, "正在调用图片模型生成人物三视图拆解")?;
    check_cancelled(cancel_token)?;
    if let Some(summary) = load_completed_portrait_final_checkpoint(
        state,
        workflow_run_id,
        "portrait_generation.final",
        &workflow_fingerprint,
        input.project_id,
    )
    .await?
    {
        report_progress(progress, 6, 6, "人物三视图拆解已从断点恢复")?;
        return Ok(summary);
    }
    let generated = run_portrait_image_generation(
        state,
        input.project_id,
        &prompt.image_prompt,
        &input.reference_image_ids,
        Some(workflow_run_id),
        cancel_token.clone(),
    )
    .await?;
    check_cancelled(cancel_token)?;
    report_progress(progress, 4, 6, "正在保存人物三视图拆解图片")?;
    report_progress(progress, 5, 6, "正在写入人物三视图拆解记录")?;
    let portrait = persist_generated_portrait(
        state,
        &generated,
        PersistGeneratedPortraitInput {
            workflow_run_id: Some(workflow_run_id),
            profile_id: profile.id,
            project_id: input.project_id,
            portrait_type: "full_body",
            prompt: &prompt.image_prompt,
            expression: Some(expression),
            pose: input.pose.as_deref(),
            is_primary: existing_count == 0,
            character_name: &profile.name,
            variant_kind: None,
            auto_generated: false,
            path_context: "人物三视图拆解图片",
            reference_image_ids: &input.reference_image_ids,
        },
    )
    .await?;
    complete_initial_portrait(
        state,
        &portrait,
        InitialPortraitCompletionInput {
            input,
            profile,
            expression,
            workflow_run_id,
            workflow_fingerprint,
            prompt,
        },
    )
    .await?;
    report_progress(progress, 6, 6, "人物三视图拆解生成完成")?;
    Ok(to_summary(&portrait))
}
