use std::collections::{HashMap, HashSet};

use a4n_api_contract::manga::MangaPipelineStatusSummary;
use a4n_manga_record_worker_contract::{
    MANGA_PIPELINE_STATUS_OPERATION, MANGA_PIPELINE_STATUSES_OPERATION, MangaChapterInput,
    MangaPipelineStatusesInput,
};
use a4n_worker_contract::WorkerEnvelope;

use super::{
    WorkerState,
    access::{user_id, verify_manga_viewer},
    protocol::{
        WorkerFailure, manga_error, positive, project_family_error, validate_request,
        workflow_error,
    },
};

impl WorkerState {
    pub(super) async fn pipeline_status(
        &self,
        request: WorkerEnvelope<MangaChapterInput>,
    ) -> Result<MangaPipelineStatusSummary, WorkerFailure> {
        validate_request(&request, MANGA_PIPELINE_STATUS_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        let mut statuses = self
            .pipeline_statuses_for(user_id, input.project_id, &[input.chapter_id])
            .await?;
        statuses
            .pop()
            .ok_or_else(|| WorkerFailure::invalid_request("章节状态列表为空"))
    }

    pub(super) async fn pipeline_statuses(
        &self,
        request: WorkerEnvelope<MangaPipelineStatusesInput>,
    ) -> Result<Vec<MangaPipelineStatusSummary>, WorkerFailure> {
        validate_request(&request, MANGA_PIPELINE_STATUSES_OPERATION)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        self.pipeline_statuses_for(user_id, input.project_id, &input.chapter_ids)
            .await
    }

    async fn pipeline_statuses_for(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_ids: &[i64],
    ) -> Result<Vec<MangaPipelineStatusSummary>, WorkerFailure> {
        positive(project_id, "project_id")?;
        verify_manga_viewer(&self.database, project_id, user_id).await?;
        if chapter_ids.is_empty() {
            return Err(WorkerFailure::invalid_request("chapter_ids 不能为空"));
        }
        if chapter_ids.len() > 200 {
            return Err(WorkerFailure::invalid_request(
                "chapter_ids 最多支持 200 个章节",
            ));
        }
        let mut seen = HashSet::with_capacity(chapter_ids.len());
        for chapter_id in chapter_ids {
            positive(*chapter_id, "chapter_id")?;
            if !seen.insert(*chapter_id) {
                return Err(WorkerFailure::invalid_request(format!(
                    "chapter_id 重复: {chapter_id}"
                )));
            }
        }

        let mut summaries = chapter_ids
            .iter()
            .copied()
            .map(|chapter_id| (chapter_id, empty_pipeline_status(chapter_id)))
            .collect::<HashMap<_, _>>();
        let (prompt_statuses, runs) = tokio::try_join!(
            async {
                a4n_db::manga::manga_page_prompt_statuses_by_chapters(
                    self.database.connection(),
                    project_id,
                    chapter_ids,
                )
                .await
                .map_err(manga_error)
            },
            async {
                a4n_db::workflow::list_workflow_runs(
                    self.database.connection(),
                    a4n_db::workflow::ListWorkflowRunsInput {
                        project_id: Some(project_id),
                        chapter_id: None,
                        chapter_ids: Some(chapter_ids.to_vec()),
                        workflow_kind: Some("manga_pipeline".to_string()),
                        status: None,
                        resumable_only: None,
                        limit: None,
                    },
                )
                .await
                .map_err(workflow_error)
            },
        )?;
        for (chapter_id, prompt_id, status) in prompt_statuses {
            let summary = summaries.get_mut(&chapter_id).ok_or_else(|| {
                WorkerFailure::invalid_request(format!("批量状态返回未知章节: {chapter_id}"))
            })?;
            match status.as_str() {
                "pending" => summary.pending += 1,
                "generating" => summary.generating += 1,
                "completed" => summary.completed += 1,
                "failed" => summary.failed += 1,
                "cancelled" => summary.cancelled += 1,
                status => {
                    return Err(WorkerFailure::invalid_request(format!(
                        "漫画页面提示词 #{} 包含未知状态: {status}",
                        prompt_id
                    )));
                }
            }
        }

        let mut latest_runs = HashMap::new();
        for run in runs {
            let chapter_id = run.chapter_id.ok_or_else(|| {
                WorkerFailure::invalid_request("漫画流水线 workflow 缺少 chapter_id")
            })?;
            latest_runs.entry(chapter_id).or_insert(run);
        }
        let run_ids = latest_runs.values().map(|run| run.id).collect::<Vec<_>>();
        let checkpoints = a4n_db::workflow::workflow_checkpoint_statuses_for_runs(
            self.database.connection(),
            &run_ids,
        )
        .await
        .map_err(workflow_error)?;
        let completed_checkpoint_runs = checkpoints
            .into_iter()
            .filter_map(|(run_id, status)| (status == "completed").then_some(run_id))
            .collect::<HashSet<_>>();
        let checkpoint_chapter_ids = latest_runs
            .iter()
            .filter_map(|(chapter_id, run)| {
                completed_checkpoint_runs
                    .contains(&run.id)
                    .then_some(*chapter_id)
            })
            .collect::<Vec<_>>();
        let (sources, character_portraits) = tokio::try_join!(
            async {
                a4n_db::project::chapter_manga_sources_by_ids(
                    self.database.connection(),
                    project_id,
                    &checkpoint_chapter_ids,
                    false,
                )
                .await
                .map_err(project_family_error)
            },
            async {
                if checkpoint_chapter_ids.is_empty() {
                    Ok(Vec::new())
                } else {
                    a4n_db::project::manga_character_portrait_refs(
                        self.database.connection(),
                        project_id,
                    )
                    .await
                    .map_err(project_family_error)
                }
            },
        )?;
        let sources_by_chapter = sources
            .into_iter()
            .map(|source| (source.chapter_id, source))
            .collect::<HashMap<_, _>>();
        for chapter_id in chapter_ids {
            let Some(run) = latest_runs.get(chapter_id) else {
                continue;
            };
            let summary = summaries.get_mut(chapter_id).ok_or_else(|| {
                WorkerFailure::invalid_request(format!("批量状态返回未知章节: {chapter_id}"))
            })?;
            summary.pipeline_status = Some(run.status.clone());
            summary.stage = run.current_step_key.clone();
            summary.task_id = run.active_task_id.clone();
            summary.can_cancel = run.status == "running";
            summary.message = run.error_message.clone();
            if let Some(progress) = run.progress.as_ref() {
                summary.current = Some(i32::try_from(progress.current).map_err(|_| {
                    WorkerFailure::invalid_request("漫画流水线 current 超出 i32 范围")
                })?);
                summary.total = Some(i32::try_from(progress.total).map_err(|_| {
                    WorkerFailure::invalid_request("漫画流水线 total 超出 i32 范围")
                })?);
                if summary.message.is_none() {
                    summary.message = Some(progress.message.clone());
                }
            }
            summary.has_checkpoint = completed_checkpoint_runs.contains(&run.id);
            if summary.has_checkpoint {
                let source = sources_by_chapter
                    .get(chapter_id)
                    .ok_or_else(|| WorkerFailure::not_found(format!("章节不存在: {chapter_id}")))?;
                let current_fingerprint = a4n_manga_core::manga_pipeline_fingerprint(
                    &source.content,
                    &character_portraits,
                );
                summary.checkpoint_valid =
                    run.input_fingerprint_json.as_ref() == Some(&current_fingerprint);
            }
            summary.can_resume = run.can_resume
                && summary.checkpoint_valid
                && matches!(run.status.as_str(), "failed" | "cancelled" | "interrupted");
        }
        chapter_ids
            .iter()
            .map(|chapter_id| {
                summaries.remove(chapter_id).ok_or_else(|| {
                    WorkerFailure::invalid_request(format!("批量状态缺少章节: {chapter_id}"))
                })
            })
            .collect()
    }
}

fn empty_pipeline_status(chapter_id: i64) -> MangaPipelineStatusSummary {
    MangaPipelineStatusSummary {
        chapter_id,
        pending: 0,
        generating: 0,
        completed: 0,
        failed: 0,
        cancelled: 0,
        pipeline_status: None,
        stage: None,
        current: None,
        total: None,
        message: None,
        task_id: None,
        can_cancel: false,
        can_resume: false,
        has_checkpoint: false,
        checkpoint_valid: false,
    }
}
