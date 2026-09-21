use serde_json::{Map, Value, json};

use a4n_domain::ProviderError as AppError;
use a4n_video_provider_contract::{SEEDANCE2_VIDEO_BACKEND, validate_video_provider_duration};
use a4n_video_sidecar_contract::{VideoGenerationRequest, VideoMediaInput};

use super::Seedance2VideoProvider;

impl Seedance2VideoProvider {
    pub(crate) fn request_body(req: &VideoGenerationRequest) -> Result<Value, AppError> {
        req.validate_dimensions()
            .map_err(AppError::InvalidArgument)?;
        let VideoMediaInput::References {
            images,
            videos,
            audio,
        } = &req.media
        else {
            return Err(AppError::InvalidArgument(
                "Seedance 2.0 参考媒体模式要求 media.type=references".into(),
            ));
        };
        if images.is_empty() && videos.is_empty() && audio.is_empty() {
            return Err(AppError::InvalidArgument(
                "Seedance 2.0 参考媒体模式至少需要一个 image_files、video_files 或 audio_files"
                    .into(),
            ));
        }

        let mut parameters = Map::new();
        parameters.insert("prompt".into(), json!(req.prompt));
        parameters.insert("functionMode".into(), json!("omni_reference"));
        parameters.insert("image_files".into(), json!(images));
        parameters.insert("video_files".into(), json!(videos));
        parameters.insert("audio_files".into(), json!(audio));
        parameters.insert(
            "ratio".into(),
            json!(ratio_from_size(req.width, req.height)),
        );
        validate_video_provider_duration(SEEDANCE2_VIDEO_BACKEND, req.duration_secs)
            .map_err(AppError::InvalidArgument)?;
        parameters.insert("duration".into(), json!(req.duration_secs));

        Ok(json!({
            "model_id": req.model,
            "parameters": Value::Object(parameters),
        }))
    }
}
fn ratio_from_size(width: i32, height: i32) -> &'static str {
    if width * 9 == height * 21 {
        return "21:9";
    }
    if width * 9 == height * 16 {
        return "16:9";
    }
    if width * 3 == height * 4 {
        return "4:3";
    }
    if width == height {
        return "1:1";
    }
    if width * 4 == height * 3 {
        return "3:4";
    }
    if width * 16 == height * 9 {
        return "9:16";
    }
    "16:9"
}
