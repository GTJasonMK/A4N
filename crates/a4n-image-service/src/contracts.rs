use a4n_image::ImageGenerationRequest;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct ImageProviderPayload {
    pub(crate) backend: String,
    pub(crate) base_url: String,
    #[serde(default)]
    pub(crate) api_key: String,
    pub(crate) request: ImageGenerationRequest,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ImageGenerationOutput {
    pub(crate) bytes_base64: String,
    pub(crate) format: String,
    pub(crate) model: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_payload_rejects_unknown_fields() {
        let error = serde_json::from_value::<ImageProviderPayload>(
            serde_json::json!({"legacy_backend": "openai"}),
        )
        .expect_err("unknown image payload fields must fail");
        assert!(error.to_string().contains("unknown field `legacy_backend`"));
    }
}
