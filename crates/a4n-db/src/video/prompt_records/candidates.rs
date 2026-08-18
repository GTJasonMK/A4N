use a4n_core::models::chapter::chapter_video_prompt;
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};

use super::super::{
    VideoDbError, validate_positive_video_i32, validate_positive_video_i64,
    validate_positive_video_ids,
};

pub async fn chapter_video_prompts_for_batch_generation(
    db: &DatabaseConnection,
    prompt_ids: &[i64],
) -> Result<Vec<chapter_video_prompt::Model>, VideoDbError> {
    validate_positive_video_ids(prompt_ids, "prompt_ids")?;
    if prompt_ids.is_empty() {
        return Ok(Vec::new());
    }
    chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::Id.is_in(prompt_ids.iter().copied()))
        .order_by_asc(chapter_video_prompt::Column::ShotNumber)
        .order_by_asc(chapter_video_prompt::Column::Id)
        .all(db)
        .await
        .map_err(VideoDbError::Database)
}

pub async fn chapter_video_boundary_frame_prompt_candidates(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
    first_frame_number: i32,
    last_frame_number: i32,
) -> Result<Vec<chapter_video_prompt::Model>, VideoDbError> {
    validate_positive_video_i64(project_id, "project_id")?;
    validate_positive_video_i64(chapter_id, "chapter_id")?;
    validate_positive_video_i32(first_frame_number, "first_frame_number")?;
    validate_positive_video_i32(last_frame_number, "last_frame_number")?;
    if first_frame_number > last_frame_number {
        return Err(VideoDbError::InvalidArgument(
            "first_frame_number 不能大于 last_frame_number".to_string(),
        ));
    }
    chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(
            chapter_video_prompt::Column::PromptType.eq(chapter_video_prompt::PROMPT_TYPE_FRAME),
        )
        .filter(
            chapter_video_prompt::Column::ShotNumber.is_in([first_frame_number, last_frame_number]),
        )
        .order_by_desc(chapter_video_prompt::Column::Id)
        .all(db)
        .await
        .map_err(VideoDbError::Database)
}

pub async fn chapter_video_first_frame_prompt_candidates(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<chapter_video_prompt::Model>, VideoDbError> {
    validate_positive_video_i64(project_id, "project_id")?;
    validate_positive_video_i64(chapter_id, "chapter_id")?;
    chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(
            chapter_video_prompt::Column::PromptType
                .eq(chapter_video_prompt::PROMPT_TYPE_FIRST_FRAME),
        )
        .filter(chapter_video_prompt::Column::ShotNumber.eq(1))
        .order_by_desc(chapter_video_prompt::Column::Id)
        .all(db)
        .await
        .map_err(VideoDbError::Database)
}

pub async fn chapter_video_first_frame_chain_prompt_candidates(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
) -> Result<Vec<chapter_video_prompt::Model>, VideoDbError> {
    validate_positive_video_i64(project_id, "project_id")?;
    validate_positive_video_i64(chapter_id, "chapter_id")?;
    chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(
            chapter_video_prompt::Column::PromptType.eq(chapter_video_prompt::PROMPT_TYPE_VIDEO),
        )
        .filter(
            chapter_video_prompt::Column::WorkflowMode
                .eq(chapter_video_prompt::WORKFLOW_MODE_FIRST_FRAME_CHAIN),
        )
        .order_by_desc(chapter_video_prompt::Column::Id)
        .all(db)
        .await
        .map_err(VideoDbError::Database)
}

pub async fn chapter_video_storyboard_prompt_candidates(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
    shot_number: i32,
) -> Result<Vec<chapter_video_prompt::Model>, VideoDbError> {
    validate_positive_video_i64(project_id, "project_id")?;
    validate_positive_video_i64(chapter_id, "chapter_id")?;
    validate_positive_video_i32(shot_number, "shot_number")?;
    chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::ProjectId.eq(project_id))
        .filter(chapter_video_prompt::Column::ChapterId.eq(chapter_id))
        .filter(
            chapter_video_prompt::Column::WorkflowMode
                .eq(chapter_video_prompt::WORKFLOW_MODE_STORYBOARD),
        )
        .filter(
            chapter_video_prompt::Column::PromptType
                .eq(chapter_video_prompt::PROMPT_TYPE_STORYBOARD),
        )
        .filter(chapter_video_prompt::Column::ShotNumber.eq(shot_number))
        .order_by_desc(chapter_video_prompt::Column::Id)
        .all(db)
        .await
        .map_err(VideoDbError::Database)
}
