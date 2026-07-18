use super::super::{
    AppState, GenerateChapterVideoInput, GenerateChapterVideosBatchInput,
    GenerateChapterVideosBatchResult, VIDEO_GENERATION_PROGRESS_TOTAL,
    VideoGenerationProgressCallback, ensure_provider_generation_workflow, has_active_provider_job,
};
use super::api::generate_chapter_video;
use super::scope::video_generation_task_scope;
use tokio_util::sync::CancellationToken;

pub async fn generate_chapter_videos_batch(
    state: &AppState,
    input: GenerateChapterVideosBatchInput,
    mut progress: Option<VideoGenerationProgressCallback<'_>>,
    cancel_token: CancellationToken,
) -> Result<GenerateChapterVideosBatchResult, String> {
    let prompt_ids = validate_chapter_video_batch_prompts(state, &input).await?;
    let item_total = VIDEO_GENERATION_PROGRESS_TOTAL;
    let prompt_count = u32::try_from(prompt_ids.len())
        .map_err(|_| "批量视频提示词数量超出 u32 范围".to_string())?;
    let total = prompt_count
        .checked_mul(item_total)
        .ok_or_else(|| "批量视频进度总量超出 u32 范围".to_string())?
        .max(1);
    let mut videos = Vec::new();
    let mut failed_count = 0usize;
    let mut pending_count = 0usize;
    for (index, prompt_id) in prompt_ids.iter().copied().enumerate() {
        if cancel_token.is_cancelled() {
            return Err("视频批量生成已停止".to_string());
        }
        let base = u32::try_from(index)
            .map_err(|_| "批量视频进度索引超出 u32 范围".to_string())?
            .checked_mul(item_total)
            .ok_or_else(|| "批量视频进度索引超出 u32 范围".to_string())?;
        if let Some(reporter) = progress.as_deref_mut() {
            reporter(
                base,
                total,
                &format!("正在生成第 {}/{} 条视频", index + 1, prompt_ids.len()),
            )
            .map_err(|err| err.to_string())?;
        }
        let video = {
            let mut mapped_progress = |current: u32, child_total: u32, message: &str| {
                let child_total = child_total.max(1);
                let scaled_current = if child_total == item_total {
                    current.min(child_total)
                } else {
                    let scaled = (u64::from(current.min(child_total)) * u64::from(item_total))
                        / u64::from(child_total);
                    u32::try_from(scaled).expect("scaled video progress is bounded by item_total")
                };
                if let Some(reporter) = progress.as_deref_mut() {
                    reporter(
                        (base + scaled_current).min(total),
                        total,
                        &format!("第 {}/{} 条视频：{}", index + 1, prompt_ids.len(), message),
                    )?;
                }
                Ok(())
            };
            generate_chapter_video(
                state,
                GenerateChapterVideoInput {
                    user_id: input.user_id,
                    project_id: input.project_id,
                    chapter_id: input.chapter_id,
                    prompt_id,
                    config_id: input.config_id,
                },
                Some(&mut mapped_progress),
                cancel_token.clone(),
            )
            .await?
        };
        if video.status == "failed" {
            failed_count += 1;
        }
        if has_active_provider_job(video.provider_job_id.as_deref(), &video.status) {
            pending_count += 1;
        }
        videos.push(video);
        if let Some(reporter) = progress.as_deref_mut() {
            reporter(
                u32::try_from(index + 1)
                    .map_err(|_| "批量视频进度索引超出 u32 范围".to_string())?
                    .checked_mul(item_total)
                    .ok_or_else(|| "批量视频进度索引超出 u32 范围".to_string())?
                    .min(total),
                total,
                &format!("第 {}/{} 条视频处理完成", index + 1, prompt_ids.len()),
            )
            .map_err(|err| err.to_string())?;
        }
    }
    let success_count = videos.len().checked_sub(failed_count).ok_or_else(|| {
        format!(
            "批量视频统计不一致：视频数 {} 小于失败数 {failed_count}",
            videos.len()
        )
    })?;
    Ok(GenerateChapterVideosBatchResult {
        success_count,
        failed_count,
        pending_count,
        videos,
    })
}

async fn validate_chapter_video_batch_prompts(
    state: &AppState,
    input: &GenerateChapterVideosBatchInput,
) -> Result<Vec<i64>, String> {
    if input.prompt_ids.is_empty() {
        return Err("批量生成视频至少需要选择一条视频提示词".to_string());
    }
    video_generation_task_scope(state, input.user_id, input.project_id).await?;
    let mut unique_prompt_ids = input.prompt_ids.clone();
    unique_prompt_ids.sort_unstable();
    unique_prompt_ids.dedup();
    if unique_prompt_ids.len() != input.prompt_ids.len() {
        return Err("批量生成视频 prompt_ids 不能重复".to_string());
    }
    let rows = state
        .orchestrator
        .chapter_video_prompts_for_batch_generation(&unique_prompt_ids)
        .await
        .map_err(crate::actions::app_error_message)?;
    if rows.len() != unique_prompt_ids.len() {
        return Err("部分视频提示词不存在，不能批量生成视频".to_string());
    }
    for row in &rows {
        if row.project_id != input.project_id || row.chapter_id != input.chapter_id {
            return Err(format!("视频提示词 #{} 不属于当前项目章节", row.id));
        }
        if row.prompt_type != "video_prompt" {
            return Err(format!("视频提示词 #{} 不是 video_prompt 类型", row.id));
        }
        ensure_provider_generation_workflow(row)?;
        if row.status == "running" {
            return Err(format!("视频提示词 #{} 正在运行，不能批量生成视频", row.id));
        }
        if row.generated_video_id.is_some() {
            return Err(format!(
                "视频提示词 #{} 已关联视频，不能批量生成视频",
                row.id
            ));
        }
    }
    Ok(rows.into_iter().map(|row| row.id).collect())
}
