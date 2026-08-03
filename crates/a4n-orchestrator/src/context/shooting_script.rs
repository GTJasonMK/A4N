//! 拍摄剧本领域私有 owner。
//!
//! 持有 `DbHandle`，封装章节共享拍摄剧本的加载、发布与编辑落库。
//! LLM 生成编排仍在 `shooting_script/` 模块，但 DB 只经本 owner。

use a4n_core::{error::AppError, shooting_script::ChapterShootingScriptV1};
use a4n_db::DbHandle;
use a4n_db::project::ResolvedChapterSource;
use a4n_db::shooting_script::{
    EditStoredChapterShootingScriptInput, InsertChapterShootingScriptInput,
    ParsedChapterShootingScript, UpdateChapterShootingScriptInput,
};

use super::OrchestratorContext;
use crate::shooting_script::errors::{resolve_source_error, shooting_script_db_error};
use crate::shooting_script::{
    ChapterShootingScriptSourceInput, EditChapterShootingScriptInput, EditedChapterShootingScript,
    ResolvedChapterShootingScript,
};

/// 拍摄剧本领域 application owner。
#[derive(Clone)]
pub(super) struct ShootingScriptApplication {
    database: DbHandle,
}

impl ShootingScriptApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    fn connection(&self) -> &sea_orm::DatabaseConnection {
        self.database.connection()
    }

    async fn load_chapter_shooting_script_source(
        &self,
        input: &ChapterShootingScriptSourceInput,
    ) -> Result<(ResolvedChapterSource, Option<ParsedChapterShootingScript>), AppError> {
        let source = a4n_db::project::resolve_chapter_source(
            self.connection(),
            input.project_id,
            input.source_chapter_id,
        )
        .await
        .map_err(resolve_source_error)?;
        let existing =
            a4n_db::shooting_script::chapter_shooting_script_for_source(self.connection(), &source)
                .await
                .map_err(shooting_script_db_error)?;
        Ok((source, existing))
    }

    async fn publish_chapter_shooting_script(
        &self,
        source: ResolvedChapterSource,
        existing: Option<ParsedChapterShootingScript>,
        input_fingerprint_sha256: String,
        script: ChapterShootingScriptV1,
        workflow_run_id: i64,
    ) -> Result<ResolvedChapterShootingScript, AppError> {
        let shooting_script = match existing {
            Some(existing) => a4n_db::shooting_script::update_chapter_shooting_script(
                self.connection(),
                UpdateChapterShootingScriptInput {
                    record_id: existing.record.id,
                    expected_revision: existing.record.revision,
                    input_fingerprint_sha256,
                    script,
                    workflow_run_id: Some(workflow_run_id),
                },
            )
            .await
            .map_err(shooting_script_db_error)?,
            None => a4n_db::shooting_script::insert_chapter_shooting_script(
                self.connection(),
                InsertChapterShootingScriptInput {
                    source: source.clone(),
                    input_fingerprint_sha256,
                    script,
                    workflow_run_id: Some(workflow_run_id),
                },
            )
            .await
            .map_err(shooting_script_db_error)?,
        };
        Ok(ResolvedChapterShootingScript {
            source,
            shooting_script,
        })
    }

    async fn edit_chapter_shooting_script(
        &self,
        input: EditChapterShootingScriptInput,
    ) -> Result<EditedChapterShootingScript, AppError> {
        input
            .script
            .validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        let outcome = a4n_db::shooting_script::edit_chapter_shooting_script_and_invalidate(
            self.connection(),
            EditStoredChapterShootingScriptInput {
                record_id: input.record_id,
                expected_revision: input.expected_revision,
                script: input.script,
            },
        )
        .await
        .map_err(shooting_script_db_error)?;
        Ok(EditedChapterShootingScript {
            shooting_script: outcome.shooting_script,
            invalidated_project_ids: outcome.invalidated_project_ids,
        })
    }
}

impl OrchestratorContext {
    pub(crate) async fn load_chapter_shooting_script_source(
        &self,
        input: &ChapterShootingScriptSourceInput,
    ) -> Result<(ResolvedChapterSource, Option<ParsedChapterShootingScript>), AppError> {
        self.shooting_script
            .load_chapter_shooting_script_source(input)
            .await
    }

