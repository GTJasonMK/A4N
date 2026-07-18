use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_video_prompt;
use serde::Deserialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

#[derive(Deserialize)]
pub(super) struct SavedVideoPromptMetadata {
    #[serde(rename = "video_unit")]
    pub(super) _video_unit: Value,
    pub(super) self_contained_context: Value,
    pub(super) start_state: Value,
    pub(super) end_state: Value,
    pub(super) transition_in: Value,
    pub(super) blocking_plan: Value,
    pub(super) lighting_plan: Value,
    pub(super) emotion_action_beats: Value,
    pub(super) dialogue_beats: Value,
    pub(super) cinematography_plan: Value,
    pub(super) temporal_beats: Value,
    pub(super) reference_binding: Value,
    pub(super) continuity_check: Value,
}

#[derive(Deserialize)]
pub(super) struct SavedChatVideoPromptMetadata {
    #[serde(rename = "video_unit")]
    pub(super) _video_unit: Value,
    pub(super) chat_video: Value,
}

pub(super) fn required_row_aspect_ratio(
    row: &chapter_video_prompt::Model,
    label: &str,
) -> Result<String, AppError> {
    row.aspect_ratio.clone().ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "已保存{label} #{} 缺少 aspect_ratio",
            row.shot_number
        ))
    })
}

pub(super) fn row_reference_image_ids(
    row: &chapter_video_prompt::Model,
    label: &str,
) -> Result<Vec<i64>, AppError> {
    match row.reference_image_ids_json.as_ref() {
        Some(value) => serde_json::from_value::<Vec<i64>>(value.clone()).map_err(|error| {
            AppError::InvalidArgument(format!(
                "已保存{label} #{} 的 reference_image_ids_json 无效：{}",
                row.shot_number, error
            ))
        }),
        None => Ok(Vec::new()),
    }
}

pub(super) fn required_row_text(
    value: &Option<String>,
    row: &chapter_video_prompt::Model,
    label: &str,
    field: &str,
) -> Result<String, AppError> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .map(str::to_string)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("已保存{label} #{} 缺少 {field}", row.shot_number))
        })
}

pub(super) fn row_metadata<T>(row: &chapter_video_prompt::Model, label: &str) -> Result<T, AppError>
where
    T: DeserializeOwned,
{
    let value = row.metadata_json.clone().ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "已保存{label} #{} 缺少 metadata_json",
            row.shot_number
        ))
    })?;
    serde_json::from_value(value).map_err(|error| {
        AppError::InvalidArgument(format!(
            "已保存{label} #{} 的 metadata_json 无效：{}",
            row.shot_number, error
        ))
    })
}
