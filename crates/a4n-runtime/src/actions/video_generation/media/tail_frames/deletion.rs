use crate::AppState;
use crate::actions::video_generation::media::video_files::combine_optional_errors;
use a4n_orchestrator::video::tail_frame_image_belongs_to_video;

pub(crate) async fn delete_video_tail_frame_assets(
    state: &AppState,
    video_id: i64,
) -> Result<(), String> {
    let images = state
        .orchestrator()
        .video_tail_frame_images()
        .await
        .map_err(|error| format!("查询视频真实尾帧图片失败：{error}"))?
        .into_iter()
        .filter(|image| tail_frame_image_belongs_to_video(image, video_id))
        .collect::<Vec<_>>();

    let mut error = None;
    for image in images {
        let image_id = image.id;
        match state
            .orchestrator()
            .delete_generated_image_validated(image)
            .await
            .map_err(|error| error.to_string())
        {
            Ok(()) => {}
            Err(err) => {
                error = combine_optional_errors(
                    error,
                    Some(format!(
                        "视频 #{} 真实尾帧图片 #{} 记录删除失败：{err}",
                        video_id, image_id
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
