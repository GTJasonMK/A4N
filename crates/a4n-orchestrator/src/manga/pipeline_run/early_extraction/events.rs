use a4n_core::error::AppError;
use serde_json::{Map, Value, json};

use super::super::super::checkpoint::checkpoint_insert;
use super::super::super::shooting_script_projection::manga_events_from_shooting_script;
use super::super::contracts::MangaRunContext;

pub(super) fn project_shooting_script_events(
    context: &MangaRunContext<'_>,
    checkpoint: &mut Map<String, Value>,
) -> Result<(String, Value), AppError> {
    let events_json = manga_events_from_shooting_script(&context.shooting_script.script);
    if events_json.as_array().is_none_or(Vec::is_empty) {
        return Err(AppError::InvalidArgument(
            "结构化拍摄剧本没有可投影的漫画事件".to_string(),
        ));
    }
    let output = json!({ "events": &events_json });
    let events_out = serde_json::to_string(&output)?;
    checkpoint_insert(
        checkpoint,
        "shooting_script_id",
        json!(context.shooting_script.record.id),
    );
    checkpoint_insert(
        checkpoint,
        "shooting_script_revision",
        json!(context.shooting_script.record.revision),
    );
    checkpoint_insert(checkpoint, "events", events_json.clone());
    Ok((events_out, events_json))
}
