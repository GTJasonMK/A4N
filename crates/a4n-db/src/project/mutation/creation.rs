use super::contracts::{
    CreateFreeformVideoProjectParams, CreateProjectRecordInput, PROJECT_KIND_TEXT,
    PROJECT_KIND_VIDEO, PROJECT_TYPE_SCRIPT, ProjectMutationError, SOURCE_SYNC_CURRENT,
};
use super::rules::{normalize_project_type, required_project_title};
use a4n_db_models::models::project::novel_project;
use a4n_domain::project::ProjectStatus;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};

pub async fn create_project_record(
    db: &DatabaseConnection,
    input: CreateProjectRecordInput,
) -> Result<novel_project::Model, ProjectMutationError> {
    let title = required_project_title(&input.title)?.to_string();
    let description = input.description.trim().to_string();
    let project_type = normalize_project_type(&input.project_type)?.to_string();
    let now = Utc::now().naive_utc();
    novel_project::ActiveModel {
        user_id: Set(input.user_id),
        title: Set(title),
        description: Set(description),
        project_type: Set(project_type),
        project_kind: Set(PROJECT_KIND_TEXT.to_string()),
        root_project_id: Set(None),
        source_snapshot_id: Set(None),
        source_sync_status: Set(SOURCE_SYNC_CURRENT.to_string()),
        status: Set(ProjectStatus::Draft.as_str().to_string()),
        target_chapters: Set(100),
        current_chapter: Set(0),
        total_chapters: Set(None),
        chapters_per_part: Set(Some(25)),
        avatar_svg: Set(None),
        avatar_animal: Set(None),
        raw_blueprint: Set(None),
        import_analysis_progress: Set(None),
        export_config: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(ProjectMutationError::Database)
}

pub async fn create_freeform_video_project_record(
    db: &DatabaseConnection,
    user_id: i64,
    params: CreateFreeformVideoProjectParams,
) -> Result<novel_project::Model, ProjectMutationError> {
    let title = required_project_title(&params.title)?.to_string();
    let description = params.description.trim().to_string();
    let now = Utc::now().naive_utc();
    novel_project::ActiveModel {
        user_id: Set(user_id),
        title: Set(title),
        description: Set(description),
        project_type: Set(PROJECT_TYPE_SCRIPT.to_string()),
        project_kind: Set(PROJECT_KIND_VIDEO.to_string()),
        root_project_id: Set(None),
        source_snapshot_id: Set(None),
        source_sync_status: Set(SOURCE_SYNC_CURRENT.to_string()),
        status: Set(ProjectStatus::Draft.as_str().to_string()),
        target_chapters: Set(0),
        current_chapter: Set(0),
        total_chapters: Set(None),
        chapters_per_part: Set(None),
        avatar_svg: Set(None),
        avatar_animal: Set(None),
        raw_blueprint: Set(None),
        import_analysis_progress: Set(None),
        export_config: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(ProjectMutationError::Database)
}
