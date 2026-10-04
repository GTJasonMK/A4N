use super::contracts::ProjectMutationError;
use crate::project::project_record_from_model;
use a4n_db_models::models::project::novel_project;
use a4n_domain::project::{ProjectRecord, ProjectSummaryRecord};
use sea_orm::{ConnectionTrait, DbBackend, DbErr, EntityTrait, FromQueryResult, QuerySelect};

#[derive(Debug, FromQueryResult)]
struct ProjectSummaryRow {
    id: i64,
    user_id: i64,
    title: String,
    description: String,
    project_type: String,
    project_kind: String,
    root_project_id: Option<i64>,
    source_snapshot_id: Option<i64>,
    source_sync_status: String,
    status: String,
    lifecycle_gate: String,
    target_chapters: i32,
    current_chapter: i32,
    total_chapters: Option<i32>,
    chapters_per_part: Option<i32>,
    created_at: chrono::NaiveDateTime,
    updated_at: chrono::NaiveDateTime,
}

impl ProjectSummaryRow {
    fn into_record(self) -> ProjectSummaryRecord {
        ProjectSummaryRecord {
            id: self.id,
            user_id: self.user_id,
            title: self.title,
            description: self.description,
            project_type: self.project_type,
            project_kind: self.project_kind,
            root_project_id: self.root_project_id,
            source_snapshot_id: self.source_snapshot_id,
            source_sync_status: self.source_sync_status,
            status: self.status,
            lifecycle_gate: self.lifecycle_gate,
            target_chapters: self.target_chapters,
            current_chapter: self.current_chapter,
            total_chapters: self.total_chapters,
            chapters_per_part: self.chapters_per_part,
            created_at: self.created_at,
            updated_at: self.updated_at,
        }
    }
}

#[derive(Debug, FromQueryResult)]
pub(crate) struct ProjectWriteIdentity {
    pub(crate) lifecycle_gate: String,
}

pub async fn project_by_id_record<C>(
    db: &C,
    project_id: i64,
) -> Result<Option<ProjectRecord>, ProjectMutationError>
where
    C: ConnectionTrait,
{
    novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map(|row| row.map(project_record_from_model))
        .map_err(ProjectMutationError::Database)
}

pub async fn project_summary_by_id_record<C>(
    db: &C,
    project_id: i64,
) -> Result<Option<ProjectSummaryRecord>, ProjectMutationError>
where
    C: ConnectionTrait,
{
    novel_project::Entity::find_by_id(project_id)
        .select_only()
        .column(novel_project::Column::Id)
        .column(novel_project::Column::UserId)
        .column(novel_project::Column::Title)
        .column(novel_project::Column::Description)
        .column(novel_project::Column::ProjectType)
        .column(novel_project::Column::ProjectKind)
        .column(novel_project::Column::RootProjectId)
        .column(novel_project::Column::SourceSnapshotId)
        .column(novel_project::Column::SourceSyncStatus)
        .column(novel_project::Column::Status)
        .column(novel_project::Column::LifecycleGate)
        .column(novel_project::Column::TargetChapters)
        .column(novel_project::Column::CurrentChapter)
        .column(novel_project::Column::TotalChapters)
        .column(novel_project::Column::ChaptersPerPart)
        .column(novel_project::Column::CreatedAt)
        .column(novel_project::Column::UpdatedAt)
        .into_model::<ProjectSummaryRow>()
        .one(db)
        .await
        .map(|row| row.map(ProjectSummaryRow::into_record))
        .map_err(ProjectMutationError::Database)
}

pub(crate) async fn project_by_id_for_update<C>(
    db: &C,
    project_id: i64,
) -> Result<Option<novel_project::Model>, DbErr>
where
    C: ConnectionTrait,
{
    let mut query = novel_project::Entity::find_by_id(project_id);
    if db.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    query.one(db).await
}

pub(crate) async fn project_write_identity_for_update<C>(
    db: &C,
    project_id: i64,
) -> Result<Option<ProjectWriteIdentity>, DbErr>
where
    C: ConnectionTrait,
{
    let mut query = novel_project::Entity::find_by_id(project_id)
        .select_only()
        .column(novel_project::Column::LifecycleGate);
    if db.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    query.into_model::<ProjectWriteIdentity>().one(db).await
}

pub async fn project_ids(db: &impl crate::DbExecutor) -> Result<Vec<i64>, ProjectMutationError> {
    novel_project::Entity::find()
        .select_only()
        .column(novel_project::Column::Id)
        .into_tuple::<i64>()
        .all(db)
        .await
        .map_err(ProjectMutationError::Database)
}

#[cfg(test)]
mod tests {
    use super::ProjectSummaryRow;
    use chrono::Utc;

    #[test]
    fn summary_row_preserves_contract_fields_without_large_payloads() {
        let now = Utc::now().naive_utc();
        let record = ProjectSummaryRow {
            id: 7,
            user_id: 3,
            title: "项目".to_string(),
            description: "描述".to_string(),
            project_type: "novel_long".to_string(),
            project_kind: "text".to_string(),
            root_project_id: None,
            source_snapshot_id: None,
            source_sync_status: "current".to_string(),
            status: "writing".to_string(),
            lifecycle_gate: "active".to_string(),
            target_chapters: 10,
            current_chapter: 2,
            total_chapters: Some(10),
            chapters_per_part: Some(5),
            created_at: now,
            updated_at: now,
        }
        .into_record();

        assert_eq!(record.id, 7);
        assert_eq!(record.title, "项目");
        assert_eq!(record.current_chapter, 2);
    }
}
