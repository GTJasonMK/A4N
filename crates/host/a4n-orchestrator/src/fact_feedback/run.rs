use a4n_api_contract::fact_feedback::RunProjectFactFeedbackInput;
use a4n_chapter_analysis_host_contract::{
    ChapterAnalysisApplicationProgress, ChapterAnalysisHostPort,
};
use a4n_character_host_contract::CharacterProfileSyncStageTracePort;
use a4n_domain::{fact_feedback::FactConsolidationOutput, workflow::WORKFLOW_STATUS_RUNNING};
use a4n_error::AppError;
use a4n_prompt::rendering::PromptRenderer;
#[cfg(test)]
use async_trait::async_trait;
use serde_json::{Value, json};

use super::{
    FACT_APPLY_STEP, FACT_CONSOLIDATION_KIND, FACT_CONSOLIDATION_PROMPT, FACT_REBUILD_KIND,
    business_key, feedback_error, request::PreparedConsolidation,
};
use crate::{
    OrchestratorContext,
    workflow::{
        CompletedLlmCallCheckpoint, CompletedLlmCallCheckpointInput, UpsertWorkflowCheckpointInput,
        UpsertWorkflowRunInput,
    },
};

impl OrchestratorContext {
    pub(crate) async fn consolidate_facts_if_due(
        &self,
        project_id: i64,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn ChapterAnalysisHostPort,
        progress: ChapterAnalysisApplicationProgress<'_>,
    ) -> Result<Option<i64>, AppError> {
        self.consolidate_facts(
            project_id,
            false,
            None,
            prompt_renderer,
            host_port,
            progress,
        )
        .await
    }

    pub(crate) async fn consolidate_facts(
        &self,
        project_id: i64,
        mut force: bool,
        resume_id: Option<i64>,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn ChapterAnalysisHostPort,
        progress: ChapterAnalysisApplicationProgress<'_>,
    ) -> Result<Option<i64>, AppError> {
        let mut last_run_id = None;
        if let Some(id) = resume_id {
            let run = self
                .feedback_resume_run(project_id, id, FACT_CONSOLIDATION_KIND)
                .await?;
            let applied = self
                .workflow_checkpoint_summaries(run.id)
                .await?
                .iter()
                .any(|checkpoint| {
                    checkpoint.step_key == FACT_APPLY_STEP && checkpoint.status == "completed"
                });
            // Prepare before reopening the run, so a stale source cannot strand it in Running.
            let prepared = if applied {
                None
            } else {
                Some(
                    self.prepare_consolidation(project_id, true, prompt_renderer)
                        .await?
                        .ok_or_else(|| {
                            AppError::InvalidArgument(
                                "没有可归纳的已完成正文事实，请先分析章节".into(),
                            )
                        })?,
                )
            };
            let run = self.mark_workflow_running_validated(run.id).await?;
            let result = if let Some(prepared) = prepared {
                self.execute_consolidation(project_id, &run, prepared, host_port, progress)
                    .await
            } else {
                self.finish_consolidation_index(project_id, run.id, host_port)
                    .await
            };
            self.finalize_workflow_result(run.id, result, "正文事实归纳")
                .await?;
            last_run_id = Some(run.id);
            force = false;
        }
        loop {
            if let Some(run) = self
                .latest_workflow_run_by_business_key(&business_key(
                    project_id,
                    FACT_CONSOLIDATION_KIND,
                ))
                .await?
            {
                if run.status == WORKFLOW_STATUS_RUNNING {
                    if !force {
                        return Ok(Some(run.id));
                    }
                    self.wait_for_fact_feedback(run.id, progress).await?;
                    last_run_id = Some(run.id);
                } else if run.status == "failed"
                    && run.can_resume
                    && self
                        .workflow_checkpoint_summaries(run.id)
                        .await?
                        .iter()
                        .any(|checkpoint| {
                            checkpoint.step_key == FACT_APPLY_STEP
                                && checkpoint.status == "completed"
                        })
                {
                    let run = self.mark_workflow_running_validated(run.id).await?;
                    let result = self
                        .finish_consolidation_index(project_id, run.id, host_port)
                        .await;
                    self.finalize_workflow_result(run.id, result, "正文事实索引重试")
                        .await?;
                    last_run_id = Some(run.id);
                    continue;
                }
            }
            let prepared = match self
                .prepare_consolidation(project_id, force, prompt_renderer)
                .await
            {
                Ok(prepared) => prepared,
                Err(error) => {
                    let run = self
                        .begin_or_resume_workflow_run_validated(workflow_input(
                            project_id,
                            FACT_CONSOLIDATION_KIND,
                            json!({"schema_version":1,"stage":"prepare"}),
                            progress.active_task_id,
                        ))
                        .await?;
                    return self
                        .finalize_workflow_result(run.id, Err(error), "正文事实归纳准备")
                        .await;
                }
            };
            let Some(prepared) = prepared else {
                if force && last_run_id.is_none() {
                    return Err(AppError::InvalidArgument(
                        "没有可归纳的已完成正文事实，请先分析章节".into(),
                    ));
                }
                return Ok(last_run_id);
            };
            let run = match self
                .begin_or_resume_workflow_run_validated(workflow_input(
                    project_id,
                    FACT_CONSOLIDATION_KIND,
                    prepared.fingerprint.clone(),
                    progress.active_task_id,
                ))
                .await
            {
                Ok(run) => run,
                Err(error) => {
                    // Another chapter may have claimed the project while this request was prepared.
                    if self
                        .latest_workflow_run_by_business_key(&business_key(
                            project_id,
                            FACT_CONSOLIDATION_KIND,
                        ))
                        .await?
                        .is_some_and(|run| run.status == WORKFLOW_STATUS_RUNNING)
                    {
                        continue;
                    }
                    return Err(error);
                }
            };
            let result = self
                .execute_consolidation(project_id, &run, prepared, host_port, progress)
                .await;
            self.finalize_workflow_result(run.id, result, "正文事实归纳")
                .await?;
            last_run_id = Some(run.id);
            // Recheck after completion: chapters finishing during this run must not miss their cadence.
            force = false;
        }
    }

