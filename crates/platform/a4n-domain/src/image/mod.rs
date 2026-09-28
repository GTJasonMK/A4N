mod config;
mod generation;
mod references;
mod sizes;

pub use config::{
    CreateImageConfigParams, IMAGE_PROVIDER_CAPABILITIES, ImageProviderCapabilities,
    image_backend_api_key, image_backend_requires_api_key, image_backend_supports_reference_images,
    image_provider_capabilities, validate_image_config_params,
};
pub use generation::{
    IMAGE_GENERATION_SOURCE_VIDEO_BOUNDARY_FRAME_PROMPT,
    IMAGE_GENERATION_SOURCE_VIDEO_FIRST_FRAME_PROMPT,
    IMAGE_GENERATION_SOURCE_VIDEO_STORYBOARD_PROMPT,
    IMAGE_GENERATION_SOURCE_VIDEO_TAIL_FRAME_CAPTURE, IMAGE_TYPE_CANVAS, IMAGE_TYPE_CANVAS_FRAME,
    IMAGE_TYPE_DIRECTOR_STORYBOARD_FRAME, IMAGE_TYPE_OBJECT_MATERIAL, IMAGE_TYPE_SCENE_MATERIAL,
    IMAGE_TYPE_VIDEO_KEY_FRAME, IMAGE_TYPE_VIDEO_START_FRAME, IMAGE_TYPE_VIDEO_STORYBOARD_FRAME,
    IMAGE_TYPE_VIDEO_TAIL_FRAME, VideoFrameImageGenerationContext, image_generation_prompt_id,
    image_generation_shot_number, image_generation_source, image_generation_source_hash,
    is_system_managed_image_type, is_video_frame_prompt_image_type,
    video_frame_image_generation_context,
};
pub use references::{
    batch_item_reference_image_ids, reference_image_ids, validate_reference_image_id_list,
    validate_reference_image_ids,
};
pub use sizes::{
    DEFAULT_IMAGE_SIZES, ImageSize, default_image_sizes, ensure_image_size_supported,
    image_sizes_from_json, image_sizes_to_json, validate_image_sizes,
};

pub const OPENAI_COMPATIBLE_IMAGE_BACKEND: &str = "openai-compatible-image";
pub const OPENAI_COMPATIBLE_IMAGE_EDIT_BACKEND: &str = "openai-compatible-image-edit";
pub const OPENAI_CHAT_IMAGE_BACKEND: &str = "openai-chat-image";
pub const CLIPROXY_IMAGE_BACKEND: &str = "cliproxy-image";
pub const STABLE_DIFFUSION_IMAGE_BACKEND: &str = "stable-diffusion";
pub const COMFYUI_IMAGE_BACKEND: &str = "comfyui";
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn system_managed_image_types_have_one_canonical_catalog() {
        assert!(is_system_managed_image_type(" project_reference "));
        assert!(is_system_managed_image_type(IMAGE_TYPE_VIDEO_TAIL_FRAME));
        assert!(!is_system_managed_image_type("custom_reference"));
    }

    #[test]
    fn reference_image_ids_are_positive_unique_and_ordered() {
        assert_eq!(validate_reference_image_ids(&[3, 5]).unwrap(), vec![3, 5]);
        assert_eq!(
            validate_reference_image_ids(&[]).unwrap(),
            Vec::<i64>::new()
        );

        let err = validate_reference_image_ids(&[0]).unwrap_err();
        assert!(err.contains("必须为正整数"));
        let err = validate_reference_image_ids(&[2, 2]).unwrap_err();
        assert!(err.contains("重复 ID：2"));
    }

    #[test]
    fn parses_reference_image_id_json_once_for_all_consumers() {
        assert_eq!(reference_image_ids(None), Ok(Vec::new()));
        assert_eq!(reference_image_ids(Some(&json!([3, 5]))), Ok(vec![3, 5]));
        assert_eq!(
            batch_item_reference_image_ids(&json!({"reference_image_ids": [1, 2]})),
            Ok(vec![1, 2])
        );
        assert!(reference_image_ids(Some(&json!([3, 3]))).is_err());
        assert!(validate_reference_image_id_list(&[0]).is_err());
    }

    #[test]
    fn image_generation_metadata_has_one_strict_reader() {
        let metadata = json!({
            "generation_context": {
                "prompt_id": 12,
                "source": IMAGE_GENERATION_SOURCE_VIDEO_STORYBOARD_PROMPT,
                "shot_number": 3,
                "source_content_sha256": "abc"
            }
        });
        assert_eq!(image_generation_prompt_id(Some(&metadata)), Some(12));
        assert_eq!(
            image_generation_source(Some(&metadata)),
            Some(IMAGE_GENERATION_SOURCE_VIDEO_STORYBOARD_PROMPT)
        );
        assert_eq!(image_generation_shot_number(Some(&metadata)), Some(3));
        assert_eq!(image_generation_source_hash(Some(&metadata)), Some("abc"));

        let invalid = json!({
            "generation_context": {
                "prompt_id": 0,
                "source": " video_storyboard_prompt ",
                "shot_number": -1,
                "source_content_sha256": " abc "
            }
        });
        assert_eq!(image_generation_prompt_id(Some(&invalid)), None);
        assert_eq!(image_generation_source(Some(&invalid)), None);
        assert_eq!(image_generation_shot_number(Some(&invalid)), None);
        assert_eq!(image_generation_source_hash(Some(&invalid)), None);
    }

    #[test]
    fn video_frame_generation_context_rejects_unknown_and_invalid_sources() {
        let metadata = json!({
            "generation_context": {
                "source": IMAGE_GENERATION_SOURCE_VIDEO_STORYBOARD_PROMPT,
                "prompt_id": 44
            }
        });
        let context = video_frame_image_generation_context(Some(&metadata))
            .expect("context should parse")
            .expect("video frame context should be present");
        assert_eq!(
            context.source,
            IMAGE_GENERATION_SOURCE_VIDEO_STORYBOARD_PROMPT
        );
        assert_eq!(context.generation_label, "分镜成图");
        assert_eq!(context.prompt_id, 44);

        let invalid = json!({
            "generation_context": {
                "source": IMAGE_GENERATION_SOURCE_VIDEO_FIRST_FRAME_PROMPT,
                "prompt_id": "43"
            }
        });
        assert!(
            video_frame_image_generation_context(Some(&invalid))
                .expect_err("prompt_id string must fail")
                .contains("generation_context.prompt_id")
        );
        assert_eq!(
            video_frame_image_generation_context(Some(&json!({
                "generation_context": {
                    "source": IMAGE_GENERATION_SOURCE_VIDEO_TAIL_FRAME_CAPTURE
                }
            })))
            .expect("tail capture is not a prompt image source"),
            None
        );
    }
}
