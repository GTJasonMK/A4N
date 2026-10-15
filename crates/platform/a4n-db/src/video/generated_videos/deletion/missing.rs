use crate::video::VideoDbError;
use a4n_db_models::models::{ai::generated_video, chapter::chapter_video_prompt};
use chrono::Utc;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect, TransactionTrait,
    sea_query::Expr,
};
use std::collections::BTreeSet;

async fn delete_missing_generated_video_references_record_in<C>(
    db: &C,
    video_id: i64,
) -> Result<super::VideoReferenceCleanupIds, VideoDbError>
where
    C: ConnectionTrait,
{
    if generated_video::Entity::find_by_id(video_id)
        .select_only()
        .column(generated_video::Column::Id)
        .into_tuple::<i64>()
        .one(db)
        .await?
        .is_some()
    {
        return Err(VideoDbError::InvalidArgument(
            "视频记录仍存在，请使用正常视频删除流程".to_string(),
        ));
    }

    let initial_prompts = chapter_video_prompt::Entity::find()
        .select_only()
        .column(chapter_video_prompt::Column::Id)
        .column(chapter_video_prompt::Column::ProjectId)
        .filter(chapter_video_prompt::Column::GeneratedVideoId.eq(video_id))
        .into_tuple::<(i64, i64)>()
        .all(db)
        .await?;
    let mut initial_project_ids = initial_prompts
        .iter()
        .map(|(_, project_id)| *project_id)
        .collect::<BTreeSet<_>>();
    initial_project_ids.extend(crate::canvas::generated_video_ref_project_ids(db, video_id).await?);
    initial_project_ids.extend(
        crate::media::generated_video_target_media_binding_project_ids(db, &[video_id]).await?,
    );
    if initial_prompts.is_empty() && initial_project_ids.is_empty() {
        return Err(VideoDbError::NotFound("视频不存在".to_string()));
    }

    for project_id in &initial_project_ids {
        crate::video::lock_generated_media_project(db, *project_id).await?;
    }

    let prompts = chapter_video_prompt::Entity::find()
        .select_only()
        .column(chapter_video_prompt::Column::Id)
        .column(chapter_video_prompt::Column::ProjectId)
        .filter(chapter_video_prompt::Column::GeneratedVideoId.eq(video_id))
        .into_tuple::<(i64, i64)>()
        .all(db)
        .await?;
    let mut project_ids = prompts
        .iter()
        .map(|(_, project_id)| *project_id)
        .collect::<BTreeSet<_>>();
    project_ids.extend(crate::canvas::generated_video_ref_project_ids(db, video_id).await?);
    project_ids.extend(
        crate::media::generated_video_target_media_binding_project_ids(db, &[video_id]).await?,
    );
    for project_id in project_ids.difference(&initial_project_ids) {
        crate::video::lock_generated_media_project(db, *project_id).await?;
    }
    let deleted_bindings =
        crate::media::delete_generated_video_target_media_bindings(db, &[video_id]).await?;
    if prompts.is_empty() && project_ids.is_empty() && deleted_bindings == 0 {
        return Err(VideoDbError::NotFound("视频不存在".to_string()));
    }

    let now = Utc::now().naive_utc();
    let updated_prompt_ids = prompts
        .iter()
        .map(|(prompt_id, _)| *prompt_id)
        .collect::<Vec<_>>();
    if !prompts.is_empty() {
        chapter_video_prompt::Entity::update_many()
            .col_expr(
                chapter_video_prompt::Column::GeneratedVideoId,
                Expr::value(Option::<i64>::None),
            )
            .col_expr(
                chapter_video_prompt::Column::Status,
                Expr::value(chapter_video_prompt::STATUS_PENDING),
            )
            .col_expr(chapter_video_prompt::Column::UpdatedAt, Expr::value(now))
            .filter(
                chapter_video_prompt::Column::Id.is_in(
                    prompts
                        .iter()
                        .map(|(prompt_id, _)| *prompt_id)
                        .collect::<Vec<_>>(),
                ),
            )
            .exec(db)
            .await?;
    }
    let project_ids = project_ids.into_iter().collect::<Vec<_>>();
    crate::canvas::mark_canvas_generated_video_refs_missing_for_projects(
        db,
        &project_ids,
        video_id,
    )
    .await?;
    crate::generation::delete_generation_run_targets_for(db, "generated_video", &[video_id])
        .await?;
    Ok(super::VideoReferenceCleanupIds {
        updated_prompt_ids,
        updated_video_ids: Vec::new(),
    })
}

pub async fn delete_missing_generated_video_references_record<C>(
    db: &C,
    video_id: i64,
) -> Result<u64, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let effects =
        delete_missing_generated_video_references_with_effects_record(db, video_id).await?;
    Ok(effects.updated_prompt_ids.len() as u64)
}

pub async fn delete_missing_generated_video_references_with_effects_record<C>(
    db: &C,
    video_id: i64,
) -> Result<super::VideoReferenceCleanupIds, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    let effects = delete_missing_generated_video_references_record_in(&txn, video_id).await?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(effects)
}
