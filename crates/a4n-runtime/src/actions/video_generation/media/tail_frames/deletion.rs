use super::rules::tail_frame_image_belongs_to_video;
use crate::AppState;
use crate::actions::video_generation::media::video_files::{
    combine_optional_errors, remove_partial_video_file,
};
use crate::file_utils::deletable_data_file_path;

pub(crate) async fn delete_video_tail_frame_assets(
    state: &AppState,
    video_id: i64,
) -> Result<(), String> {
    let images = state
        .orchestrator
        .video_tail_frame_images()
        .await
        .map_err(|error| error.to_string())
        .map_err(|err| format!("查询视频真实尾帧图片失败：{err}"))?
        .into_iter()
        .filter(|image| tail_frame_image_belongs_to_video(image, video_id))
        .collect::<Vec<_>>();

    let mut error = None;
    for image in images {
        let path = image.file_path.trim();
        let image_path = if path.is_empty() {
            None
        } else {
            deletable_data_file_path(
                &state.config.data_dir,
                path,
                &format!("视频 #{} 真实尾帧图片 #{}", video_id, image.id),
            )?
        };
        match state
            .orchestrator
            .delete_generated_image_record(image.id)
            .await
            .map_err(|error| error.to_string())
        {
            Ok(()) => {
                if let Some(path) = image_path.as_ref() {
                    error = combine_optional_errors(
                        error,
                        remove_partial_video_file(
                            path,
                            &format!("视频 #{} 真实尾帧图片 #{}", video_id, image.id),
                        )
                        .err(),
                    );
                }
            }
            Err(err) => {
                error = combine_optional_errors(
                    error,
                    Some(format!(
                        "视频 #{} 真实尾帧图片 #{} 记录删除失败：{err}",
                        video_id, image.id
                    )),
                );
            }
        }
    }
    match error {
        Some(error) => Err(error),
        None => Ok(()),
    }
}
