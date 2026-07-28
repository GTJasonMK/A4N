use a4n_core::models::project::novel_project;
use a4n_orchestrator::collaboration::ProjectAccessInfo;
use a4n_orchestrator::project::ProjectFamilyMetadata;

use super::super::contracts::{ProjectProgressSummary, ProjectSummary};

pub(in crate::actions::project_records::summaries) fn to_summary(
    project: &novel_project::Model,
) -> ProjectSummary {
    ProjectSummary {
        id: project.id,
        title: project.title.clone(),
        description: project.description.clone(),
        project_type: project.project_type.clone(),
        project_kind: project.project_kind.clone(),
        source_mode: project.source_mode.clone(),
        root_project_id: project.root_project_id,
        parent_project_id: project.parent_project_id,
        source_snapshot_id: project.source_snapshot_id,
        source_sync_status: project.source_sync_status.clone(),
        derived_project_status: project.derived_project_status.clone(),
        root_project_title: None,
        source_snapshot_created_at: None,
        derived_chapter_scope: None,
        status: project.status.clone(),
        current_chapter: project.current_chapter,
        target_chapters: project.target_chapters,
        total_chapters: project.total_chapters,
        chapters_per_part: project.chapters_per_part,
        owner_user_id: project.user_id,
        owner_username: None,
        my_role: None,
        collaboration_root_project_id: project.root_project_id.unwrap_or(project.id),
        member_count: 0,
        pending_invite_count: None,
        progress: ProjectProgressSummary {
            kind: project.project_kind.clone(),
            stage: "pending".to_string(),
            label: "进度待计算".to_string(),
            current: None,
            total: None,
            status: "idle".to_string(),
            detail: None,
        },
        created_at: Some(project.created_at.to_string()),
        updated_at: Some(project.updated_at.to_string()),
    }
}

pub(in crate::actions::project_records::summaries) fn apply_project_family_metadata(
    summary: &mut ProjectSummary,
    metadata: &ProjectFamilyMetadata,
) {
    summary.root_project_title = metadata.root_project_title.clone();
    summary.source_snapshot_created_at = metadata.source_snapshot_created_at.clone();
    summary.derived_chapter_scope = metadata.derived_chapter_scope.clone();
}

pub(in crate::actions::project_records::summaries) fn apply_project_access_info(
    summary: &mut ProjectSummary,
    info: &ProjectAccessInfo,
) {
    summary.owner_user_id = info.owner_user_id;
    summary.owner_username = info.owner_username.clone();
    summary.my_role = info.my_role.clone();
    summary.collaboration_root_project_id = info.collaboration_root_project_id;
    summary.member_count = info.member_count;
    summary.pending_invite_count = info.pending_invite_count;
}
