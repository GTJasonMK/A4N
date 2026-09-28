use a4n_error::AppError;
use serde_json::Value;

use super::super::super::super::adaptation_projection::manga_events_from_adaptation;
use super::super::contracts::MangaPreviewContext;

pub(super) fn preview_events(context: &MangaPreviewContext<'_>) -> Result<Value, AppError> {
    let events_json = manga_events_from_adaptation(&context.adaptation.plan);
    if events_json.as_array().is_none_or(Vec::is_empty) {
        return Err(AppError::InvalidArgument(
            "本项目改编计划没有可用的漫画事件".to_string(),
        ));
    }
    Ok(events_json)
}
