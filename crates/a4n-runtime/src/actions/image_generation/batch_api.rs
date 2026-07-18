use super::*;

pub async fn batch_generate_images(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    requests_json: Value,
    progress: Option<&ImageGenerationProgress<'_>>,
) -> Result<Vec<GeneratedImageSummary>, String> {
    let Some(scope) =
        batch_image_generation_task_scope(state, user_id, project_id, &requests_json).await?
    else {
        return Ok(vec![]);
    };
    let arr = requests_json
        .as_array()
        .ok_or_else(|| "requests_json 必须是数组".to_string())?;
    let workflow_run = start_batch_image_workflow(state, project_id, &requests_json).await?;
    let workflow_run_id = workflow_run.id;

    let result = async {
        let total = scope.total;
        let progress = progress.unwrap_or(&DEFAULT_IMAGE_GENERATION_PROGRESS);
        progress.progress(0, total, "正在准备批量图片请求");
        let jobs = build_batch_image_jobs(state, project_id, arr, true).await?;
        i32::try_from(jobs.len())
            .map_err(|_| "批量图片请求数量超出 checkpoint 序号范围".to_string())?;

        let concurrency = jobs
            .iter()
            .map(|job| job.config.max_concurrent.max(1) as usize)
            .min()
            .unwrap_or(1)
            .clamp(1, jobs.len().max(1));
        progress.progress(0, total, &format!("开始批量生成图片，并发 {concurrency}"));
        let completed = Arc::new(AtomicU32::new(0));
        let mut results = stream::iter(jobs)
            .map(|job| {
                let completed = Arc::clone(&completed);
                async move {
                    let display_index = job.index + 1;
                    progress.check_cancelled()?;
                    progress.progress(
                        completed.load(Ordering::Relaxed),
                        total,
                        &format!("正在生成图片 {display_index}/{}", arr.len()),
                    );
                    let request_json =
                        image_generation_request_trace_json(&job.config, &job.request);
                    let item_fingerprint =
                        batch_image_item_fingerprint(project_id, &job, &request_json);
                    let step_key = batch_image_item_step_key(job.index);
                    if let Some(summary) = load_completed_image_checkpoint(
                        state,
                        workflow_run_id,
                        &step_key,
                        &item_fingerprint,
                        project_id,
                    )
                    .await?
                    {
                        let next_current = completed.fetch_add(1, Ordering::SeqCst) + 1;
                        progress.progress(
                            next_current,
                            total,
                            &format!("已从断点恢复图片 {display_index}/{}", arr.len()),
                        );
                        return Ok::<_, String>((job.index, summary));
                    }
                    let req = job.request.clone();
                    let result = run_image_generation_provider_request_with_workflow(
                        state,
                        project_id,
                        &job.config,
                        req,
                        Some(workflow_run_id),
                        progress,
                    )
                    .await?;
                    progress.check_cancelled()?;
                    let extra = job.chapter_id.map(|c| format!("ch{c:02}"));
                    let now = chrono::Utc::now().naive_utc();
                    let img = save_generated_image_result(
                        state,
                        result,
                        GeneratedImageSaveInput {
                            project_id,
                            image_type: job.image_type,
                            command: "batch_generate_images",
                            config: &job.config,
                            file_extra: extra,
                            file_context: "批量图片",
                            prompt: job.request.prompt,
                            request_json,
                            width: job.request.width,
                            height: job.request.height,
                            chapter_id: job.chapter_id,
                            panel_id: job.panel_id,
                            metadata_json: job.metadata_json,
                            created_at: now,
                            workflow_run_id: Some(workflow_run_id),
                        },
                    )
                    .await?;
                    let summary = to_summary(&img);
                    let step_order = i32::try_from(job.index + 1)
                        .map_err(|_| "批量图片 checkpoint 序号超出 i32 范围".to_string())?;
                    save_completed_batch_image_checkpoint(
                        state,
                        workflow_run_id,
                        step_key,
                        step_order,
                        item_fingerprint,
                        &img,
                        &summary,
                    )
                    .await?;
                    let next_current = completed.fetch_add(1, Ordering::SeqCst) + 1;
                    progress.progress(
                        next_current,
                        total,
                        &format!("已生成图片 {display_index}/{}", arr.len()),
                    );
                    Ok::<_, String>((job.index, to_summary(&img)))
                }
            })
            .buffer_unordered(concurrency)
            .collect::<Vec<Result<(usize, GeneratedImageSummary), String>>>()
            .await;

        let mut ordered = Vec::with_capacity(results.len());
        for item in results.drain(..) {
            ordered.push(item?);
        }
        ordered.sort_by_key(|(index, _)| *index);
        Ok(ordered.into_iter().map(|(_, image)| image).collect())
    }
    .await;

    if let Err(status_err) = finalize_batch_image_workflow(state, workflow_run_id, &result).await {
        return match result {
            Ok(_) => Err(format!(
                "批量图片生成完成，但工作流状态写入失败: {status_err}"
            )),
            Err(err) => Err(format!("{err}；工作流失败状态写入失败: {status_err}")),
        };
    }
    result
}
