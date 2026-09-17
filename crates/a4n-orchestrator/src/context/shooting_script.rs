//! 拍摄剧本领域私有 owner。
//!
//! 持有 `DbHandle`，封装章节共享拍摄剧本的加载与生成发布落库。
//! LLM 生成编排仍在 `shooting_script/` 模块，但 DB 只经本 owner。

use a4n_db::DbHandle;
use a4n_db::chapter_visual_plan::ParsedChapterVisualPlan;
use a4n_db::project::ResolvedChapterSource;
use a4n_db::shooting_script::{
    ChapterVisualPlanIdentity, InsertChapterShootingScriptInput, ParsedChapterShootingScript,
    UpdateChapterShootingScriptInput,
};
use a4n_domain::shooting_script::ChapterShootingScriptV1;
use a4n_error::AppError;

use super::OrchestratorContext;
use crate::chapter_visual_plan::visual_plan_db_error;
use crate::shooting_script::errors::{resolve_source_error, shooting_script_db_error};
use crate::shooting_script::{ChapterShootingScriptSourceInput, ResolvedChapterShootingScript};

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

    async fn load_chapter_shooting_script_source_record(
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

    async fn load_current_chapter_shooting_script_for_source(
        &self,
        source: ResolvedChapterSource,
        existing: Option<ParsedChapterShootingScript>,
        requested_visual_plan_id: Option<i64>,
    ) -> Result<Option<ParsedChapterShootingScript>, AppError> {
        let confirmed = a4n_db::chapter_visual_plan::confirmed_chapter_visual_plan_for_source(
            self.connection(),
            &source,
        )
        .await
        .map_err(visual_plan_db_error)?;
        let Some(confirmed) = confirmed else {
            return Ok(None);
        };
        if requested_visual_plan_id.is_some_and(|requested_id| requested_id != confirmed.record.id)
        {
            return Err(AppError::InvalidArgument(format!(
                "visual_plan_id 不是当前正文的 confirmed 方案: requested={requested_visual_plan_id:?}, current={}",
                confirmed.record.id
            )));
        }
        Ok(current_script_for_confirmed_visual_plan(
            existing, &confirmed,
        ))
    }

    async fn load_current_chapter_shooting_scripts_for_sources(
        &self,
        sources: &[ResolvedChapterSource],
    ) -> Result<Vec<(ResolvedChapterSource, Option<ParsedChapterShootingScript>)>, AppError> {
        let existing = a4n_db::shooting_script::chapter_shooting_scripts_for_sources(
            self.connection(),
            sources,
        )
        .await
        .map_err(shooting_script_db_error)?;
        let visual_plans = a4n_db::chapter_visual_plan::chapter_visual_plans_for_sources(
            self.connection(),
            sources,
        )
        .await
        .map_err(visual_plan_db_error)?;
        Ok(sources
            .iter()
            .cloned()
            .zip(existing)
            .zip(visual_plans)
            .map(|((source, existing), plans)| {
                (
                    source,
                    plans.confirmed.as_ref().and_then(|confirmed| {
                        current_script_for_confirmed_visual_plan(existing, confirmed)
                    }),
                )
            })
            .collect())
    }

    async fn publish_chapter_shooting_script(
        &self,
        source: ResolvedChapterSource,
        existing: Option<ParsedChapterShootingScript>,
        input_fingerprint_sha256: String,
        visual_plan: ChapterVisualPlanIdentity,
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
                    visual_plan,
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
                    visual_plan,
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
}

impl OrchestratorContext {
    pub(crate) async fn load_chapter_shooting_script_source_record(
        &self,
        input: &ChapterShootingScriptSourceInput,
    ) -> Result<(ResolvedChapterSource, Option<ParsedChapterShootingScript>), AppError> {
        self.shooting_script
            .load_chapter_shooting_script_source_record(input)
            .await
    }

    pub(crate) async fn load_current_chapter_shooting_script_source(
        &self,
        input: &ChapterShootingScriptSourceInput,
    ) -> Result<(ResolvedChapterSource, Option<ParsedChapterShootingScript>), AppError> {
        let (source, existing) = self
            .load_chapter_shooting_script_source_record(input)
            .await?;
        let current = self
            .shooting_script
            .load_current_chapter_shooting_script_for_source(
                source.clone(),
                existing,
                input.visual_plan_id,
            )
            .await?;
        Ok((source, current))
    }

    pub(crate) async fn require_current_chapter_shooting_scripts_for_resolved_sources(
        &self,
        sources: Vec<ResolvedChapterSource>,
    ) -> Result<Vec<ResolvedChapterShootingScript>, AppError> {
        self.shooting_script
            .load_current_chapter_shooting_scripts_for_sources(&sources)
            .await?
            .into_iter()
            .map(|(source, current)| require_current_chapter_shooting_script(source, current))
            .collect()
    }

    pub(crate) async fn publish_chapter_shooting_script(
        &self,
        source: ResolvedChapterSource,
        existing: Option<ParsedChapterShootingScript>,
        input_fingerprint_sha256: String,
        visual_plan: ChapterVisualPlanIdentity,
        script: ChapterShootingScriptV1,
        workflow_run_id: i64,
    ) -> Result<ResolvedChapterShootingScript, AppError> {
        self.shooting_script
            .publish_chapter_shooting_script(
                source,
                existing,
                input_fingerprint_sha256,
                visual_plan,
                script,
                workflow_run_id,
            )
            .await
    }

    pub(crate) async fn current_chapter_shooting_script(
        &self,
        input: ChapterShootingScriptSourceInput,
    ) -> Result<Option<ResolvedChapterShootingScript>, AppError> {
        let (source, shooting_script) = self
            .load_current_chapter_shooting_script_source(&input)
            .await?;
        Ok(
            shooting_script.map(|shooting_script| ResolvedChapterShootingScript {
                source,
                shooting_script,
            }),
        )
    }

    pub(crate) async fn require_current_chapter_shooting_script(
        &self,
        input: ChapterShootingScriptSourceInput,
    ) -> Result<ResolvedChapterShootingScript, AppError> {
        let (source, shooting_script) = self
            .load_current_chapter_shooting_script_source(&input)
            .await?;
        require_current_chapter_shooting_script(source, shooting_script)
    }
}

fn require_current_chapter_shooting_script(
    source: ResolvedChapterSource,
    shooting_script: Option<ParsedChapterShootingScript>,
) -> Result<ResolvedChapterShootingScript, AppError> {
    match shooting_script {
        Some(shooting_script) => Ok(ResolvedChapterShootingScript {
            source,
            shooting_script,
        }),
        None => Err(AppError::NotFound(format!(
            "项目 {} 的第 {} 章（章节 ID {}）未找到与当前正文版本匹配的结构化拍摄剧本；请先在正文页的“剧本改编”标签中生成",
            source.requested_project_id, source.chapter_number, source.source_chapter_id
        ))),
    }
}

fn current_script_for_confirmed_visual_plan(
    existing: Option<ParsedChapterShootingScript>,
    confirmed: &ParsedChapterVisualPlan,
) -> Option<ParsedChapterShootingScript> {
    existing.filter(|script| {
        script.record.visual_plan_id == Some(confirmed.record.id)
            && script.record.visual_plan_version == Some(confirmed.record.plan_version)
            && script.record.visual_plan_sha256.as_deref()
                == Some(confirmed.record.plan_sha256.as_str())
    })
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
