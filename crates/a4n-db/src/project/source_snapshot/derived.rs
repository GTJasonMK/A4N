use super::super::mutation::required_project_title;
use super::{
    super::{
        PROJECT_KIND_TEXT, ProjectMutationError, SOURCE_SYNC_CURRENT,
        normalize_derived_project_kind,
    },
    creation::{create_source_snapshot_in_record, normalize_chapter_scope},
    types::CreateDerivedProjectParams,
};
use a4n_db_models::models::project::novel_project;
use a4n_domain::project::ProjectStatus;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ConnectionTrait, DatabaseConnection, DbBackend, EntityTrait, QuerySelect,
    Set, TransactionTrait,
};

pub async fn create_derived_project_record(
    db: &DatabaseConnection,
    user_id: i64,
    params: CreateDerivedProjectParams,
) -> Result<novel_project::Model, ProjectMutationError> {
    let derived_kind = normalize_derived_project_kind(&params.derived_kind)?.to_string();
    let title = required_project_title(&params.title)?.to_string();
    let description = params.description.trim().to_string();
    let scope = normalize_chapter_scope(params.chapter_scope)?;

    let tx = db.begin().await.map_err(ProjectMutationError::Database)?;
    let mut root_query = novel_project::Entity::find_by_id(params.root_project_id)
        .select_only()
        .column(novel_project::Column::Id)
        .column(novel_project::Column::UserId)
        .column(novel_project::Column::ProjectType)
        .column(novel_project::Column::ProjectKind)
        .column(novel_project::Column::ChaptersPerPart)
        .column(novel_project::Column::AvatarSvg)
        .column(novel_project::Column::AvatarAnimal);
    if tx.get_database_backend() != DbBackend::Sqlite {
        root_query = root_query.lock_exclusive();
    }
    let root = root_query
        .into_tuple::<(
            i64,
            i64,
            String,
            String,
            Option<i32>,
            Option<String>,
            Option<String>,
        )>()
        .one(&tx)
        .await
        .map_err(ProjectMutationError::Database)?
        .ok_or_else(|| {
            ProjectMutationError::NotFound(format!("根文本项目不存在: {}", params.root_project_id))
        })?;
    if root.1 != user_id {
        return Err(ProjectMutationError::Forbidden("无权操作根文本项目".into()));
    }
    if root.3 != PROJECT_KIND_TEXT {
        return Err(ProjectMutationError::InvalidArgument(
            "派生项目的根项目必须是文本项目".to_string(),
        ));
    }

    let now = Utc::now().naive_utc();
    let snapshot = create_source_snapshot_in_record(&tx, root.0, scope, now).await?;
    let derived = novel_project::ActiveModel {
        user_id: Set(user_id),
        title: Set(title),
        description: Set(description),
        project_type: Set(root.2.clone()),
        project_kind: Set(derived_kind.clone()),
        root_project_id: Set(Some(root.0)),
        source_snapshot_id: Set(Some(snapshot.id)),
        source_sync_status: Set(SOURCE_SYNC_CURRENT.to_string()),
        status: Set(ProjectStatus::Draft.as_str().to_string()),
        target_chapters: Set(scope.end_chapter - scope.start_chapter + 1),
        current_chapter: Set(0),
        total_chapters: Set(Some(scope.end_chapter - scope.start_chapter + 1)),
        chapters_per_part: Set(root.4),
        avatar_svg: Set(root.5.clone()),
        avatar_animal: Set(root.6.clone()),
        raw_blueprint: Set(None),
        import_analysis_progress: Set(None),
        export_config: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&tx)
    .await
    .map_err(ProjectMutationError::Database)?;

    tx.commit().await.map_err(ProjectMutationError::Database)?;
    Ok(derived)
}
