use super::comic::comic_project_progress_by_project;
use super::freeform::canvas_project_progress_by_project;
use super::galgame::galgame_project_progress_by_project;
use super::text::text_project_progress;
use super::video::video_project_progress_by_project;
use super::{ProjectProgressQuery, ProjectProgressReadError, ProjectProgressSummary};
use crate::project::validate_positive_i64;
use a4n_domain::project::ProjectKind;
use std::collections::{HashMap, HashSet};

pub async fn project_progress_summary(
    db: &impl crate::DbExecutor,
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
    db: &impl crate::DbExecutor,
    queries: &[ProjectProgressQuery],
) -> Result<HashMap<i64, ProjectProgressSummary>, ProjectProgressReadError> {
    let mut progress_by_project_id = HashMap::with_capacity(queries.len());
    let mut comic_project_ids = Vec::new();
    let mut video_project_ids = Vec::new();
    let mut canvas_project_ids = Vec::new();
    let mut galgame_project_ids = Vec::new();
    let mut project_ids = HashSet::with_capacity(queries.len());

    for query in queries {
        validate_positive_i64(query.project_id, "project_id")
            .map_err(ProjectProgressReadError::InvalidArgument)?;
        if !project_ids.insert(query.project_id) {
            return Err(ProjectProgressReadError::InvalidArgument(format!(
                "项目进度查询包含重复 project_id: {}",
                query.project_id
            )));
        }
        match query.project_kind {
            ProjectKind::Text => {
                progress_by_project_id.insert(query.project_id, text_project_progress(query));
            }
            ProjectKind::Comic => comic_project_ids.push(query.project_id),
            ProjectKind::Canvas => {
                canvas_project_ids.push(query.project_id);
            }
            ProjectKind::Video => video_project_ids.push(query.project_id),
            ProjectKind::Galgame => galgame_project_ids.push(query.project_id),
        }
    }

    let (comic, video, canvas, galgame) = futures::try_join!(
        comic_project_progress_by_project(db, &comic_project_ids),
        video_project_progress_by_project(db, &video_project_ids),
        canvas_project_progress_by_project(db, &canvas_project_ids),
        galgame_project_progress_by_project(db, &galgame_project_ids),
    )?;
    progress_by_project_id.extend(comic);
    progress_by_project_id.extend(video);
    progress_by_project_id.extend(canvas);
    progress_by_project_id.extend(galgame);
    Ok(progress_by_project_id)
}
