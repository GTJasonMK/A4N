use a4n_domain::ProviderError as AppError;

pub(crate) fn with_provider_idempotency_key(
    request: reqwest::RequestBuilder,
    value: Option<&str>,
) -> Result<reqwest::RequestBuilder, AppError> {
    a4n_video_provider_contract::validate_video_provider_idempotency_key(value)?;
    Ok(match value {
        Some(value) => request.header("Idempotency-Key", value),
        None => request,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adds_validated_idempotency_header() {
        let value = "a".repeat(64);
        let request = with_provider_idempotency_key(
            reqwest::Client::new().post("https://example.test/video"),
            Some(&value),
        )
        .unwrap()
        .build()
        .unwrap();
        assert_eq!(request.headers()["Idempotency-Key"], value);
    }
}
