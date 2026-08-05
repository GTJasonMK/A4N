use a4n_image_providers::pick_image_provider;
use a4n_service_protocol::{ServiceRequest, ServiceResponse};
use axum::Json;
use base64::Engine as _;

use crate::contracts::{ImageGenerationOutput, ImageProviderPayload};
use crate::protocol::{error_response, result_response, validate_image_request};

pub(crate) async fn generate(
    Json(request): Json<ServiceRequest<ImageProviderPayload>>,
) -> Json<ServiceResponse<ImageGenerationOutput>> {
    if let Err(error) = validate_image_request(&request) {
        return Json(error_response(&request, error));
    }
    let result = async {
        let provider = pick_image_provider(
            &request.payload.backend,
            &request.payload.base_url,
            &request.payload.api_key,
        )?;
        let result = provider.generate(request.payload.request.clone()).await?;
        Ok(ImageGenerationOutput {
            bytes_base64: base64::engine::general_purpose::STANDARD.encode(result.bytes),
            format: result.format,
            model: result.model,
        })
    }
    .await;
    Json(result_response(&request, result))
}
