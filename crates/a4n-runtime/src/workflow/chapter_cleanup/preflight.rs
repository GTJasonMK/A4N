use crate::state::AppState;
use a4n_core::models::character::protagonist_profile;
use a4n_orchestrator::project::chapter_cleanup_requires_snapshot;
use std::collections::BTreeSet;

pub(in crate::workflow) async fn preflight_chapter_derived_cleanup(
    state: &AppState,
    project_id: i64,
    chapter_number: i32,
) -> Result<(), String> {
    let profiles = state
        .orchestrator
        .protagonist_profiles_for_project(project_id)
        .await
        .map_err(|err| format!("章节派生数据清理预检失败：{err}"))?;
    let snapshot_chapter = chapter_number - 1;
    let snapshot_profile_ids = profiles
        .iter()
        .filter(|profile| chapter_cleanup_requires_snapshot(profile, chapter_number))
        .map(|profile| profile.id)
        .collect::<Vec<_>>();
    if snapshot_profile_ids.is_empty() {
        return Ok(());
    }

    let existing_snapshot_profile_ids = state
        .orchestrator
        .snapshot_profile_ids_for_chapter(&snapshot_profile_ids, snapshot_chapter)
        .await
        .map_err(|err| format!("章节派生数据清理预检失败：{err}"))?
        .into_iter()
        .collect::<BTreeSet<_>>();
    let missing_names = profiles_missing_cleanup_snapshots(
        &profiles,
        &existing_snapshot_profile_ids,
        chapter_number,
    );
    if missing_names.is_empty() {
        return Ok(());
    }

    Err(format!(
        "章节派生数据清理预检失败：以下角色缺少第 {snapshot_chapter} 章快照，无法在重写第 {chapter_number} 章前回滚档案状态：{}",
        missing_names.join("、")
    ))
}

pub(in crate::workflow::chapter_cleanup) fn profiles_missing_cleanup_snapshots(
    profiles: &[protagonist_profile::Model],
    existing_snapshot_profile_ids: &BTreeSet<i64>,
    chapter_number: i32,
) -> Vec<String> {
    profiles
        .iter()
        .filter(|profile| chapter_cleanup_requires_snapshot(profile, chapter_number))
        .filter(|profile| !existing_snapshot_profile_ids.contains(&profile.id))
        .map(|profile| profile.name.clone())
        .collect()
}
