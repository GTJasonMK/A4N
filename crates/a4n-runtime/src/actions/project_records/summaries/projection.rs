use a4n_api_contract::{
    collaboration::ProjectAccessRole,
    project::{ProjectKind, ProjectStatus},
};
use a4n_core::models::project::novel_project;
use a4n_orchestrator::collaboration::ProjectAccessInfo;
use a4n_orchestrator::project::ProjectFamilyMetadata;

use super::super::contracts::{ProjectProgressSummary, ProjectSummary};

pub(in crate::actions::project_records::summaries) fn to_summary(
    project: &novel_project::Model,
    metadata: &ProjectFamilyMetadata,
    info: &ProjectAccessInfo,
    progress: ProjectProgressSummary,
) -> Result<ProjectSummary, String> {
    let status = ProjectStatus::parse(&project.status)
        .ok_or_else(|| format!("项目 #{} 的状态无效: {}", project.id, project.status))?;
    let project_kind = ProjectKind::parse(&project.project_kind).ok_or_else(|| {
        format!(
            "项目 #{} 的项目族无效: {}",
            project.id, project.project_kind
        )
    })?;
    let owner_username = info
        .owner_username
        .clone()
        .ok_or_else(|| format!("项目 #{} 缺少所有者用户名", project.id))?;
    let my_role = match info.my_role.as_deref() {
        Some("owner") => ProjectAccessRole::Owner,
        Some("manager") => ProjectAccessRole::Manager,
        Some("editor") => ProjectAccessRole::Editor,
        Some("viewer") => ProjectAccessRole::Viewer,
        Some(role) => return Err(format!("项目 #{} 的访问角色无效: {role}", project.id)),
        None => return Err(format!("项目 #{} 缺少当前用户访问角色", project.id)),
    };
    Ok(ProjectSummary {
        id: project.id,
        title: project.title.clone(),
        description: project.description.clone(),
        lifecycle_gate: project.lifecycle_gate.clone(),
        project_type: project.project_type.clone(),
        project_kind,
        source_mode: project.source_mode.clone(),
        root_project_id: project.root_project_id,
        source_sync_status: project.source_sync_status.clone(),
        root_project_title: metadata.root_project_title.clone(),
        derived_chapter_scope: metadata.derived_chapter_scope.clone(),
        status,
        current_chapter: project.current_chapter,
        target_chapters: project.target_chapters,
        total_chapters: project.total_chapters,
        chapters_per_part: project.chapters_per_part,
        owner_username,
        my_role,
        collaboration_root_project_id: info.collaboration_root_project_id,
        member_count: info.member_count,
        pending_invite_count: info.pending_invite_count,
        progress,
        updated_at: project.updated_at.to_string(),
    })
}
