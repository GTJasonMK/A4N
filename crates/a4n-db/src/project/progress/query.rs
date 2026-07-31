use super::comic::comic_project_progress_by_project;
use super::freeform::freeform_video_project_progress_by_project;
use super::galgame::galgame_project_progress_by_project;
use super::text::{text_project_progress, unknown_project_progress};
use super::video::video_project_progress_by_project;
use super::{ProjectProgressQuery, ProjectProgressReadError, ProjectProgressSummary};
use crate::project::{
    PROJECT_KIND_COMIC, PROJECT_KIND_GALGAME, PROJECT_KIND_TEXT, PROJECT_KIND_VIDEO,
    SOURCE_MODE_FREEFORM, validate_positive_i64,
};
use sea_orm::DatabaseConnection;
use std::collections::HashMap;

pub async fn project_progress_summary(
    db: &DatabaseConnection,
    query: ProjectProgressQuery,
) -> Result<ProjectProgressSummary, ProjectProgressReadError> {
    let mut summaries = project_progress_summaries(db, std::slice::from_ref(&query)).await?;
    summaries
        .remove(&query.project_id)
        .ok_or(ProjectProgressReadError::SummaryMissing {
            project_id: query.project_id,
        })
}

pub async fn project_progress_summaries(
    db: &DatabaseConnection,
    queries: &[ProjectProgressQuery],
) -> Result<HashMap<i64, ProjectProgressSummary>, ProjectProgressReadError> {
    let mut progress_by_project_id = HashMap::with_capacity(queries.len());
    let mut comic_project_ids = Vec::new();
    let mut video_project_ids = Vec::new();
    let mut freeform_video_project_ids = Vec::new();
    let mut galgame_project_ids = Vec::new();

    for query in queries {
        validate_positive_i64(query.project_id, "project_id")
            .map_err(ProjectProgressReadError::InvalidArgument)?;
        match query.project_kind.as_str() {
            PROJECT_KIND_TEXT => {
                progress_by_project_id.insert(query.project_id, text_project_progress(query));
            }
            PROJECT_KIND_COMIC => comic_project_ids.push(query.project_id),
            PROJECT_KIND_VIDEO if query.source_mode == SOURCE_MODE_FREEFORM => {
                freeform_video_project_ids.push(query.project_id);
            }
            PROJECT_KIND_VIDEO => video_project_ids.push(query.project_id),
            PROJECT_KIND_GALGAME => galgame_project_ids.push(query.project_id),
            _ => {
                progress_by_project_id.insert(
                    query.project_id,
                    unknown_project_progress(&query.project_kind),
                );
            }
        }
    }

    progress_by_project_id.extend(comic_project_progress_by_project(db, &comic_project_ids).await?);
    progress_by_project_id.extend(video_project_progress_by_project(db, &video_project_ids).await?);
    progress_by_project_id
        .extend(freeform_video_project_progress_by_project(db, &freeform_video_project_ids).await?);
    progress_by_project_id
        .extend(galgame_project_progress_by_project(db, &galgame_project_ids).await?);
    Ok(progress_by_project_id)
}
