//! Shared project summary projection for Host mutations and the project read worker.

use a4n_api_contract::project::ProjectSummary;
use a4n_db::collaboration::ProjectAccessInfo;
use a4n_db::project::ProjectFamilyMetadata;
use a4n_db_models::models::project::novel_project;
use a4n_domain::project::{ProjectKind, ProjectProgressSummary, ProjectSourceMode, ProjectStatus};
use sea_orm::DatabaseConnection;

#[derive(Debug, thiserror::Error)]
pub enum ProjectSummaryError {
    #[error(transparent)]
    Progress(#[from] a4n_db::project::ProjectProgressReadError),
    #[error(transparent)]
    Family(#[from] a4n_db::project::ProjectFamilyMetadataReadError),
    #[error(transparent)]
    Collaboration(#[from] a4n_db::collaboration::CollaborationDbError),
}

pub async fn project_summary_for_user(
    database: &DatabaseConnection,
    user_id: i64,
    project: &novel_project::Model,
) -> Result<ProjectSummary, ProjectSummaryError> {
    let progress_query = a4n_db::project::ProjectProgressQuery::from_project(project)?;
    let project_kind = progress_query.project_kind;
    let source_mode = progress_query.source_mode;
    let status = progress_query.status;
    let family_query = a4n_db::project::ProjectFamilyMetadataQuery::from_project(project);
    let access_query = a4n_db::collaboration::ProjectAccessInfoQuery::from_project(project);
    let (progress, metadata, access) = tokio::try_join!(
        async {
            a4n_db::project::project_progress_summary(database, progress_query)
                .await
                .map_err(ProjectSummaryError::from)
        },
        async {
            a4n_db::project::project_family_metadata(database, family_query)
                .await
                .map_err(ProjectSummaryError::from)
        },
        async {
            a4n_db::collaboration::project_access_info(database, access_query, user_id)
                .await
                .map_err(ProjectSummaryError::from)
        },
    )?;
    Ok(project_summary_from_parts(
        project,
        &metadata,
        &access,
        project_kind,
        source_mode,
        status,
        progress,
    ))
}

pub fn project_summary_from_parts(
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

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_db::collaboration::ProjectAccessRole;

    #[test]
    fn summary_projection_keeps_access_family_and_progress_fields() {
        let project = novel_project::Model {
            id: 9,
            user_id: 3,
            title: "项目".into(),
            description: "说明".into(),
            project_type: "novel_long".into(),
            project_kind: "text".into(),
            root_project_id: None,
            source_snapshot_id: None,
            source_sync_status: "current".into(),
            status: "writing".into(),
            lifecycle_gate: "active".into(),
            target_chapters: 20,
            current_chapter: 4,
            total_chapters: Some(20),
            chapters_per_part: Some(5),
            avatar_svg: None,
            avatar_animal: None,
            raw_blueprint: None,
            import_analysis_progress: None,
            export_config: None,
            created_at: Default::default(),
            updated_at: Default::default(),
        };
        let summary = project_summary_from_parts(
            &project,
            &ProjectFamilyMetadata {
                root_project_title: Some("根项目".into()),
                derived_chapter_scope: None,
            },
            &ProjectAccessInfo {
                collaboration_root_project_id: 9,
                owner_username: "owner".into(),
                my_role: ProjectAccessRole::Editor,
                member_count: 2,
                pending_invite_count: Some(1),
            },
            ProjectKind::Text,
            ProjectSourceMode::Self_,
            ProjectStatus::Writing,
            ProjectProgressSummary {
                label: "4/20".into(),
                detail: None,
            },
        );
        assert_eq!(summary.root_project_title.as_deref(), Some("根项目"));
        assert_eq!(summary.my_role, ProjectAccessRole::Editor);
        assert_eq!(summary.progress.label, "4/20");
    }
}
