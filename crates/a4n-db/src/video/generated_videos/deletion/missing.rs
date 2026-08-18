use crate::video::VideoDbError;
use a4n_core::models::{ai::generated_video, chapter::chapter_video_prompt};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set, TransactionTrait,
};
use std::collections::BTreeSet;

async fn delete_missing_generated_video_references_record_in<C>(
    db: &C,
    video_id: i64,
) -> Result<u64, VideoDbError>
where
    C: ConnectionTrait,
{
    if generated_video::Entity::find_by_id(video_id)
        .one(db)
        .await?
        .is_some()
    {
        return Err(VideoDbError::InvalidArgument(
            "视频记录仍存在，请使用正常视频删除流程".to_string(),
        ));
    }

    let prompts = chapter_video_prompt::Entity::find()
        .filter(chapter_video_prompt::Column::GeneratedVideoId.eq(video_id))
        .all(db)
        .await?;
    let mut project_ids = prompts
        .iter()
        .map(|prompt| prompt.project_id)
        .collect::<BTreeSet<_>>();
    project_ids.extend(crate::canvas::generated_video_ref_project_ids(db, video_id).await?);
    project_ids.extend(
        crate::media::generated_video_target_media_binding_project_ids(db, &[video_id]).await?,
    );
    let deleted_bindings =
        crate::media::delete_generated_video_target_media_bindings(db, &[video_id]).await?;
    if prompts.is_empty() && project_ids.is_empty() && deleted_bindings == 0 {
        return Err(VideoDbError::NotFound("视频不存在".to_string()));
    }

    let now = Utc::now().naive_utc();
    for prompt in prompts.iter().cloned() {
        let mut active: chapter_video_prompt::ActiveModel = prompt.into();
        active.generated_video_id = Set(None);
        active.status = Set(chapter_video_prompt::STATUS_PENDING.to_string());
        active.updated_at = Set(now);
        active.update(db).await?;
    }
    for project_id in project_ids {
        crate::canvas::mark_canvas_generated_video_refs_missing(db, project_id, video_id).await?;
    }
    crate::generation::delete_generation_run_targets_for(db, "generated_video", &[video_id])
        .await?;
    Ok(prompts.len() as u64)
}

pub async fn delete_missing_generated_video_references_record<C>(
    db: &C,
    video_id: i64,
) -> Result<u64, VideoDbError>
where
    C: ConnectionTrait + TransactionTrait,
{
    let txn = db.begin().await.map_err(VideoDbError::Database)?;
    let cleared = delete_missing_generated_video_references_record_in(&txn, video_id).await?;
    txn.commit().await.map_err(VideoDbError::Database)?;
    Ok(cleared)
}