    pub(crate) async fn publish_chapter_shooting_script(
        &self,
        source: ResolvedChapterSource,
        existing: Option<ParsedChapterShootingScript>,
        input_fingerprint_sha256: String,
        script: ChapterShootingScriptV1,
        workflow_run_id: i64,
    ) -> Result<ResolvedChapterShootingScript, AppError> {
        self.shooting_script
            .publish_chapter_shooting_script(
                source,
                existing,
                input_fingerprint_sha256,
                script,
                workflow_run_id,
            )
            .await
    }

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

    pub async fn current_chapter_shooting_script_for_user_validated(
        &self,
        user_id: i64,
        input: ChapterShootingScriptSourceInput,
    ) -> Result<Option<ResolvedChapterShootingScript>, AppError> {
        self.verify_project_or_derived_family_access_validated(user_id, input.project_id)
            .await?;
        self.current_chapter_shooting_script(input).await
    }

    pub async fn require_current_chapter_shooting_script(
        &self,
        input: ChapterShootingScriptSourceInput,
    ) -> Result<ResolvedChapterShootingScript, AppError> {
        let project_id = input.project_id;
        let (source, shooting_script) = self.load_chapter_shooting_script_source(&input).await?;
        match shooting_script {
            Some(shooting_script) => Ok(ResolvedChapterShootingScript {
                source,
                shooting_script,
            }),
            None => Err(AppError::NotFound(format!(
                "项目 {project_id} 的第 {} 章（章节 ID {}）未找到与当前正文版本匹配的结构化拍摄剧本；请先在正文页的“剧本改编”标签中生成",
                source.chapter_number, source.source_chapter_id
            ))),
        }
    }

    pub(crate) async fn next_chapter_shooting_script_revision(
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

    pub async fn edit_current_chapter_shooting_script_validated(
        &self,
        source: ChapterShootingScriptSourceInput,
        input: EditChapterShootingScriptInput,
    ) -> Result<EditedChapterShootingScript, AppError> {
        let current = self.require_current_chapter_shooting_script(source).await?;
        ensure_current_shooting_script_record_id(
            current.shooting_script.record.id,
            input.record_id,
        )?;
        self.shooting_script
            .edit_chapter_shooting_script(input)
            .await
    }

    pub async fn edit_current_chapter_shooting_script_for_user_validated(
        &self,
        user_id: i64,
        source: ChapterShootingScriptSourceInput,
        input: EditChapterShootingScriptInput,
    ) -> Result<EditedChapterShootingScript, AppError> {
        self.verify_project_or_derived_family_editor_validated(user_id, source.project_id)
            .await?;
        self.edit_current_chapter_shooting_script_validated(source, input)
            .await
    }

    /// 章节分镜脚本：校验后创建 workflow_run（RUNNING）。
    pub(crate) async fn begin_chapter_shooting_script_workflow_validated(
        &self,
        project_id: i64,
        source_chapter_id: i64,
        target_revision: i32,
        active_task_id: Option<&str>,
    ) -> Result<a4n_core::models::generation::workflow_run::Model, AppError> {
        let input = crate::shooting_script::chapter_shooting_script_workflow_run_input(
            project_id,
            source_chapter_id,
            target_revision,
            active_task_id,
        )
        .map_err(AppError::InvalidArgument)?;
        self.begin_or_resume_workflow_run_validated(input).await
    }
}

fn ensure_current_shooting_script_record_id(
    expected_record_id: i64,
    actual_record_id: i64,
) -> Result<(), AppError> {
    if expected_record_id != actual_record_id {
        return Err(AppError::InvalidArgument(format!(
            "拍摄剧本与当前章节来源不匹配: expected_record_id={expected_record_id}, actual_record_id={actual_record_id}"
        )));
    }
    Ok(())
}

#[cfg(test)]
mod current_record_tests {
    use super::ensure_current_shooting_script_record_id;

    #[test]
    fn rejects_non_current_record() {
        let error = ensure_current_shooting_script_record_id(7, 8)
            .expect_err("non-current record must be rejected");

        assert!(error.to_string().contains("expected_record_id=7"));
        assert!(error.to_string().contains("actual_record_id=8"));
    }
}
