//! 视频时间线 owner。

use super::*;

impl VideoApplication {
    pub(in crate::context) async fn chapter_video_timeline_by_scope(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
    ) -> Result<Option<VideoTimelineRecord>, AppError> {
        a4n_db::video::chapter_video_timeline_by_scope(
            self.connection(),
            project_id,
            chapter_id,
            workflow_mode,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::context) async fn chapter_video_timeline_snapshot(
        &self,
        timeline_id: i64,
    ) -> Result<a4n_db::video::VideoTimelineSnapshot, AppError> {
        a4n_db::video::chapter_video_timeline_snapshot(self.connection(), timeline_id)
            .await
            .map_err(video_db_error)
    }

    pub(in crate::context) async fn ensure_chapter_video_timeline(
        &self,
        project_id: i64,
        chapter_id: i64,
        workflow_mode: &str,
    ) -> Result<VideoTimelineRecord, AppError> {
        a4n_db::video::ensure_chapter_video_timeline(
            self.connection(),
            project_id,
            chapter_id,
            workflow_mode,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::context) async fn reconcile_native_video_timeline(
        &self,
        timeline_id: i64,
        keys: &[NativeVideoTimelineKey],
    ) -> Result<a4n_db::video::VideoTimelineSnapshot, AppError> {
        a4n_db::video::reconcile_native_timeline(self.connection(), timeline_id, keys)
            .await
            .map_err(video_db_error)
    }

    pub(in crate::context) async fn commit_inserted_video_timeline_item(
        &self,
        input: a4n_db::video::CommitInsertedTimelineItemInput,
    ) -> Result<(), AppError> {
        a4n_db::video::commit_inserted_timeline_item(self.connection(), input)
            .await
            .map_err(video_db_error)
    }

    pub(in crate::context) async fn move_inserted_video_timeline_item(
        &self,
        timeline_id: i64,
        expected_revision: i64,
        item_id: i64,
        gap: VideoTimelineGap,
    ) -> Result<(), AppError> {
        a4n_db::video::move_inserted_timeline_item_persisted(
            self.connection(),
            timeline_id,
            expected_revision,
            item_id,
            gap,
        )
        .await
        .map_err(video_db_error)
    }

    pub(in crate::context) async fn delete_inserted_video_timeline_item(
        &self,
        timeline_id: i64,
        expected_revision: i64,
        item_id: i64,
    ) -> Result<(), AppError> {
        a4n_db::video::delete_inserted_timeline_item(
            self.connection(),
            timeline_id,
            expected_revision,
            item_id,
        )
        .await
        .map_err(video_db_error)
    }
}
