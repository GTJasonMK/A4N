use crate::AppState;
use crate::actions::app_error_message;
use crate::actions::video_generation::media::append_secondary_error;
use a4n_core::models::media::media_asset;
use a4n_orchestrator::media::MEDIA_ORIGIN_IMPORTED;
use std::future::Future;

pub(super) async fn sync_imported_canvas_media_asset_or_cleanup<T, S, C>(
    sync_asset: S,
    cleanup_import: C,
    cleanup_label: &str,
) -> Result<T, String>
where
    S: Future<Output = Result<T, String>>,
    C: Future<Output = Result<(), String>>,
{
    match sync_asset.await {
        Ok(asset) => Ok(asset),
        Err(error) => {
            let cleanup_error = cleanup_import
                .await
                .err()
                .map(|cleanup_error| format!("{cleanup_label}：{cleanup_error}"));
            Err(append_secondary_error(error, cleanup_error))
        }
    }
}

pub(super) async fn sync_imported_canvas_image_asset(
    state: &AppState,
    project_id: i64,
    image_id: i64,
) -> Result<media_asset::Model, String> {
    let image = state
        .orchestrator
        .generated_image_by_id(image_id)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("画布导入图片不存在: {image_id}"))?;
    super::super::ensure_canvas_image_media_asset(
        state,
        project_id,
        &image,
        None,
        MEDIA_ORIGIN_IMPORTED,
    )
    .await
}

pub(super) async fn sync_imported_canvas_video_asset(
    state: &AppState,
    project_id: i64,
    video_id: i64,
) -> Result<media_asset::Model, String> {
    let video = state
        .orchestrator
        .generated_video_by_id(video_id)
        .await
        .map_err(app_error_message)?
        .ok_or_else(|| format!("画布导入视频不存在: {video_id}"))?;
    super::super::ensure_canvas_video_media_asset(
        state,
        project_id,
        &video,
        None,
        MEDIA_ORIGIN_IMPORTED,
    )
    .await
}

#[cfg(test)]
mod tests {
    use super::sync_imported_canvas_media_asset_or_cleanup;
    use std::sync::atomic::{AtomicBool, Ordering};

    #[tokio::test]
    async fn canvas_media_asset_sync_success_does_not_cleanup_import() {
        let cleanup_called = AtomicBool::new(false);

        let value = sync_imported_canvas_media_asset_or_cleanup(
            async { Ok::<_, String>(42) },
            async {
                cleanup_called.store(true, Ordering::SeqCst);
                Ok(())
            },
            "画布导入补偿删除失败",
        )
        .await
        .expect("successful media asset sync");

        assert_eq!(value, 42);
        assert!(!cleanup_called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn canvas_media_asset_sync_failure_runs_cleanup_and_preserves_primary_error() {
        let cleanup_called = AtomicBool::new(false);

        let error = sync_imported_canvas_media_asset_or_cleanup(
            async { Err::<(), _>("media asset sync failed".to_string()) },
            async {
                cleanup_called.store(true, Ordering::SeqCst);
                Ok(())
            },
            "画布导入补偿删除失败",
        )
        .await
        .expect_err("failed media asset sync must cleanup import");

        assert_eq!(error, "media asset sync failed");
        assert!(cleanup_called.load(Ordering::SeqCst));
    }

    #[tokio::test]
    async fn canvas_media_asset_cleanup_failure_is_appended_to_sync_error() {
        let error = sync_imported_canvas_media_asset_or_cleanup(
            async { Err::<(), _>("media asset sync failed".to_string()) },
            async { Err("generated record delete failed".to_string()) },
            "画布导入视频补偿删除失败",
        )
        .await
        .expect_err("cleanup failure must remain explicit");

        assert_eq!(
            error,
            "media asset sync failed；画布导入视频补偿删除失败：generated record delete failed"
        );
    }
}
