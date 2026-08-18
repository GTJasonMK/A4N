use a4n_api_contract::project::{ProjectKind, ProjectSourceMode, ProjectStatus};
use a4n_core::models::project::novel_project;
use a4n_orchestrator::collaboration::ProjectAccessInfo;
use a4n_orchestrator::project::ProjectFamilyMetadata;

use super::super::contracts::{ProjectProgressSummary, ProjectSummary};

pub(in crate::actions::project_records::summaries) fn to_summary(
    project: &novel_project::Model,
    metadata: &ProjectFamilyMetadata,
    info: &ProjectAccessInfo,
    project_kind: ProjectKind,
    source_mode: ProjectSourceMode,
    status: ProjectStatus,
    progress: ProjectProgressSummary,
) -> ProjectSummary {
    ProjectSummary {
        id: project.id,
        title: project.title.clone(),
        description: project.description.clone(),
        lifecycle_gate: project.lifecycle_gate.clone(),
        project_type: project.project_type.clone(),
        project_kind,
        source_mode,
        root_project_id: project.root_project_id,
        source_sync_status: project.source_sync_status.clone(),
        root_project_title: metadata.root_project_title.clone(),
        derived_chapter_scope: metadata.derived_chapter_scope.clone(),
        status,
        current_chapter: project.current_chapter,
        target_chapters: project.target_chapters,
        total_chapters: project.total_chapters,
        chapters_per_part: project.chapters_per_part,
        owner_username: info.owner_username.clone(),
        my_role: info.my_role,
        collaboration_root_project_id: info.collaboration_root_project_id,
        member_count: info.member_count,
        pending_invite_count: info.pending_invite_count,
        progress,
        updated_at: project.updated_at.to_string(),
    }
}
