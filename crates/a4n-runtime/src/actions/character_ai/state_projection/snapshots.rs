use a4n_core::models::character::protagonist_profile;
use a4n_orchestrator::character::snapshot_attributes_from_data;
use serde_json::Value;

use crate::state::AppState;

pub(in crate::actions::character_ai) async fn profile_state_before_chapter(
    state: &AppState,
    profile: &protagonist_profile::Model,
    chapter_number: i32,
) -> Result<Value, String> {
    if chapter_number <= 1 {
        return Ok(profile.base_attributes.clone());
    }

    let snapshot_chapter = chapter_number - 1;
    let snapshot = state
        .orchestrator
        .snapshot_reference_by_chapter(profile.id, snapshot_chapter)
        .await
        .map_err(|error| error.to_string())?
        .ok_or_else(|| {
            format!("第 {snapshot_chapter} 章没有角色快照，无法从第 {chapter_number} 章断点重同步")
        })?;

    snapshot_attributes_from_data(&snapshot.data, snapshot_chapter).map_err(|e| e.to_string())
}
