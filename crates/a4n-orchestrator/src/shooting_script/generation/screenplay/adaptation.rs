use a4n_core::{config::app_config::llm_token_profiles, error::AppError};
use a4n_llm::provider::ChatRequest;
use tokio_util::sync::CancellationToken;

use crate::{
    OrchestratorContext,
    shooting_script::{
        context::shooting_context_request,
        contracts::{
            CHAPTER_SCREENPLAY_UNIT_ADAPTATION_PROMPT, ChapterScreenplayLayoutV1,
            ChapterScreenplayPlan, PreparedChapterShootingScriptRequest, ScreenplayLayoutUnitV1,
            ScreenplayUnitAdaptationV1, ShootingScriptProgressReporter,
        },
        progress::unit_adaptation_stream_progress,
        screenplay::{assemble_screenplay_plan, validate_screenplay_plan},
    },
    workflow::{required_llm_call_checkpoint_json, workflow_checkpoint_contract_app_error},
};

use super::validation::validate_unit_adaptation;

pub(super) async fn generate_screenplay_adaptations(
    orchestrator: &OrchestratorContext,
    workflow_run_id: i64,
    prepared: &PreparedChapterShootingScriptRequest,
    layout: ChapterScreenplayLayoutV1,
    progress_total: u32,
    progress: ShootingScriptProgressReporter<'_>,
    cancel_token: Option<&CancellationToken>,
) -> Result<ChapterScreenplayPlan, AppError> {
    let restored_adaptations =
        restorable_adaptation_prefix(orchestrator, workflow_run_id, prepared, &layout).await?;
    let adaptation_start_progress = if restored_adaptations == 0 {
        18
    } else {
        unit_adaptation_stream_progress(restored_adaptations - 1, layout.units.len())
            .span
            .end()
    };
    progress(
        adaptation_start_progress,
        progress_total,
        if restored_adaptations == 0 {
            "章节布局已确定性装配，开始逐单元剧本改编".to_string()
        } else {
            format!(
                "已确认可恢复 {restored_adaptations}/{} 个剧本改编单元",
                layout.units.len()
            )
        }
        .as_str(),
    )?;

    let mut adaptations = Vec::with_capacity(layout.units.len());
    for (offset, unit) in layout.units.iter().enumerate() {
        let unit_progress = unit_adaptation_stream_progress(offset, layout.units.len());
        let request = unit_adaptation_request(prepared, workflow_run_id, &layout, unit)?;
        let stage_progress = |current: u32, total: u32, message: &str| {
            progress(
                if offset < restored_adaptations {
                    current.max(adaptation_start_progress)
                } else {
                    current
                },
                total,
                message,
            )
        };
        let stage = orchestrator
            .run_shooting_script_stage::<ScreenplayUnitAdaptationV1>(
                workflow_run_id,
                &format!("screenplay_unit_adaptation:{}", unit.narrative_unit_index),
                i32::try_from(layout.units.len() + offset + 2)
                    .map_err(|_| AppError::InvalidArgument("改编单元数量超限".into()))?,
                request,
                "叙事单元剧本改编",
                unit_progress,
                progress_total,
                &stage_progress,
                cancel_token,
                |output| validate_unit_adaptation(output, unit),
            )
            .await?;
        progress(
            if offset < restored_adaptations {
                adaptation_start_progress
            } else {
                unit_progress.span.end()
            },
            progress_total,
            &format!(
                "叙事单元 {}/{} {}",
                offset + 1,
                layout.units.len(),
                if stage.restored {
                    "已从调用断点恢复"
                } else {
                    "表演事实完成"
                }
            ),
        )?;
        adaptations.push(stage.output);
    }

    let plan = assemble_screenplay_plan(layout, adaptations)?;
    validate_screenplay_plan(&plan, &prepared.source_spans)?;
    Ok(plan)
}

fn unit_adaptation_request(
    prepared: &PreparedChapterShootingScriptRequest,
    workflow_run_id: i64,
    layout: &ChapterScreenplayLayoutV1,
    unit: &ScreenplayLayoutUnitV1,
) -> Result<ChatRequest, AppError> {
    shooting_context_request(
        prepared,
        workflow_run_id,
        CHAPTER_SCREENPLAY_UNIT_ADAPTATION_PROMPT,
        format!("screenplay_unit_adaptation:{}", unit.narrative_unit_index),
        prepared.unit_adaptation_system_prompt.clone(),
        Some(("shooting_screenplay_layout", layout)),
        format!(
            "只改编以下目标 unit 的 performance_facts，不得使用其他 unit 的 source_span_ids：\n{}",
            serde_json::to_string(unit)?
        ),
        llm_token_profiles::LONG.into(),
    )
}

async fn restorable_adaptation_prefix(
    orchestrator: &OrchestratorContext,
    workflow_run_id: i64,
    prepared: &PreparedChapterShootingScriptRequest,
    layout: &ChapterScreenplayLayoutV1,
) -> Result<usize, AppError> {
    let mut restored = 0;
    for unit in &layout.units {
        let call_key = format!("screenplay_unit_adaptation:{}", unit.narrative_unit_index);
        let request = unit_adaptation_request(prepared, workflow_run_id, layout, unit)?;
        let fingerprint = a4n_llm::trace::context_aware_chat_request_trace_json(&request)?;
        let Some(checkpoint) = orchestrator
            .load_completed_llm_call_checkpoint_with_repair_state(
                workflow_run_id,
                &call_key,
                &fingerprint,
            )
            .await?
        else {
            break;
        };
        let checkpoint_json = required_llm_call_checkpoint_json(checkpoint, &call_key)
            .map_err(workflow_checkpoint_contract_app_error)?;
        let Some(parsed) = checkpoint_json
            .get("parsed_output")
            .filter(|value| !value.is_null())
        else {
            break;
        };
        let output: ScreenplayUnitAdaptationV1 = serde_json::from_value(parsed.clone())?;
        if validate_unit_adaptation(&output, unit).is_err() {
            break;
        }
        restored += 1;
    }
    Ok(restored)
}
