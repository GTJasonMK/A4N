pub(crate) use a4n_image::{ImageGenerationOutput, ImageProviderPayload};

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
