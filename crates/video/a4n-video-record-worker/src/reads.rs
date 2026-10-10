use std::collections::HashSet;

use super::*;
use a4n_worker_contract::WorkerEnvelope;

impl WorkerState {
    pub(super) async fn playback_source(
        &self,
        request: WorkerEnvelope<VideoIdInput>,
    ) -> Result<GeneratedVideoSummary, WorkerFailure> {
        validate_request(&request, VIDEO_RECORD_PLAYBACK_OPERATION)?;
        let video_id = WorkerFailure::require_positive_id(request.payload.video_id, "video_id")?;
        let video =
            a4n_db::video::read_models::generated_video_by_id(self.database.connection(), video_id)
                .await
                .map_err(database_error)?
                .ok_or_else(|| WorkerFailure::not_found("视频不存在"))?;
        validate_optional_project_scope(request.project_id, video.project_id)?;
        verify_project_permission(
            &self.database,
            video.project_id,
            user_id(&request)?,
            ProjectAccessRole::Editor,
        )
        .await?;
        Ok(video_summary(&video))
    }

    pub(super) async fn list_by_chapter(
        &self,
        request: WorkerEnvelope<ChapterVideosInput>,
    ) -> Result<Vec<GeneratedVideoSummary>, WorkerFailure> {
        validate_request(&request, VIDEO_RECORD_LIST_OPERATION)?;
        validate_required_project_scope(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        WorkerFailure::require_positive_id(input.project_id, "project_id")?;
        WorkerFailure::require_positive_id(input.chapter_id, "chapter_id")?;
        verify_project_permission(
            &self.database,
            input.project_id,
            user_id,
            ProjectAccessRole::Viewer,
        )
        .await?;
        a4n_db::video::ensure_video_chapter_belongs_to_project_record(
            self.database.connection(),
            input.project_id,
            input.chapter_id,
        )
        .await
        .map_err(video_error)?;
        a4n_db::video::read_models::generated_videos_for_chapter(
            self.database.connection(),
            input.project_id,
            input.chapter_id,
        )
        .await
        .map_err(database_error)
        .map(|videos| videos.iter().map(video_summary).collect())
    }

    pub(super) async fn validated_reference_images(
        &self,
        request: WorkerEnvelope<ReferenceImagesInput>,
    ) -> Result<Vec<GeneratedImageSummary>, WorkerFailure> {
        validate_request(&request, VIDEO_RECORD_REFERENCE_IMAGES_OPERATION)?;
        validate_required_project_scope(&request, request.payload.project_id)?;
        let user_id = user_id(&request)?;
        let input = request.payload;
        WorkerFailure::require_positive_id(input.project_id, "project_id")?;
        if let Some(chapter_id) = input.chapter_id {
            WorkerFailure::require_positive_id(chapter_id, "chapter_id")?;
        }
        verify_project_permission(
            &self.database,
            input.project_id,
            user_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let images = a4n_db::video::read_models::validated_video_reference_image_records(
            self.database.connection(),
            input.project_id,
            input.chapter_id,
            input.reference_image_ids_json.as_ref(),
        )
        .await
        .map_err(video_error)?;
        let project_ids = images
            .iter()
            .map(|image| image.project_id)
            .collect::<HashSet<_>>();
        for project_id in project_ids {
            verify_project_permission(
                &self.database,
                project_id,
                user_id,
                ProjectAccessRole::Editor,
            )
            .await?;
        }
        Ok(images.iter().map(image_summary).collect())
    }
}
