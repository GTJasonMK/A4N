use super::*;

pub(super) async fn named_profile(
    database: &DbHandle,
    project_id: i64,
    name: &str,
) -> Result<a4n_domain::character::CharacterProfileRecord, WorkerFailure> {
    let name = name.trim();
    if name.is_empty() {
        return Err(WorkerFailure::invalid_request("角色名称不能为空"));
    }
    a4n_db::character::profile_record_by_project_name(database.connection(), project_id, name)
        .await
        .map_err(|error| WorkerFailure::unavailable(error.to_string()))?
        .ok_or_else(|| {
            WorkerFailure::not_found(format!("角色不存在: project_id={project_id}, name={name}"))
        })
}

pub(super) async fn profile_for_editor(
    database: &DbHandle,
    user_id: i64,
    profile_id: i64,
) -> Result<a4n_domain::character::CharacterProfileRecord, WorkerFailure> {
    let profile_id = positive(profile_id, "profile_id")?;
    let profile = a4n_db::character::profile_record_by_id(database.connection(), profile_id)
        .await
        .map_err(|error| WorkerFailure::unavailable(error.to_string()))?
        .ok_or_else(|| WorkerFailure::not_found(format!("角色档案不存在: {profile_id}")))?;
    verify_project_editor(database, profile.project_id, user_id).await?;
    Ok(profile)
}

pub(super) fn validate_snapshot_range(
    start_chapter: Option<i32>,
    end_chapter: Option<i32>,
) -> Result<(), WorkerFailure> {
    if start_chapter.is_some_and(|chapter| chapter <= 0) {
        return Err(WorkerFailure::invalid_request("快照起始章节必须大于 0"));
    }
    if end_chapter.is_some_and(|chapter| chapter <= 0) {
        return Err(WorkerFailure::invalid_request("快照结束章节必须大于 0"));
    }
    if let (Some(start), Some(end)) = (start_chapter, end_chapter)
        && start > end
    {
        return Err(WorkerFailure::invalid_request(
            "快照起始章节不能大于结束章节",
        ));
    }
    Ok(())
}
