use super::super::super::image_pipeline::{
    PersistGeneratedPortraitInput, character_portrait_by_id, persist_generated_portrait,
    portrait_profile_by_id, run_portrait_image_generation,
};
use super::super::super::workflow::{
    finalize_portrait_workflow, load_completed_portrait_final_checkpoint,
    portrait_image_business_key, portrait_image_workflow_fingerprint,
};
use super::super::super::{GenerateAgeVariantInput, PortraitProgress, PortraitSummary, to_summary};
use super::super::{
    age_variant_prompt, check_cancelled, generate_age_variant_task_scope, report_progress,
};
use crate::AppState;
use a4n_orchestrator::workflow::{
    CompletedWorkflowCheckpointInput, UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING,
};
use serde_json::json;
use tokio_util::sync::CancellationToken;

pub async fn generate_age_variant(
    state: &AppState,
    user_id: i64,
    input: GenerateAgeVariantInput,
    mut progress: Option<PortraitProgress<'_>>,
    cancel_token: CancellationToken,
) -> Result<PortraitSummary, String> {
    generate_age_variant_task_scope(state, user_id, &input).await?;
    let existing = character_portrait_by_id(state, input.portrait_id).await?;
    report_progress(&mut progress, 0, 4, "正在读取立绘和角色档案")?;
    check_cancelled(&cancel_token)?;
    let profile = portrait_profile_by_id(state, existing.profile_id, existing.id).await?;
    let new_prompt = age_variant_prompt(&existing.prompt, &input.age);
    let workflow_fingerprint = portrait_image_workflow_fingerprint(
        "portrait_age_variant_generation",
        existing.project_id,
        existing.id,
        &new_prompt,
        &input.reference_image_ids,
        json!({
            "variant_kind": "age_variant",
            "age": input.age}),
    );
    let workflow_run = state
        .orchestrator
        .upsert_workflow_run(UpsertWorkflowRunInput {
            business_key: portrait_image_business_key(
                "portrait_age_variant_generation",
                &workflow_fingerprint,
            )?,
            workflow_kind: "portrait_age_variant_generation".to_string(),
            project_id: Some(existing.project_id),
            root_project_id: None,
            source_snapshot_id: None,
            scope_table: Some("character_portrait".to_string()),
            scope_id: Some(existing.id),
            chapter_id: None,
            mode: Some("age_variant".to_string()),
            input_fingerprint_json: Some(workflow_fingerprint.clone()),
            status: WORKFLOW_STATUS_RUNNING.to_string(),
            current_step_key: Some("portrait_age_variant_generation.final".to_string()),
            active_task_id: None,
            can_resume: false,
            resume_block_reason: None,
            error_message: None,
        })
        .await
        .map_err(|err| err.to_string())?;
    let workflow_run_id = workflow_run.id;

    let result = async {
        report_progress(&mut progress, 1, 4, "正在调用图片模型生成年龄变体")?;
        if let Some(summary) = load_completed_portrait_final_checkpoint(
            state,
            workflow_run_id,
            "portrait_age_variant_generation.final",
            &workflow_fingerprint,
            existing.project_id,
        )
        .await?
        {
            report_progress(&mut progress, 4, 4, "年龄变体已从断点恢复")?;
            return Ok(summary);
        }
        let generated = run_portrait_image_generation(
            state,
            existing.project_id,
            &new_prompt,
            &input.reference_image_ids,
            Some(workflow_run_id),
            cancel_token.clone(),
        )
        .await?;
        check_cancelled(&cancel_token)?;
        report_progress(&mut progress, 2, 4, "正在保存年龄变体图片")?;
        report_progress(&mut progress, 3, 4, "正在写入年龄变体记录")?;
        let new_portrait = persist_generated_portrait(
            state,
            &generated,
            PersistGeneratedPortraitInput {
                workflow_run_id: Some(workflow_run_id),
                profile_id: existing.profile_id,
                project_id: existing.project_id,
                portrait_type: &existing.portrait_type,
                prompt: &new_prompt,
                expression: existing.expression.as_deref(),
                pose: existing.pose.as_deref(),
                is_primary: false,
                character_name: &profile.name,
                variant_kind: Some("age_variant"),
                auto_generated: false,
                path_context: "年龄变体立绘",
                reference_image_ids: &input.reference_image_ids,
            },
        )
        .await?;
        state
            .orchestrator
            .save_completed_workflow_checkpoint(CompletedWorkflowCheckpointInput {
                workflow_run_id: Some(workflow_run_id),
                step_key: "portrait_age_variant_generation.final".to_string(),
                step_order: 1,
                input_fingerprint_json: workflow_fingerprint,
                checkpoint_json: json!({
                    "portrait_id": new_portrait.id,
                    "image_path": new_portrait.image_path,
                    "summary": to_summary(&new_portrait)}),
            })
            .await
            .map_err(|error| error.to_string())?;
        report_progress(&mut progress, 4, 4, "年龄变体生成完成")?;
        Ok(to_summary(&new_portrait))
    }
    .await;

    if let Err(status_err) = finalize_portrait_workflow(state, workflow_run_id, &result).await {
        return match result {
            Ok(_) => Err(format!(
                "年龄变体生成完成，但工作流状态写入失败: {status_err}"
            )),
            Err(err) => Err(format!("{err}；工作流失败状态写入失败: {status_err}")),
        };
    }
    result
}
