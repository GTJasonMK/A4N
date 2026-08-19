use a4n_core::error::AppError;
use a4n_core::models::chapter::{chapter, chapter_video_prompt};

use crate::OrchestratorContext;

use super::{ChapterVideoSource, ProjectChapterVideoSource, video_db_error};

impl OrchestratorContext {
    pub async fn project_chapter_video_source(
        &self,
        project_id: i64,
        chapter_id: i64,
        operation: &str,
        writable: bool,
    ) -> Result<ProjectChapterVideoSource, AppError> {
        let (project, chapter) = a4n_db::video::project_and_chapter_for_video_source(
            self.database.connection(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(video_db_error)?;

        match project.project_kind.as_str() {
            "text" => {
                if chapter.project_id != project.id {
                    return Err(AppError::InvalidArgument(format!(
                        "{operation}失败：章节不属于文本项目 {project_id}"
                    )));
                }
                if chapter.status == "deleted" {
                    return Err(AppError::InvalidState {
                        current: chapter.status.clone(),
                        operation: operation.to_string(),
                    });
                }
                if writable && chapter.status == "generating" {
                    return Err(AppError::InvalidState {
                        current: chapter.status.clone(),
                        operation: operation.to_string(),
                    });
                }
                let source = resolve_chapter_video_source(&chapter)?;
                Ok(ProjectChapterVideoSource { chapter, source })
            }
            "video" => {
                if project.root_project_id != Some(chapter.project_id) {
                    return Err(AppError::InvalidArgument(format!(
                        "{operation}失败：章节不属于视频项目 {project_id} 绑定的根文本项目"
                    )));
                }
                let snapshot_id =
                    project
                        .source_snapshot_id
                        .ok_or_else(|| AppError::InvalidState {
                            current: "missing_source_snapshot".to_string(),
                            operation: operation.to_string(),
                        })?;
                let snapshot_chapter = a4n_db::video::source_snapshot_chapter_for_video_source(
                    self.database.connection(),
                    snapshot_id,
                    chapter_id,
                )
                .await
                .map_err(video_db_error)?
                .ok_or_else(|| {
                    AppError::NotFound(format!(
                        "{operation}失败：视频项目 {project_id} 的源快照不包含章节 {chapter_id}"
                    ))
                })?;
                let mut chapter = chapter;
                chapter.title = snapshot_chapter.title.clone().or(chapter.title);
                chapter.content = snapshot_chapter.content.clone();
                chapter.summary = snapshot_chapter.summary.clone();
                chapter.analysis_data = snapshot_chapter.analysis_data_json.clone();
                chapter.status = "completed".to_string();
                Ok(ProjectChapterVideoSource {
                    chapter,
                    source: ChapterVideoSource {
                        content: snapshot_chapter.content,
                        summary: snapshot_chapter.summary,
                        analysis: snapshot_chapter.analysis_data_json,
                    },
                })
            }
            other => Err(AppError::InvalidArgument(format!(
                "{operation}失败：视频链路不支持项目类型 {other}: project_id={project_id}"
            ))),
        }
    }
}

fn resolve_chapter_video_source(chapter: &chapter::Model) -> Result<ChapterVideoSource, AppError> {
    if chapter.status == "deleted" {
        return Err(AppError::InvalidState {
            current: chapter.status.clone(),
            operation: "生成或使用视频素材".to_string(),
        });
    }
    Ok(ChapterVideoSource {
        content: chapter.content.clone(),
        summary: chapter.summary.clone(),
        analysis: chapter.analysis_data.clone(),
    })
}

pub(super) fn validate_import_chapter_video_prompt(
    prompt: &chapter_video_prompt::Model,
    project_id: i64,
    chapter_id: i64,
) -> Result<(), String> {
    validate_import_chapter_video_prompt_fields(
        prompt.id,
        prompt.project_id,
        prompt.chapter_id,
        &prompt.prompt_type,
        &prompt.status,
        project_id,
        chapter_id,
    )
}

pub(super) fn validate_import_chapter_video_prompt_fields(
    prompt_id: i64,
    prompt_project_id: i64,
    prompt_chapter_id: i64,
    prompt_type: &str,
    status: &str,
    project_id: i64,
    chapter_id: i64,
) -> Result<(), String> {
    if prompt_project_id != project_id {
        return Err(format!(
            "视频提示词 #{} 不属于当前项目 {}",
            prompt_id, project_id
        ));
    }
    if prompt_chapter_id != chapter_id {
        return Err(format!(
            "视频提示词 #{} 不属于当前章节 {}",
            prompt_id, chapter_id
        ));
    }
    if prompt_type != chapter_video_prompt::PROMPT_TYPE_VIDEO {
        return Err("请选择 video_prompt 类型的提示词导入视频".to_string());
    }
    if status == chapter_video_prompt::STATUS_RUNNING {
        return Err(format!("视频提示词 #{} 正在运行，不能导入视频", prompt_id));
    }
    Ok(())
}
