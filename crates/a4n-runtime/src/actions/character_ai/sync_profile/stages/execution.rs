use serde_json::json;

use crate::actions::character_ai::sync_trace::record_character_sync_llm_trace;
use crate::actions::character_ai::sync_types::{
    CHARACTER_PROFILE_SYNC_STEPS, CharacterProgressScope, CharacterSyncContext,
    CharacterSyncExtraction,
};
use crate::actions::workflow_checkpoint::CompletedLlmCallCheckpointInput;
use crate::state::AppState;

use super::super::contracts::{
    CharacterSyncGenerationTrace, CharacterSyncStage, CharacterSyncStageExchange,
    CharacterSyncStageOutputs, CharacterSyncStageRun,
};
use super::super::parsing::{parse_character_sync_stage_output, validate_extraction_evidence};
use super::call::{CharacterSyncStageCall, run_character_sync_stage_call};
use super::rules::stage_temperature;

pub(in crate::actions::character_ai::sync_profile) async fn run_staged_character_sync(
    state: &AppState,
    project_id: i64,
    workflow_run_id: i64,
    context: &CharacterSyncContext,
    progress: CharacterProgressScope<'_>,
) -> Result<CharacterSyncStageRun, String> {
    let workflow_run_id = Some(workflow_run_id);
    let name = context.profile.name.as_str();
    let mut outputs = CharacterSyncStageOutputs::default();
    let mut previous_steps = json!({});
    let mut exchanges = Vec::new();
    let mut llm_trace_paths = Vec::new();

    for (index, stage) in CharacterSyncStage::ALL.iter().copied().enumerate() {
        let current = u32::try_from(index)
            .map_err(|_| format!("角色同步阶段进度索引超过 u32 范围：{index}"))?
            .checked_add(1)
            .ok_or_else(|| format!("角色同步阶段进度索引超过 u32 范围：{index}"))?;
        progress.report(
            current,
            CHARACTER_PROFILE_SYNC_STEPS,
            &stage.progress_message(name),
        )?;
        let CharacterSyncStageCall {
            workflow_run_id,
            call_key,
            request,
            input_fingerprint_json,
            response_content,
            response_usage_json,
            response_finish_reason,
            restored_parsed_output,
        } = run_character_sync_stage_call(
            state,
            project_id,
            workflow_run_id,
            context,
            stage,
            &previous_steps,
        )
        .await?;
        match parse_character_sync_stage_output(stage, &response_content, context, &mut outputs) {
            Ok(stage_value) => {
                if let Some(parsed_output) = restored_parsed_output {
                    if parsed_output != stage_value {
                        return Err(format!(
                            "{call_key} checkpoint 的 parsed_output 与 response_content 重新解析结果不一致"
                        ));
                    }
                } else {
                    let call_order = i32::try_from(index)
                        .map_err(|_| format!("角色同步阶段索引超过 i32 范围：{index}"))?
                        .checked_add(1)
                        .ok_or_else(|| format!("角色同步阶段 call_order 超过 i32 范围：{index}"))?;
                    state
                        .save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
                            workflow_run_id,
                            call_key: call_key.clone(),
                            call_order,
                            input_fingerprint_json: input_fingerprint_json.clone(),
                            checkpoint_json: json!({
                                "output": stage_value.clone(),
                                "parsed_output": stage_value.clone(),
                                "response_content": response_content.clone(),
                                "usage": response_usage_json,
                                "finish_reason": response_finish_reason}),
                            artifacts_json: None,
                        })
                        .await
                        .map_err(|err| err.to_string())?;
                }
                if let Some(path) = record_character_sync_llm_trace(
                    state,
                    name,
                    &format!("{}_success", stage.key()),
                    &request.messages,
                    &response_content,
                    Some(stage_temperature(stage)),
                    None,
                ) {
                    llm_trace_paths.push(path);
                }
                previous_steps[stage.key()] = stage_value;
                exchanges.push(CharacterSyncStageExchange {
                    stage,
                    request,
                    response_content,
                });
            }
            Err(error) => {
                if let Some(path) = record_character_sync_llm_trace(
                    state,
                    name,
                    &format!("{}_invalid", stage.key()),
                    &request.messages,
                    &response_content,
                    Some(stage_temperature(stage)),
                    Some(&error),
                ) {
                    llm_trace_paths.push(path);
                }
                let trace_summary = if llm_trace_paths.is_empty() {
                    String::new()
                } else {
                    format!("；LLM 调试日志：{}", llm_trace_paths.join("，"))
                };
                return Err(format!(
                    "{}输出不符合角色同步阶段契约：{error}{trace_summary}",
                    stage.title()
                ));
            }
        }
    }

    progress.report(
        6,
        CHARACTER_PROFILE_SYNC_STEPS,
        &format!("{name}：正在组合并校验角色同步阶段结果"),
    )?;
    let extraction = validate_extraction_evidence(
        CharacterSyncExtraction {
            changes: outputs.changes,
            behaviors: outputs.behaviors,
            deletion_candidates: outputs.deletion_candidates,
            chapter_states: outputs.chapter_states,
        },
        context,
    )?;
    let generation_trace = CharacterSyncGenerationTrace::from_stage_exchanges(&exchanges)?;
    Ok(CharacterSyncStageRun {
        extraction,
        generation_trace,
        llm_trace_paths,
    })
}
