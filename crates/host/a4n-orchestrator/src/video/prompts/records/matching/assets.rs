use serde_json::Value;

use a4n_domain::source_text_hash as source_content_sha256;
#[cfg(test)]
use a4n_domain::video_prompt::ChapterVideoPromptPatch;

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
pub(in crate::video::prompts::records) fn db_video_prompt_update_input(
    input: VideoPromptUpdateInput<'_>,
) -> ChapterVideoPromptPatch {
    ChapterVideoPromptPatch {
        visual_prompt: input.visual_prompt.map(ToString::to_string),
        motion_prompt: input
            .motion_prompt
            .map(|value| value.map(ToString::to_string)),
        negative_prompt: input
            .negative_prompt
            .map(|value| value.map(ToString::to_string)),
        duration_secs: input.duration_secs,
        reference_image_ids_json: input.reference_image_ids_json,
    }
}
