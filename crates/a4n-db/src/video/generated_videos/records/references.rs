use std::collections::BTreeSet;

use a4n_db_models::models::chapter::chapter_video_prompt;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QuerySelect};

use crate::video::VideoDbError;

pub async fn missing_video_reference_project_ids<C>(
    db: &C,
    video_id: i64,
) -> Result<Vec<i64>, VideoDbError>
where
    C: ConnectionTrait,
{
    let mut project_ids = chapter_video_prompt::Entity::find()
        .select_only()
        .column(chapter_video_prompt::Column::ProjectId)
        .filter(chapter_video_prompt::Column::GeneratedVideoId.eq(video_id))
        .into_tuple::<i64>()
        .all(db)
        .await?
        .into_iter()
        .collect::<BTreeSet<_>>();
    project_ids.extend(crate::canvas::generated_video_ref_project_ids(db, video_id).await?);
    project_ids.extend(
        crate::media::generated_video_target_media_binding_project_ids(db, &[video_id]).await?,
    );
    Ok(project_ids.into_iter().collect())
}
