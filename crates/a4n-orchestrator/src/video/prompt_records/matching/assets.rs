use serde_json::Value;

use a4n_core::source_spans::source_text_hash as source_content_sha256;

#[cfg(test)]
use super::super::VideoPromptUpdateInput;

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
        "available_assets_sha256": source_content_sha256(&available_assets_json),
    })
}

#[cfg(test)]
pub(in crate::video::prompt_records) fn db_video_prompt_update_input(
    input: VideoPromptUpdateInput<'_>,
) -> a4n_db::video_prompt::UpdateChapterVideoPromptInput {
    a4n_db::video_prompt::UpdateChapterVideoPromptInput {
        visual_prompt: input.visual_prompt.map(ToString::to_string),
        motion_prompt: input
            .motion_prompt
            .map(|value| value.map(ToString::to_string)),
        negative_prompt: input
            .negative_prompt
            .map(|value| value.map(ToString::to_string)),
        duration_secs: input.duration_secs,
        reference_image_ids_json: input.reference_image_ids_json,
        metadata_patch: None,
    }
}
