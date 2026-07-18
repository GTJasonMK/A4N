use super::{MEDIA_FILE_FIELD_FILE, MEDIA_FILE_FIELD_THUMBNAIL};
use a4n_core::models::media::media_asset;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::path::Path;

pub(super) fn sha256_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

pub(super) fn hash_json(value: &Value) -> String {
    let bytes = serde_json::to_vec(value).expect("serde_json::Value serialization cannot fail");
    sha256_bytes(&bytes)
}

pub(super) fn hash_text(text: &str) -> String {
    sha256_bytes(text.as_bytes())
}

pub(super) fn media_asset_path<'a>(
    asset: &'a media_asset::Model,
    field: &str,
) -> Result<Option<&'a str>, String> {
    match field {
        MEDIA_FILE_FIELD_FILE => Ok(asset.file_path.as_deref()),
        MEDIA_FILE_FIELD_THUMBNAIL => Ok(asset.thumbnail_path.as_deref()),
        _ => Err(format!("媒体文件字段无效: {field}")),
    }
}

pub(super) fn media_file_key(media_asset_id: i64, field: &str) -> (i64, String) {
    (media_asset_id, field.to_string())
}

pub(super) fn required_package_extension(path: &str) -> Result<String, String> {
    Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
        .ok_or_else(|| format!("媒体包路径缺少扩展名: {path}"))
}
