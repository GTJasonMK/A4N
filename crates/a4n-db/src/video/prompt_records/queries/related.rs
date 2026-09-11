use a4n_db_models::models::chapter::chapter_video_prompt;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, QueryOrder, QuerySelect,
};

use super::super::types::PromptDependencyRecord;

pub(crate) async fn chapter_video_prompt_sibling_records<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    prompt_id: i64,
) -> Result<Vec<chapter_video_prompt::Model>, DbErr>
where
    C: ConnectionTrait,
{
    chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::Id.ne(prompt_id))
        .order_by_asc(chapter_video_prompt::Column::Id)
        .all(db)
        .await
}

pub(crate) async fn chapter_video_prompt_sibling_dependency_records<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    prompt_id: i64,
) -> Result<Vec<PromptDependencyRecord>, DbErr>
where
    C: ConnectionTrait,
{
    chapter_video_prompt_dependency_query(project_id, chapter_id)
        .filter(chapter_video_prompt::Column::Id.ne(prompt_id))
        .into_model::<PromptDependencyRecord>()
        .all(db)
        .await
}

pub(crate) async fn chapter_video_prompt_dependency_records<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<PromptDependencyRecord>, DbErr>
where
    C: ConnectionTrait,
{
    chapter_video_prompt_dependency_query(project_id, chapter_id)
        .into_model::<PromptDependencyRecord>()
        .all(db)
        .await
}

fn chapter_video_prompt_dependency_query(
    project_id: i64,
    chapter_id: i64,
) -> sea_orm::Select<chapter_video_prompt::Entity> {
    chapter_video_prompt::Entity::find()
        .select_only()
        .column(chapter_video_prompt::Column::Id)
        .column(chapter_video_prompt::Column::ProjectId)
        .column(chapter_video_prompt::Column::ChapterId)
        .column(chapter_video_prompt::Column::Style)
        .column(chapter_video_prompt::Column::PromptType)
        .column(chapter_video_prompt::Column::WorkflowMode)
        .column(chapter_video_prompt::Column::AspectRatio)
        .column(chapter_video_prompt::Column::MetadataJson)
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .order_by_asc(chapter_video_prompt::Column::Id)
}

pub(crate) async fn chapter_video_prompt_previous_frame_candidates<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: &str,
    prompt_type: &str,
    frame_number: i32,
) -> Result<Vec<PromptDependencyRecord>, DbErr>
where
    C: ConnectionTrait,
{
    chapter_video_prompt::Entity::find()
        .select_only()
        .column(chapter_video_prompt::Column::Id)
        .column(chapter_video_prompt::Column::ProjectId)
        .column(chapter_video_prompt::Column::ChapterId)
        .column(chapter_video_prompt::Column::Style)
        .column(chapter_video_prompt::Column::PromptType)
        .column(chapter_video_prompt::Column::WorkflowMode)
        .column(chapter_video_prompt::Column::AspectRatio)
        .column(chapter_video_prompt::Column::MetadataJson)
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq(workflow_mode))
        .filter(chapter_video_prompt::Column::PromptType.eq(prompt_type))
        .filter(chapter_video_prompt::Column::ShotNumber.eq(frame_number))
        .filter(chapter_video_prompt::Column::Status.ne(chapter_video_prompt::STATUS_RUNNING))
        .order_by_desc(chapter_video_prompt::Column::UpdatedAt)
        .order_by_desc(chapter_video_prompt::Column::Id)
        .into_model::<PromptDependencyRecord>()
        .all(db)
        .await
}
