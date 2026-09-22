use a4n_error::AppError;
use serde_json::Value;

use super::super::super::super::shooting_script_projection::manga_events_from_shooting_script;
use super::super::contracts::MangaRunContext;

pub(super) fn project_shooting_script_events(
    context: &MangaRunContext<'_>,
) -> Result<Value, AppError> {
    let events_json = manga_events_from_shooting_script(&context.shooting_script.script);
    if events_json.as_array().is_none_or(Vec::is_empty) {
        return Err(AppError::InvalidArgument(
            "结构化拍摄剧本没有可投影的漫画事件".to_string(),
        ));
    }
    Ok(events_json)
}