    async fn wait_for_fact_feedback(
        &self,
        run_id: i64,
        progress: ChapterAnalysisApplicationProgress<'_>,
    ) -> Result<(), AppError> {
        progress.report(5, "正在等待当前正文事实归纳完成")?;
        loop {
            if progress
                .cancel_token
                .is_some_and(|token| token.is_cancelled())
            {
                return Err(AppError::Cancelled);
            }
            let run = self.workflow_run_model_by_id(run_id).await?;
            match run.status.as_str() {
                "running" => tokio::time::sleep(std::time::Duration::from_millis(250)).await,
                "completed" => return Ok(()),
                _ => {
                    return Err(AppError::InvalidState {
                        current: run.status,
                        operation: run
                            .error_message
                            .unwrap_or_else(|| "当前正文事实归纳未完成，请继续或重新启动".into()),
                    });
                }
            }
        }
    }

    async fn execute_consolidation(
        &self,
        project_id: i64,
        run: &a4n_domain::workflow::WorkflowRunSummary,
        mut prepared: PreparedConsolidation,
        host_port: &dyn ChapterAnalysisHostPort,
        progress: ChapterAnalysisApplicationProgress<'_>,
    ) -> Result<(), AppError> {
        progress.report(10, "正在归纳正文事实变化")?;
        self.mark_workflow_current_step(run.id, FACT_CONSOLIDATION_PROMPT)
            .await?;
        crate::workflow::attach_llm_call_checkpoint_trace(
            &mut prepared.request,
            run.id,
            FACT_CONSOLIDATION_PROMPT,
            &prepared.fingerprint,
        )?;
        let output = if let Some(saved) = self
            .load_completed_llm_call_checkpoint(
                run.id,
                FACT_CONSOLIDATION_PROMPT,
                &prepared.fingerprint,
            )
            .await?
        {
            self.validate_llm_call_output(
                run.id,
                FACT_CONSOLIDATION_PROMPT,
                &prepared.fingerprint,
                parse_output(saved.parsed_output),
            )
            .await?
        } else {
            let response = crate::generation::stream::execute_llm_chat_response(
                self.llm_backend(),
                prepared.request,
                progress.cancel_token.cloned().unwrap_or_default(),
            )
            .await?;
            if a4n_llm_provider_contract::provider::is_truncated_finish_reason(
                response.finish_reason.as_deref(),
            ) {
                return Err(AppError::InvalidArgument(
                    "正文事实归纳输出被截断，请提高输出预算或减小归纳周期".into(),
                ));
            }
            let value: Value = crate::structured_llm::parse_strict_json_response(
                &response.message.content,
                FACT_CONSOLIDATION_PROMPT,
            )?;
            let value = prepared.source_markers.decode_output(&value)?;
            let output = parse_output(value.clone())?;
            self.save_completed_llm_call_checkpoint(CompletedLlmCallCheckpointInput {
                workflow_run_id: run.id,
                call_key: FACT_CONSOLIDATION_PROMPT.into(),
                call_order: 1,
                input_fingerprint_json: prepared.fingerprint.clone(),
                checkpoint: CompletedLlmCallCheckpoint {
                    parsed_output: value,
                    response_content: response.message.content,
                    usage: response
                        .usage
                        .as_ref()
                        .map(serde_json::to_value)
                        .transpose()?,
                    finish_reason: response.finish_reason,
                    artifacts_json: None,
                },
            })
            .await?;
            output
        };
        progress.report(70, "正在校验来源并写入资料变更")?;
        if self.workflow_run_model_by_id(run.id).await?.status != WORKFLOW_STATUS_RUNNING {
            return Err(AppError::Cancelled);
        }
        let applied = a4n_db::fact_feedback::apply_consolidation(
            self.database.connection(),
            a4n_db::fact_feedback::ConsolidationApplyInput {
                project_id,
                workflow_run_id: run.id,
                expected_blueprint_version: prepared.expected_blueprint_version,
                sources: prepared.sources,
                changes: output.changes,
            },
        )
        .await;
        let applied = match applied {
            Ok(result) => result,
            Err(error @ a4n_db::fact_feedback::FactFeedbackDbError::Invalid(_)) => {
                return self
                    .validate_llm_call_output(
                        run.id,
                        FACT_CONSOLIDATION_PROMPT,
                        &prepared.fingerprint,
                        Err(feedback_error(error)),
                    )
                    .await;
            }
            Err(error) => return Err(feedback_error(error)),
        };
        if let Some(version) = applied.blueprint_version {
            self.publish_internal_event(a4n_event::InternalEvent::BlueprintUpdated {
                project_id,
                version,
            })?;
        }
        progress.report(90, "正在同步事实资料检索索引")?;
        self.finish_consolidation_index(project_id, run.id, host_port)
            .await?;
        progress.report(100, "正文事实归纳完成")
    }

