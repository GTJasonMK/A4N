use super::projections::required_text;
use crate::AppState;
use crate::actions::{access, app_error_message};
use a4n_core::models::character::{protagonist_profile, snapshot_reference};

pub(super) async fn require_profile_snapshot_ref(
    state: &AppState,
    profile_id: i64,
    chapter_number: i32,
    missing_suffix: &str,
) -> Result<snapshot_reference::Model, String> {
    state
        .orchestrator
        .snapshot_reference_by_chapter(profile_id, chapter_number)
        .await
        .map_err(app_error_message)?
        .ok_or_else(|| format!("第 {chapter_number} 章没有角色快照{missing_suffix}"))
}

pub(super) async fn require_profile_for_user(
    state: &AppState,
    user_id: i64,
    profile_id: i64,
) -> Result<protagonist_profile::Model, String> {
    let profile = state
        .orchestrator
        .profile_by_id(profile_id)
        .await
        .map_err(app_error_message)?
        .ok_or_else(|| format!("角色档案不存在: {profile_id}"))?;
    verify_profile_owner(state, user_id, &profile).await?;
    Ok(profile)
}

pub(super) async fn verify_profile_owner(
    state: &AppState,
    user_id: i64,
    profile: &protagonist_profile::Model,
) -> Result<(), String> {
    access::verify_project_editor_string(state, user_id, profile.project_id).await?;
    Ok(())
}

pub(super) fn required_positive_chapter_number(
    value: Option<i32>,
    label: &str,
) -> Result<i32, String> {
    let chapter_number = value.ok_or_else(|| format!("{label}缺少章节号"))?;
    if chapter_number <= 0 {
        return Err(format!("{label}章节号必须大于 0"));
    }
    Ok(chapter_number)
}

pub(super) fn required_trimmed_string(
    value: Option<String>,
    label: &str,
) -> Result<String, String> {
    let text = value.ok_or_else(|| format!("{label}不能为空"))?;
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(format!("{label}不能为空"));
    }
    Ok(trimmed.to_string())
}

pub(super) async fn find_profile_by_name_for_user(
    state: &AppState,
    user_id: i64,
    project_id: i64,
    name: &str,
) -> Result<protagonist_profile::Model, String> {
    access::verify_project_editor_string(state, user_id, project_id).await?;
    let name = required_text(name, "角色名称")?;
    state
        .orchestrator
        .profile_by_project_name(project_id, &name)
        .await
        .map_err(app_error_message)?
        .ok_or_else(|| format!("角色不存在: project_id={project_id}, name={name}"))
}

pub(super) fn validate_snapshot_range(
    start_chapter: Option<i32>,
    end_chapter: Option<i32>,
) -> Result<(), String> {
    if start_chapter.is_some_and(|value| value <= 0) {
        return Err("快照起始章节必须大于 0".to_string());
    }
    if end_chapter.is_some_and(|value| value <= 0) {
        return Err("快照结束章节必须大于 0".to_string());
    }
    if let (Some(start), Some(end)) = (start_chapter, end_chapter)
        && start > end
    {
        return Err("快照起始章节不能大于结束章节".to_string());
    }
    Ok(())
}
