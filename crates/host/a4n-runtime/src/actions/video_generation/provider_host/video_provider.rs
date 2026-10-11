use a4n_canvas_host_contract::{CanvasVideoFileInput, CanvasVideoGenerationHostPort};
use a4n_domain::media::{GeneratedMediaArtifactFile, video_mime_from_path};
use a4n_video::{VideoJobState, is_canvas_video_record_fields};
use a4n_video_host_contract::{
    PersistedVideoResult, VideoCancellationHandle, VideoProviderHostPort, VideoProviderResultTarget,
};
use async_trait::async_trait;
use serde_json::json;

use crate::{
    actions::video_generation::{
        job_lifecycle::{persist_video_result, poll_provider_job},
        media::delete_video_tail_frame_assets,
    },
    cleanup_outbox::enqueue_file_artifact_cleanup_by_project,
    file_utils::inspect_generated_media_file,
};

use super::*;

#[async_trait]
impl VideoProviderHostPort for RuntimeVideoProviderHostPort<'_> {
    fn inspect_video_file(
        &self,
        relative_path: &str,
        label: &str,
    ) -> Result<GeneratedMediaArtifactFile, AppError> {
        inspect_generated_media_file(
            self.state.data_dir(),
            relative_path,
            label,
            video_mime_from_path,
        )
        .map_err(host_error)
    }

    async fn poll_video_job(
        &self,
        project_id: i64,
        workflow_run_id: i64,
        config: &VideoProviderConfig,
        job_id: &str,
        cancel_token: &VideoCancellationHandle,
    ) -> Result<VideoJobState, AppError> {
        let root_project_id = video_sidecar_root_project_id(
            self.state.runtime_profile(),
            self.state.project_access_application(),
            project_id,
        )
        .await
        .map_err(host_error)?;
        let provider_cancel = CancellationToken::new();
        let poll = poll_provider_job(
            self.state,
            config,
            project_id,
            root_project_id,
            job_id,
            workflow_run_id,
            provider_cancel.clone(),
        );
        tokio::pin!(poll);
        let result = tokio::select! {
            result = &mut poll => result,
            _ = cancel_token.cancelled() => {
                provider_cancel.cancel();
                (&mut poll).await
            }
        };
        result.map_err(host_error)
    }

    async fn persist_video_result(
        &self,
        target: &VideoProviderResultTarget,
        _prompt_id: Option<i64>,
        mut result: VideoGenerationResult,
    ) -> Result<PersistedVideoResult, AppError> {
        result.metadata = super::super::preview::completed_video_artifact_metadata(&result);
        if is_canvas_video_record_fields(target.chapter_id, target.workflow_mode.as_deref()) {
            let persisted = self
                .persist_canvas_video_file(CanvasVideoFileInput {
                    project_id: target.project_id,
                    video_id: target.video_id,
                    bytes: &result.bytes,
                    format: &result.format,
                })
                .await?;
            return Ok(PersistedVideoResult {
                video_id: target.video_id,
                project_id: target.project_id,
                file_path: persisted.relative_path,
                thumbnail_path: None,
                properties: persisted.properties,
                metadata_json: Some(json!({
                    "provider": result.metadata,
                    "canvas_video": true,
                })),
            });
        }
        persist_video_result(self.state, target, result)
            .await
            .map_err(host_error)
    }

    async fn remove_video_result(&self, persisted: &PersistedVideoResult) -> Result<(), AppError> {
        // 先写 durable cleanup intent，再做同步尾帧清理。
        // 进程若在 DB 失败窗口崩溃，outbox 仍可跨重启删除正式视频文件。
        let mut paths = vec![persisted.file_path.clone()];
        if let Some(path) = persisted.thumbnail_path.clone() {
            paths.push(path);
        }
        enqueue_file_artifact_cleanup_by_project(
            self.state,
            &format!(
                "video-provider-result:{}:{}",
                persisted.project_id, persisted.video_id
            ),
            &paths,
        )
        .await
        .map_err(host_error)?;
        if let Err(error) = delete_video_tail_frame_assets(self.state, persisted.video_id).await {
            return Err(host_error(format!(
                "视频文件 cleanup intent 已写入，但尾帧同步清理失败：{error}"
            )));
        }
        Ok(())
    }
}
