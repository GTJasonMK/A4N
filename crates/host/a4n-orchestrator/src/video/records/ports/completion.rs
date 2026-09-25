//! 视频完成 port。

use super::*;
use async_trait::async_trait;

#[async_trait]
impl VideoRecordCompletionApplicationPort for crate::OrchestratorContext {
    async fn complete(
        &self,
        input: CompleteVideoRecordInput,
    ) -> Result<VideoRecordCompletion, VideoRecordCompletionError> {
        let CompleteVideoRecordInput {
            record_id,
            output_path,
            thumbnail_path,
            properties,
            prompt_id,
            metadata_json,
        } = input;
        let record = match self
            .mark_generated_video_completed_and_link_prompt(
                record_id,
                &output_path,
                thumbnail_path.as_deref(),
                &properties,
                metadata_json,
                prompt_id,
            )
            .await
        {
            Ok(record) => record,
            Err(error) => {
                let can_adopt = crate::video::can_adopt_authoritative_video_completion(&error);
                let authoritative = match self.generated_video_by_id(record_id).await {
                    Ok(authoritative) => authoritative,
                    Err(lookup_error) => {
                        return Err(VideoRecordCompletionError::Submission {
                            error,
                            authoritative: None,
                            lookup_error: Some(lookup_error),
                        });
                    }
                };
                if can_adopt
                    && let Some(authoritative) = authoritative.as_ref()
                    && authoritative.status == a4n_domain::video_workflow::VIDEO_STATUS_COMPLETED
                {
                    return Ok(VideoRecordCompletion {
                        summary: generated_video_summary(authoritative),
                        adopted_authoritative: true,
                    });
                }
                return Err(VideoRecordCompletionError::Submission {
                    error,
                    authoritative: authoritative.map(|record| VideoRecordCompletion {
                        summary: generated_video_summary(&record),
                        adopted_authoritative: false,
                    }),
                    lookup_error: None,
                });
            }
        };
        Ok(VideoRecordCompletion {
            summary: generated_video_summary(&record),
            adopted_authoritative: false,
        })
    }
}
