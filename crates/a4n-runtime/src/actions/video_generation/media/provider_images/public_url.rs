use crate::actions::video_generation::media::types::MEDIA_TOKEN_TTL_ENV;
use crate::jwt;
use a4n_core::config::env::{required_non_empty_env_string, required_public_app_endpoint};
use a4n_core::models::ai::generated_image;
use a4n_orchestrator::video::encode_query_component;

pub(in crate::actions::video_generation::media) fn image_file_to_provider_media_url(
    jwt_secret: &str,
    user_id: i64,
    base: &str,
    image: &generated_image::Model,
) -> Result<String, String> {
    let media_path = image.file_path.trim();
    let token = jwt::create_media_token(
        &user_id.to_string(),
        media_path,
        jwt_secret,
        media_token_ttl_minutes()?,
    )
    .map_err(|err| err.to_string())?;
    Ok(format!(
        "{base}?path={}&media_token={}",
        encode_query_component(media_path),
        encode_query_component(&token)
    ))
}

fn media_token_ttl_minutes() -> Result<u64, String> {
    let value = required_non_empty_env_string(MEDIA_TOKEN_TTL_ENV)
        .map_err(|error| error.to_string())?;
    value
        .parse::<u64>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("{MEDIA_TOKEN_TTL_ENV} 必须是正整数"))
}

pub(in crate::actions::video_generation::media) fn public_media_base_url() -> Result<String, String>
{
    required_public_app_endpoint("/api/media").map_err(|error| error.to_string())
}
