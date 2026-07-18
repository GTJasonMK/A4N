use a4n_core::error::AppError;

use crate::OrchestratorContext;

use super::contracts::{ChapterShootingScriptSourceInput, ResolvedChapterShootingScript};

impl OrchestratorContext {
    pub async fn current_chapter_shooting_script(
        &self,
        input: ChapterShootingScriptSourceInput,
    ) -> Result<Option<ResolvedChapterShootingScript>, AppError> {
        let (source, shooting_script) = self.load_chapter_shooting_script_source(&input).await?;
        Ok(
            shooting_script.map(|shooting_script| ResolvedChapterShootingScript {
                source,
                shooting_script,
            }),
        )
    }

    pub async fn require_current_chapter_shooting_script(
        &self,
        input: ChapterShootingScriptSourceInput,
    ) -> Result<ResolvedChapterShootingScript, AppError> {
        let project_id = input.project_id;
        let source_chapter_id = input.source_chapter_id;
        self.current_chapter_shooting_script(input)
            .await?
            .ok_or_else(|| {
                AppError::NotFound(format!(
                    "项目 {project_id} 的章节 {source_chapter_id} 尚未生成当前来源版本的结构化拍摄剧本；请先在正文页的“剧本改编”标签中生成"
                ))
            })
    }

    pub async fn next_chapter_shooting_script_revision(
        &self,
        input: ChapterShootingScriptSourceInput,
    ) -> Result<i32, AppError> {
        let (_, existing) = self.load_chapter_shooting_script_source(&input).await?;
        match existing {
            Some(existing) => {
                existing
                    .record
                    .revision
                    .checked_add(1)
                    .ok_or_else(|| AppError::InvalidState {
                        current: format!(
                            "chapter_shooting_script_revision_exhausted:id={},revision={}",
                            existing.record.id, existing.record.revision
                        ),
                        operation: "创建章节拍摄剧本工作流".to_string(),
                    })
            }
            None => Ok(1),
        }
    }
}
