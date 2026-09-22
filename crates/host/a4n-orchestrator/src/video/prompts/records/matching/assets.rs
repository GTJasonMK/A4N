use serde_json::Value;

use a4n_domain::source_text_hash;

pub(in crate::video) fn video_generation_asset_fingerprint(
    reference_image_ids: &[i64],
    video_library_image_ids: &[i64],
    available_assets: &Value,
) -> Value {
    let available_assets_json = available_assets.to_string();
    serde_json::json!({
        "schema_version": 1,
        "reference_image_ids": reference_image_ids,
        "video_library_image_ids": video_library_image_ids,
        "available_assets_sha256": source_text_hash(&available_assets_json),
    })
}