    async fn finish_consolidation_index(
        &self,
        project_id: i64,
        run_id: i64,
        host_port: &dyn ChapterAnalysisHostPort,
    ) -> Result<(), AppError> {
        let checkpoint = self
            .workflow_checkpoint_summaries(run_id)
            .await?
            .into_iter()
            .find(|checkpoint| {
                checkpoint.step_key == FACT_APPLY_STEP && checkpoint.status == "completed"
            })
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_fact_apply_checkpoint".into(),
                operation: "同步归纳资料索引".into(),
            })?;
        let value = checkpoint
            .checkpoint_json
            .ok_or_else(|| AppError::InvalidArgument("事实写入断点缺少结果".into()))?;
        host_port
            .sync_fact_material_rag_records(
                project_id,
                run_id,
                value.get("blueprint_version").is_some_and(Value::is_string),
                value.get("characters_changed").and_then(Value::as_bool) == Some(true),
            )
            .await
    }

    async fn feedback_resume_run(
        &self,
        project_id: i64,
        id: i64,
        kind: &str,
    ) -> Result<a4n_domain::workflow::WorkflowRunSummary, AppError> {
        let run = self.workflow_run_model_by_id(id).await?;
        if run.project_id != Some(project_id) || run.workflow_kind != kind {
            return Err(AppError::InvalidArgument(
                "事实反馈工作流不属于当前项目或操作".into(),
            ));
        }
        a4n_domain::workflow::require_workflow_can_resume(
            &run.status,
            run.can_resume,
            run.resume_block_reason.as_deref(),
        )
        .map_err(AppError::InvalidArgument)?;
        let latest = self
            .latest_workflow_run_by_business_key(&business_key(project_id, kind))
            .await?;
        if latest.as_ref().map(|current| current.id) != Some(id) {
            return Err(AppError::InvalidArgument(
                "该事实反馈已被新的运行替代".into(),
            ));
        }
        Ok(run)
    }

    pub(super) async fn rebuild_facts(
        &self,
        user_id: i64,
        input: &RunProjectFactFeedbackInput,
        prompt_renderer: &dyn PromptRenderer,
        host_port: &dyn ChapterAnalysisHostPort,
        progress: ChapterAnalysisApplicationProgress<'_>,
        trace: Option<&dyn CharacterProfileSyncStageTracePort>,
    ) -> Result<(), AppError> {
        let chapters: Vec<_> = self
            .chapters_for_project(input.project_id)
            .await?
            .into_iter()
            .filter(|chapter| chapter.has_available_content())
            .collect();
        if chapters.is_empty() {
            return Err(AppError::InvalidArgument("项目没有可分析正文".into()));
        }
        let fingerprint = json!({"sources":chapters.iter().map(|chapter| json!({"id":chapter.id,"revision":a4n_domain::chapter::chapter_edit_revision(&chapter.content, chapter.title.as_deref())})).collect::<Vec<_>>()});
        let run = if let Some(id) = input.workflow_run_id {
            let run = self
                .feedback_resume_run(input.project_id, id, FACT_REBUILD_KIND)
                .await?;
            if run.input_fingerprint_json.as_ref() != Some(&fingerprint) {
                return Err(AppError::InvalidArgument(
                    "正文已变化，请重新启动事实重建".into(),
                ));
            }
            self.mark_workflow_running_validated(id).await?
        } else {
            self.begin_or_resume_workflow_run_validated(workflow_input(
                input.project_id,
                FACT_REBUILD_KIND,
                fingerprint,
                progress.active_task_id,
            ))
            .await?
        };
        let result = async {
            let completed = self.workflow_checkpoint_summaries(run.id).await?;
            for (index, chapter) in chapters.iter().enumerate() {
                let key = format!("fact_rebuild.chapter.{}", chapter.id);
                if completed.iter().any(|checkpoint| {
                    checkpoint.step_key == key && checkpoint.status == "completed"
                }) {
                    continue;
                }
                progress.report(
                    (index * 90 / chapters.len()) as u32,
                    &format!("正在重建第 {} 章正文事实", chapter.chapter_number),
                )?;
                if self.workflow_run_model_by_id(run.id).await?.status != WORKFLOW_STATUS_RUNNING {
                    return Err(AppError::Cancelled);
                }
                self.run_chapter_analysis_validated(
                    user_id,
                    chapter.id,
                    prompt_renderer,
                    host_port,
                    ChapterAnalysisApplicationProgress {
                        reporter: None,
                        ..progress
                    },
                    trace,
                )
                .await?;
                self.upsert_workflow_checkpoint(UpsertWorkflowCheckpointInput {
                    workflow_run_id: run.id,
                    step_key: key,
                    step_order: (index + 1) as i32,
                    status: "completed".into(),
                    input_fingerprint_sha256: None,
                    input_fingerprint_json: Some(json!({"chapter_id":chapter.id})),
                    checkpoint_json: Some(json!({"completed":true})),
                    artifacts_json: None,
                    generation_run_ids_json: None,
                    error_message: None,
                    completed: true,
                })
                .await?;
            }
            self.consolidate_facts(
                input.project_id,
                true,
                None,
                prompt_renderer,
                host_port,
                progress,
            )
            .await?;
            Ok(())
        }
        .await;
        self.finalize_workflow_result(run.id, result, "正文事实重建")
            .await
    }
}

fn parse_output(value: Value) -> Result<FactConsolidationOutput, AppError> {
    let output: FactConsolidationOutput = serde_json::from_value(value)
        .map_err(|error| AppError::InvalidArgument(format!("事实归纳结构无效: {error}")))?;
    for change in &output.changes {
        change.validate().map_err(AppError::InvalidArgument)?;
    }
    Ok(output)
}

fn workflow_input(
    project_id: i64,
    kind: &str,
    fingerprint: Value,
    task_id: Option<&str>,
) -> UpsertWorkflowRunInput {
    UpsertWorkflowRunInput {
        business_key: business_key(project_id, kind),
        workflow_kind: kind.into(),
        project_id: Some(project_id),
        root_project_id: None,
        source_snapshot_id: None,
        scope_table: Some("novel_project".into()),
        scope_id: Some(project_id),
        chapter_id: None,
        mode: None,
        input_fingerprint_json: Some(fingerprint),
        status: WORKFLOW_STATUS_RUNNING.into(),
        current_step_key: Some(kind.into()),
        active_task_id: task_id.map(str::to_string),
        can_resume: true,
        resume_block_reason: None,
        error_message: None,
    }
}

#[cfg(test)]
mod tests;
