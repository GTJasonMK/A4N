use super::errors::internal_error;
use super::format::{decode_base64_image, image_format_hint, optional_format_from_response_item};
use super::http::download_image_url;
use a4n_domain::ProviderError as AppError;
use a4n_image::ImageGenerationResult;
pub(crate) async fn image_result_from_openai_payload(
    client: &reqwest::Client,
    text: &str,
    model: String,
    label: &str,
) -> Result<ImageGenerationResult, AppError> {
    let v: serde_json::Value = serde_json::from_str(text)
        .map_err(|e| internal_error(format!("{label}响应解析失败：{e}")))?;
    let item = v
        .get("data")
        .and_then(|v| v.as_array())
        .and_then(|items| items.first())
        .ok_or_else(|| internal_error(format!("{label}响应缺少 data[0]")))?;
    let declared_format = optional_format_from_response_item(item)?;

    if let Some(b64) = item
        .get("b64_json")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        let bytes = decode_base64_image(b64)?;
        let format = image_format_hint(declared_format.as_deref(), None, None, &bytes)?;
        return Ok(ImageGenerationResult {
            format,
            bytes,
            model,
        });
    }

    if let Some(img_url) = item
        .get("url")
        .and_then(|v| v.as_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        tracing::debug!("provider 返回 URL，下载：{img_url}");
        let (bytes, format) =
            download_image_url(client, img_url, declared_format.as_deref()).await?;
        return Ok(ImageGenerationResult {
            bytes,
            format,
            model,
        });
    }

    Err(internal_error(format!(
        "{label}响应缺少 data[0].b64_json 或 data[0].url"
    )))
}
