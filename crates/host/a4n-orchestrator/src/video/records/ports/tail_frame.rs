//! 尾帧 port。

use super::*;
use async_trait::async_trait;

#[async_trait]
impl VideoTailFrameApplicationPort for crate::OrchestratorContext {
    async fn create_video_tail_frame(
        &self,
        input: VideoTailFrameInput,
    ) -> Result<VideoTailFrameSummary, AppError> {
        let video = self.required_video_record(input.video_id).await?;
        let chapter_id = video.chapter_id.ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "视频 #{} 缺少 chapter_id，无法登记真实尾帧",
                video.id
            ))
        })?;
        let shot_number = video_shot_number(&video).map_err(AppError::InvalidArgument)?;
        let image = self
            .create_video_tail_frame_record_validated(
                &video,
                chapter_id,
                input.file_path,
                input.file_size,
            )
            .await?;
        Ok(VideoTailFrameSummary {
            image_id: image.id,
            file_path: image.file_path,
            shot_number,
        })
    }

    async fn delete_video_tail_frame_assets(&self, video_id: i64) -> Result<(), AppError> {
        let images = self
            .video_tail_frame_images()
            .await?
            .into_iter()
            .filter(|(_, image_type, metadata_json)| {
                tail_frame_metadata_belongs_to_video(image_type, metadata_json.as_ref(), video_id)
            })
            .collect::<Vec<_>>();
        let mut failures = Vec::new();
        for (image_id, _, _) in images {
            if let Err(error) = self
                .delete_generated_image_record_with_cleanup(image_id)
                .await
            {
                failures.push(format!("尾帧图片 #{image_id} 删除失败：{error}"));
            }
        }
        if failures.is_empty() {
            Ok(())
        } else {
            Err(AppError::Internal(anyhow::anyhow!(failures.join("；"))))
        }
    }
}
