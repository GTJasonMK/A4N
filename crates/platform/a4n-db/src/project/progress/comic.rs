use super::shared::{
    PipelineProjectState, ProgressCounts, latest_pipeline_states_by_project, progress_count_detail,
    project_progress,
};
use super::{ProjectProgressReadError, ProjectProgressSummary};
use a4n_db_models::models::chapter::chapter_manga_prompt::{self, MANGA_PAGE_PROMPT_TYPE};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QuerySelect};
use std::collections::HashMap;

const MANGA_PIPELINE_WORKFLOW_KIND: &str = "manga_pipeline";

pub(super) async fn comic_project_progress_by_project(
    db: &DatabaseConnection,
    project_ids: &[i64],
) -> Result<HashMap<i64, ProjectProgressSummary>, ProjectProgressReadError> {
    if project_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let (rows, pipeline) = futures::join!(
        async {
            chapter_manga_prompt::Entity::find()
                .select_only()
                .column(chapter_manga_prompt::Column::Id)
                .column(chapter_manga_prompt::Column::ProjectId)
                .column(chapter_manga_prompt::Column::Status)
                .filter(chapter_manga_prompt::Column::ProjectId.is_in(project_ids.iter().copied()))
                .filter(chapter_manga_prompt::Column::PromptType.eq(MANGA_PAGE_PROMPT_TYPE))
                .into_tuple::<(i64, i64, String)>()
                .all(db)
                .await
                .map_err(|error| {
                    ProjectProgressReadError::database("查询漫画页面提示词失败", error)
                })
        },
        latest_pipeline_states_by_project(
            db,
            project_ids,
            MANGA_PIPELINE_WORKFLOW_KIND,
            "查询漫画最近流水线失败",
        ),
    );
    let rows = rows?;
    let mut counts_by_project: HashMap<i64, ProgressCounts> = HashMap::new();
    for (id, project_id, status) in rows {
        counts_by_project.entry(project_id).or_default().add(
            "chapter_manga_prompt",
            id,
            &status,
        )?;
    }
    let mut pipeline_by_project = pipeline?;
    let mut progress = HashMap::with_capacity(project_ids.len());
    for project_id in project_ids {
        progress.insert(
            *project_id,
            comic_project_progress_from_counts(
                counts_by_project.remove(project_id).unwrap_or_default(),
                pipeline_by_project.remove(project_id).unwrap_or_default(),
            ),
        );
    }
    Ok(progress)
}

fn comic_project_progress_from_counts(
    counts: ProgressCounts,
    pipeline: PipelineProjectState,
) -> ProjectProgressSummary {
    let total = (counts.total() > 0).then_some(counts.total());
    let current = total.map(|_| counts.completed);
    if counts.failed > 0 || pipeline.failed {
        return project_progress(
            "漫画生成失败",
            Some("存在失败的漫画流水线或页面生成记录".to_string()),
        );
    }
    if counts.running > 0 || pipeline.running {
        return project_progress(
            "漫画生成中",
            pipeline
                .message
                .or_else(|| progress_count_detail("已完成页面", current, total)),
        );
    }
    if pipeline.cancelled {
        return project_progress("漫画生成已取消", pipeline.message);
    }
    if total.is_some() && counts.completed >= counts.total() {
        return project_progress(
            "漫画已生成",
            progress_count_detail("已完成页面", current, total),
        );
    }
    if counts.completed > 0 {
        return project_progress(
            "漫画部分完成",
            progress_count_detail("已完成页面", current, total),
        );
    }
    project_progress("漫画制作中", progress_count_detail("页面", current, total))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn comic_project_progress_reports_running_pipeline_before_idle_counts() {
        let mut pipeline = PipelineProjectState {
            running: true,
            ..Default::default()
        };
        pipeline.message = Some("正在生成漫画页".to_string());
        let summary = comic_project_progress_from_counts(ProgressCounts::default(), pipeline);

        assert_eq!(summary.label, "漫画生成中");
        assert_eq!(summary.detail.as_deref(), Some("正在生成漫画页"));
    }
}
