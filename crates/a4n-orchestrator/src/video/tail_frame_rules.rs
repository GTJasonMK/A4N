//! 视频尾帧归属与元数据纯规则（无 DB、无 Host）。

use a4n_db_models::models::ai::{generated_image, generated_video};

/// 尾帧轻量引用是否属于指定视频，供只读清理列表避免加载完整图片实体。
pub fn tail_frame_reference_belongs_to_video(
    image: &generated_image::Reference,
    video_id: i64,
) -> bool {
    a4n_video::tail_frame_metadata_belongs_to_video(
        &image.image_type,
        image.metadata_json.as_ref(),
        video_id,
    )
}

/// 从视频元数据读取镜头序号。
pub fn video_shot_number(video: &generated_video::Model) -> Result<i32, String> {
    a4n_video::video_shot_number(video.metadata_json.as_ref(), video.id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_domain::image::{
        IMAGE_GENERATION_SOURCE_VIDEO_TAIL_FRAME_CAPTURE, IMAGE_TYPE_VIDEO_TAIL_FRAME,
    };
    use serde_json::{Value, json};

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
