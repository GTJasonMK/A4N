use a4n_core::error::AppError;
use serde_json::{Value, json};

use super::super::super::shooting_script_projection::manga_events_from_shooting_script;
use super::super::contracts::MangaPreviewContext;

pub(super) fn preview_events(
    context: &MangaPreviewContext<'_>,
) -> Result<(String, Value), AppError> {
    let events_json = manga_events_from_shooting_script(&context.shooting_script.script);
    if events_json.as_array().is_none_or(Vec::is_empty) {
        return Err(AppError::InvalidArgument(
            "结构化拍摄剧本没有可投影的漫画事件".to_string(),
        ));
    }
    let events_out = serde_json::to_string(&json!({ "events": &events_json }))?;
    Ok((events_out, events_json))
}
