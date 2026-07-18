use a4n_core::models::ai::generated_video;
use a4n_orchestrator::workflow::WORKFLOW_STATUS_COMPLETED;
use a4n_video::VideoGenerationResult;

use super::super::context::{
    completed_provider_metadata, is_first_frame_chain_video, required_video_chapter_id,
    should_capture_video_tail_frame,
};
use super::super::workflow::mark_video_provider_workflow;
use super::failure::{fail_saved_video_result, mark_video_failed_and_update_prompt};
use super::runs::complete_video_generation_runs;
use crate::AppState;
use crate::actions::video_generation::media::{
    best_effort_video_thumbnail_path, capture_video_tail_frame_asset, save_video_bytes,
};
use crate::file_utils::{data_dir_relative_file_path, remove_created_file_on_error};

pub(in crate::actions::video_generation) async fn save_completed_video_result(
    state: &AppState,
    video: &generated_video::Model,
    prompt_id: Option<i64>,
    result: VideoGenerationResult,
) -> Result<generated_video::Model, String> {
    let prompt_id =
        prompt_id.or_else(|| crate::actions::video_generation::media::prompt_id_from_video(video));
    if result.bytes.is_empty() {
        let updated = mark_video_failed_and_update_prompt(
            state,
            video.id,
            prompt_id,
            "视频 provider 返回了空文件，无法保存生成结果",
        )
        .await?;
        return Ok(updated);
    }

    let chapter_id = required_video_chapter_id(video.chapter_id, video.id)?;
    let path = match save_video_bytes(
        state,
        video.project_id,
        chapter_id,
        video.id,
        &result.bytes,
        &result.format,
    ) {
        Ok(path) => path,
        Err(msg) => {
            let updated =
                mark_video_failed_and_update_prompt(state, video.id, prompt_id, &msg).await?;
            return Ok(updated);
        }
    };
    let persisted_path = path.clone();
    let video_path_str = remove_created_file_on_error(&path, "已写出的视频文件", async {
        data_dir_relative_file_path(&state.config.data_dir, &persisted_path, "视频文件")
    })
    .await?;
    let thumbnail_path = best_effort_video_thumbnail_path(
        &state.config.data_dir,
        video.project_id,
        chapter_id,
        video.id,
        &path,
    );
    let mut tail_frame_capture_error = None;
    let tail_frame_metadata = if should_capture_video_tail_frame(video) {
        match capture_video_tail_frame_asset(state, video, chapter_id, prompt_id, &path).await {
            Ok(metadata) => Some(metadata),
            Err(msg) => {
                if is_first_frame_chain_video(video) {
                    let msg = format!("视频已生成但首帧串联尾帧采集失败：{msg}");
                    return fail_saved_video_result(
                        state,
                        video.id,
                        prompt_id,
                        &path,
                        thumbnail_path.as_deref(),
                        msg,
                    )
                    .await;
                }
                tail_frame_capture_error = Some(msg);
                None
            }
        }
    } else {
        None
    };
    let completed_metadata = completed_provider_metadata(
        result.metadata,
        tail_frame_metadata,
        tail_frame_capture_error,
    );
    let updated = match state
        .orchestrator
        .mark_generated_video_completed_and_link_prompt(
            video.id,
            &video_path_str,
            thumbnail_path.as_deref(),
            result.bytes.len() as i64,
            completed_metadata,
            prompt_id,
        )
        .await
    {
        Ok(updated) => updated,
        Err(err) => {
            let msg = format!("视频已生成但记录完成状态失败：{err}");
            return fail_saved_video_result(
                state,
                video.id,
                prompt_id,
                &path,
                thumbnail_path.as_deref(),
                msg,
            )
            .await;
        }
    };
    complete_video_generation_runs(state, &updated).await?;
    mark_video_provider_workflow(state, &updated, WORKFLOW_STATUS_COMPLETED, false, None).await?;
    Ok(updated)
}
