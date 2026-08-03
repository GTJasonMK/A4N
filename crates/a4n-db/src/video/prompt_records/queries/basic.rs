use a4n_core::models::chapter::chapter_video_prompt;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect,
    sea_query::{Expr, extension::postgres::PgExpr},
};

use crate::video::{VideoDbError, required_video_record_text, validate_positive_video_i64};

pub async fn chapter_video_prompt_by_id(
    db: &DatabaseConnection,
    prompt_id: i64,
) -> Result<Option<chapter_video_prompt::Model>, VideoDbError> {
    validate_positive_video_i64(prompt_id, "prompt_id")?;
    chapter_video_prompt::Entity::find_by_id(prompt_id)
        .one(db)
        .await
        .map_err(VideoDbError::Database)
}

pub async fn chapter_video_prompt_scope_by_id<C>(
    db: &C,
    prompt_id: i64,
) -> Result<Option<(i64, i64)>, VideoDbError>
where
    C: ConnectionTrait,
{
    let prompt_id = validate_positive_video_i64(prompt_id, "prompt_id")?;
    chapter_video_prompt::Entity::find_by_id(prompt_id)
        .select_only()
        .column(chapter_video_prompt::Column::ProjectId)
        .column(chapter_video_prompt::Column::ChapterId)
        .into_tuple::<(i64, i64)>()
        .one(db)
        .await
        .map_err(VideoDbError::Database)
}

pub async fn chapter_video_prompts_for_chapter<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<chapter_video_prompt::Model>, DbErr>
where
    C: ConnectionTrait,
{
    chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .order_by_asc(chapter_video_prompt::Column::WorkflowMode)
        .order_by_asc(chapter_video_prompt::Column::ShotNumber)
        .order_by_asc(chapter_video_prompt::Column::Id)
        .all(db)
        .await
}

pub async fn completed_shared_segment_plan_id_for_source_hash<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    source_content_sha256: &str,
) -> Result<Option<i64>, VideoDbError>
where
    C: ConnectionTrait,
{
    let project_id = validate_positive_video_i64(project_id, "project_id")?;
    let chapter_id = validate_positive_video_i64(chapter_id, "chapter_id")?;
    let source_content_sha256 =
        required_video_record_text(source_content_sha256, "source_content_sha256")?;
    chapter_video_prompt::Entity::find()
        .select_only()
        .column(chapter_video_prompt::Column::Id)
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(chapter_video_prompt::Column::PromptType.eq("frame_segment_plan"))
        .filter(chapter_video_prompt::Column::WorkflowMode.eq("shared_segment_plan"))
        .filter(chapter_video_prompt::Column::Status.eq("completed"))
        .filter(
            Expr::col(chapter_video_prompt::Column::MetadataJson)
                .get_json_field("video_execution_identity")
                .cast_json_field("source_content_sha256")
                .eq(source_content_sha256),
        )
        .into_tuple::<i64>()
        .one(db)
        .await
        .map_err(VideoDbError::Database)
}

pub async fn chapter_video_prompt_record<C>(
    db: &C,
    prompt_id: i64,
) -> Result<chapter_video_prompt::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    chapter_video_prompt_record_with_message(db, prompt_id, "视频提示词不存在").await
}

pub async fn frame_chapter_video_prompt_record<C>(
    db: &C,
    prompt_id: i64,
) -> Result<chapter_video_prompt::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    chapter_video_prompt_record_with_message(db, prompt_id, "视频帧提示词不存在").await
}

async fn chapter_video_prompt_record_with_message<C>(
    db: &C,
    prompt_id: i64,
    not_found_message: &str,
) -> Result<chapter_video_prompt::Model, VideoDbError>
where
    C: ConnectionTrait,
{
    chapter_video_prompt::Entity::find_by_id(prompt_id)
        .one(db)
        .await?
        .ok_or_else(|| VideoDbError::NotFound(not_found_message.to_string()))
}

pub async fn chapter_video_prompt_records_by_ids<C>(
    db: &C,
    prompt_ids: &[i64],
) -> Result<Vec<chapter_video_prompt::Model>, VideoDbError>
where
    C: ConnectionTrait,
{
    if prompt_ids.is_empty() {
        return Ok(Vec::new());
    }
    chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::Id.is_in(prompt_ids.iter().copied()))
        .all(db)
        .await
        .map_err(VideoDbError::Database)
}
