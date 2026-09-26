use serde_json::{Map, Value, json};

use a4n_domain::ProviderError as AppError;
use a4n_video_provider_contract::{
    SEEDANCE2_VIDEO_BACKEND, validate_video_provider_duration, video_provider_prompt,
};
use a4n_video_sidecar_contract::{VideoGenerationRequest, VideoMediaInput};

use super::Seedance2VideoProvider;

impl Seedance2VideoProvider {
    pub(crate) fn request_body(req: &VideoGenerationRequest) -> Result<Value, AppError> {
        req.validate_dimensions()
            .map_err(AppError::InvalidArgument)?;
        let mut parameters = Map::new();
        parameters.insert("model".into(), json!(req.model));
        parameters.insert(
            "prompt".into(),
            json!(
                video_provider_prompt(
                    SEEDANCE2_VIDEO_BACKEND,
                    &req.prompt,
                    req.negative_prompt.as_deref()
                )
                .map_err(AppError::InvalidArgument)?
            ),
        );
        validate_video_provider_duration(SEEDANCE2_VIDEO_BACKEND, req.duration_secs)
            .map_err(AppError::InvalidArgument)?;
        parameters.insert("seconds".into(), json!(req.duration_secs.to_string()));
        parameters.insert(
            "size".into(),
            json!(format!("{}x{}", req.width, req.height)),
        );

        match &req.media {
            VideoMediaInput::None => {}
            VideoMediaInput::Frames {
                first,
                last,
                references,
            } => {
                parameters.insert("first_frame_image".into(), json!(first));
                if let Some(last) = last {
                    parameters.insert("last_frame_image".into(), json!(last));
                }
                insert_input_references(&mut parameters, references);
            }
            VideoMediaInput::References {
                images,
                videos,
                audio,
            } => {
                let mut references = Vec::with_capacity(images.len() + videos.len() + audio.len());
                references.extend(images.iter().cloned());
                references.extend(videos.iter().cloned());
                references.extend(audio.iter().cloned());
                if references.is_empty() {
                    return Err(AppError::InvalidArgument(
                        "Seedance 2.0 参考媒体模式至少需要一个 input_reference".into(),
                    ));
                }
                insert_input_references(&mut parameters, &references);
            }
        }

        Ok(Value::Object(parameters))
    }
}

fn insert_input_references(parameters: &mut Map<String, Value>, references: &[String]) {
    if !references.is_empty() {
        parameters.insert("input_reference".into(), json!(references));
    }
}
