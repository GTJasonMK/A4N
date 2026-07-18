use a4n_db::project::ChapterGenerationReadiness;

use super::super::{chapter, project_status_read_error_to_app_error};
use crate::OrchestratorContext;

impl OrchestratorContext {
    pub async fn validate_chapter_generation_readiness(
        &self,
        project_id: i64,
        chapter_number: i32,
    ) -> Result<(), String> {
        validate_chapter_generation_number(chapter_number)?;

        let readiness = a4n_db::project::chapter_generation_readiness(
            self.db.connection(),
            project_id,
            chapter_number,
        )
        .await
        .map_err(project_status_read_error_to_app_error)
        .map_err(|error| error.to_string())?;

        validate_chapter_generation_readiness_record(chapter_number, &readiness)
    }
}

pub(super) fn validate_chapter_generation_number(chapter_number: i32) -> Result<(), String> {
    if chapter_number <= 0 {
        return Err("章节序号必须大于 0".to_string());
    }
    Ok(())
}

pub(super) fn validate_chapter_generation_readiness_record(
    chapter_number: i32,
    readiness: &ChapterGenerationReadiness,
) -> Result<(), String> {
    if !readiness.has_blueprint {
        return Err("请先完成故事蓝图后再生成正文".to_string());
    }

    if !readiness.has_chapter_outline {
        return Err(format!(
            "第{chapter_number}章章节大纲未找到，请先生成该章章纲后再生成正文"
        ));
    }

    if readiness
        .existing_chapter
        .as_ref()
        .is_some_and(chapter::Model::has_available_content)
        || chapter_number == 1
    {
        return Ok(());
    }

    let Some(previous) = readiness.previous_chapter.as_ref() else {
        return Err(format!(
            "请先完成第{}章正文后，再生成第{}章",
            chapter_number - 1,
            chapter_number
        ));
    };

    if !previous.has_available_content() {
        return Err(format!(
            "第{}章尚无可用正文，请先生成或导入正文后再生成第{}章",
            chapter_number - 1,
            chapter_number
        ));
    }

    Ok(())
}
