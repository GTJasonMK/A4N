//! 视频尾帧归属与元数据纯规则（无 DB、无 Host）。

use a4n_db_models::models::ai::{generated_image, generated_video};
use a4n_domain::image::{
    IMAGE_GENERATION_SOURCE_VIDEO_TAIL_FRAME_CAPTURE, IMAGE_TYPE_VIDEO_TAIL_FRAME,
    image_generation_source,
};
use serde_json::Value;

/// 尾帧图是否属于指定视频的采集产物。
pub fn tail_frame_image_belongs_to_video(image: &generated_image::Model, video_id: i64) -> bool {
    tail_frame_metadata_belongs_to_video(&image.image_type, image.metadata_json.as_ref(), video_id)
}

/// 尾帧轻量引用是否属于指定视频，供只读清理列表避免加载完整图片实体。
pub fn tail_frame_reference_belongs_to_video(
    image: &generated_image::Reference,
    video_id: i64,
) -> bool {
    tail_frame_metadata_belongs_to_video(&image.image_type, image.metadata_json.as_ref(), video_id)
}

pub fn tail_frame_metadata_belongs_to_video(
    image_type: &str,
    metadata_json: Option<&Value>,
    video_id: i64,
) -> bool {
    if image_type != IMAGE_TYPE_VIDEO_TAIL_FRAME
        || image_generation_source(metadata_json)
            != Some(IMAGE_GENERATION_SOURCE_VIDEO_TAIL_FRAME_CAPTURE)
    {
        return false;
    }
    metadata_json
        .and_then(|metadata| metadata.get("generation_context"))
        .and_then(|context| context.get("video_id"))
        .and_then(Value::as_i64)
        == Some(video_id)
}

/// 从视频元数据读取镜头序号。
pub fn video_shot_number(video: &generated_video::Model) -> Result<i32, String> {
    video
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("shot_number"))
        .and_then(Value::as_i64)
        .and_then(|value| i32::try_from(value).ok())
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("视频 #{} 缺少有效 shot_number，无法采集真实尾帧", video.id))
}

pub fn is_first_frame_chain_video(video: &generated_video::Model) -> bool {
    a4n_video::is_first_frame_chain_workflow(video.workflow_mode.as_deref())
}

pub fn should_capture_video_tail_frame(video: &generated_video::Model) -> bool {
    a4n_video::should_capture_video_tail_frame_fields(
        &video.video_type,
        video.workflow_mode.as_deref(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_domain::image::IMAGE_TYPE_VIDEO_TAIL_FRAME;
    use serde_json::json;

    fn image_with_context(context: Value) -> generated_image::Reference {
        generated_image::Reference {
            id: 1,
            image_type: IMAGE_TYPE_VIDEO_TAIL_FRAME.into(),
            metadata_json: Some(json!({ "generation_context": context })),
        }
    }

    #[test]
    fn belongs_to_video_requires_capture_source_and_id() {
        let ok = image_with_context(json!({
            "source": IMAGE_GENERATION_SOURCE_VIDEO_TAIL_FRAME_CAPTURE,
            "video_id": 42
        }));
        assert!(tail_frame_reference_belongs_to_video(&ok, 42));
        assert!(!tail_frame_reference_belongs_to_video(&ok, 41));

        let wrong_source = image_with_context(json!({
            "source": "other",
            "video_id": 42
        }));
        assert!(!tail_frame_reference_belongs_to_video(&wrong_source, 42));
    }
}
