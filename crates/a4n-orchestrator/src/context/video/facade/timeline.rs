//! 视频时间线 facade。

use super::*;

impl OrchestratorContext {
    pub(crate) async fn chapter_video_timeline_by_scope(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
    ) -> Result<Option<chapter_video_timeline::Model>, AppError> {
        self.video
            .chapter_video_timeline_by_scope(project_id, chapter_id, workflow_mode)
            .await
    }

    pub(crate) async fn chapter_video_timeline_snapshot(
        &self,
        timeline_id: i64,
    ) -> Result<a4n_db::video::VideoTimelineSnapshot, AppError> {
        self.video
            .chapter_video_timeline_snapshot(timeline_id)
            .await
    }

    pub(crate) async fn ensure_chapter_video_timeline(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
    ) -> Result<chapter_video_timeline::Model, AppError> {
        self.video
            .ensure_chapter_video_timeline(project_id, chapter_id, workflow_mode)
            .await
    }

    pub(crate) async fn reconcile_native_video_timeline(
        &self,
        timeline_id: i64,
        keys: &[NativeVideoTimelineKey],
    ) -> Result<a4n_db::video::VideoTimelineSnapshot, AppError> {
        self.video
            .reconcile_native_video_timeline(timeline_id, keys)
            .await
    }

    pub(crate) async fn commit_inserted_video_timeline_item(
        &self,
        input: a4n_db::video::CommitInsertedTimelineItemInput,
    ) -> Result<(), AppError> {
        self.video.commit_inserted_video_timeline_item(input).await
    }

    pub(crate) async fn move_inserted_video_timeline_item(
        &self,
        timeline_id: i64,
        expected_revision: i64,
        item_id: i64,
        gap: VideoTimelineGap,
    ) -> Result<(), AppError> {
        self.video
            .move_inserted_video_timeline_item(timeline_id, expected_revision, item_id, gap)
            .await
    }

    pub(crate) async fn delete_inserted_video_timeline_item(
        &self,
        timeline_id: i64,
        expected_revision: i64,
        item_id: i64,
    ) -> Result<(), AppError> {
        self.video
            .delete_inserted_video_timeline_item(timeline_id, expected_revision, item_id)
            .await
    }
}
