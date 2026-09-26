//! 拍摄剧本领域私有 owner。
//!
//! 持有 `DbHandle`，封装章节共享拍摄剧本的加载与生成发布落库。
//! LLM 生成编排仍在 `shooting_script/` 模块，但 DB 只经本 owner。

use std::sync::Arc;

use a4n_api_contract::shooting_script::{
    PublishChapterShootingScriptInput, ShootingScriptRecordMutationApplicationPort,
};
use a4n_db::DbHandle;
use a4n_db::chapter_visual_plan::ParsedChapterVisualPlan;
use a4n_db::project::ResolvedChapterSource;
use a4n_db::shooting_script::{ChapterVisualPlanIdentity, ParsedChapterShootingScript};
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
    publication: Arc<dyn ShootingScriptRecordMutationApplicationPort>,
}

impl ShootingScriptApplication {
    pub(super) fn new(
        database: DbHandle,
        publication: Arc<dyn ShootingScriptRecordMutationApplicationPort>,
    ) -> Self {
        Self {
            database,
            publication,
        }
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
        user_id: i64,
        source: ResolvedChapterSource,
        existing: Option<ParsedChapterShootingScript>,
        input_fingerprint_sha256: String,
        visual_plan: ChapterVisualPlanIdentity,
        script: ChapterShootingScriptV1,
        workflow_run_id: i64,
    ) -> Result<ResolvedChapterShootingScript, AppError> {
        let visual_plan_id = visual_plan.id;
        self.publication
            .publish(
                user_id,
                PublishChapterShootingScriptInput {
                    project_id: source.requested_project_id,
                    source_chapter_id: source.source_chapter_id,
                    input_fingerprint_sha256,
                    visual_plan_id,
                    visual_plan_version: visual_plan.version,
                    visual_plan_sha256: visual_plan.sha256,
                    script,
                    workflow_run_id,
                    expected_revision: existing.map(|record| record.record.revision),
                },
            )
            .await?;
        let (resolved_source, existing) = self
            .load_chapter_shooting_script_source_record(&ChapterShootingScriptSourceInput {
                project_id: source.requested_project_id,
                source_chapter_id: source.source_chapter_id,
                visual_plan_id: Some(visual_plan_id),
            })
            .await?;
        let shooting_script = self
            .load_current_chapter_shooting_script_for_source(
                resolved_source,
                existing,
                Some(visual_plan_id),
            )
            .await?;
        let shooting_script = shooting_script.ok_or_else(|| AppError::InvalidState {
            current: "Novel Planning worker 已发布拍摄剧本，但 Host 无法读取当前版本".into(),
            operation: "读取已发布章节拍摄剧本".into(),
        })?;
        Ok(ResolvedChapterShootingScript {
            source,
            shooting_script,
        })
    }
}

impl OrchestratorContext {
    pub(crate) async fn current_chapter_shooting_script_unit_progress(
        &self,
        source: ChapterShootingScriptSourceInput,
    ) -> Result<Option<a4n_domain::shooting_script::ChapterShootingScriptUnitProgressView>, AppError>
    {
        let (source, _) = self
            .load_chapter_shooting_script_source_record(&source)
            .await?;
        a4n_db::shooting_script::chapter_shooting_script_progress_for_source(
            self.shooting_script.connection(),
            &source,
        )
        .await
        .map_err(shooting_script_db_error)
    }

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
        user_id: i64,
        source: ResolvedChapterSource,
        existing: Option<ParsedChapterShootingScript>,
        input_fingerprint_sha256: String,
        visual_plan: ChapterVisualPlanIdentity,
        script: ChapterShootingScriptV1,
        workflow_run_id: i64,
    ) -> Result<ResolvedChapterShootingScript, AppError> {
        self.shooting_script
            .publish_chapter_shooting_script(
                user_id,
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
            "项目 {} 的第 {} 章（章节 ID {}）未找到与当前来源版本匹配的拍摄剧本；请先在文本工作台的“拍摄剧本”中生成场景，派生项目还需同步来源",
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
pub(crate) fn test_shooting_script_publication(
    database: DbHandle,
) -> Arc<dyn ShootingScriptRecordMutationApplicationPort> {
    Arc::new(TestShootingScriptPublication { database })
}

#[cfg(test)]
#[derive(Clone)]
struct TestShootingScriptPublication {
    database: DbHandle,
}

#[cfg(test)]
#[async_trait::async_trait]
impl ShootingScriptRecordMutationApplicationPort for TestShootingScriptPublication {
    async fn edit_progress(
        &self,
        _user_id: i64,
        _input: a4n_api_contract::shooting_script::EditChapterShootingScriptProgressInput,
    ) -> Result<a4n_api_contract::shooting_script::EditedChapterShootingScriptProgressView, AppError>
    {
        Err(AppError::InvalidState {
            current: "测试 publication 只支持生成发布".into(),
            operation: "编辑测试拍摄剧本进度".into(),
        })
    }

    async fn publish(
        &self,
        _user_id: i64,
        input: PublishChapterShootingScriptInput,
    ) -> Result<a4n_api_contract::shooting_script::ChapterShootingScriptView, AppError> {
        let source = a4n_db::project::resolve_chapter_source(
            self.database.connection(),
            input.project_id,
            input.source_chapter_id,
        )
        .await
        .map_err(resolve_source_error)?;
        let visual_plan = ChapterVisualPlanIdentity {
            id: input.visual_plan_id,
            version: input.visual_plan_version,
            sha256: input.visual_plan_sha256,
        };
        let record = match input.expected_revision {
            Some(expected_revision) => {
                let existing = a4n_db::shooting_script::chapter_shooting_script_for_source(
                    self.database.connection(),
                    &source,
                )
                .await
                .map_err(shooting_script_db_error)?
                .ok_or_else(|| AppError::NotFound("测试拍摄剧本不存在".into()))?;
                a4n_db::shooting_script::update_chapter_shooting_script(
                    self.database.connection(),
                    a4n_db::shooting_script::UpdateChapterShootingScriptInput {
                        record_id: existing.record.id,
                        expected_revision,
                        input_fingerprint_sha256: input.input_fingerprint_sha256,
                        visual_plan,
                        script: input.script,
                        workflow_run_id: Some(input.workflow_run_id),
                    },
                )
                .await
                .map_err(shooting_script_db_error)?
            }
            None => a4n_db::shooting_script::insert_chapter_shooting_script(
                self.database.connection(),
                a4n_db::shooting_script::InsertChapterShootingScriptInput {
                    source,
                    input_fingerprint_sha256: input.input_fingerprint_sha256,
                    visual_plan,
                    script: input.script,
                    workflow_run_id: Some(input.workflow_run_id),
                },
            )
            .await
            .map_err(shooting_script_db_error)?,
        };
        let plan_sync = a4n_db::shooting_script::chapter_shooting_script_plan_sync(
            self.database.connection(),
            &record,
        )
        .await
        .map_err(shooting_script_db_error)?;
        Ok(
            a4n_api_contract::shooting_script::ChapterShootingScriptView {
                record: a4n_api_contract::shooting_script::ChapterShootingScriptRecordSummary {
                    id: record.record.id,
                    revision: record.record.revision,
                    visual_plan_id: record.record.visual_plan_id,
                    visual_plan_version: record.record.visual_plan_version,
                    visual_plan_sha256: record.record.visual_plan_sha256,
                },
                script: record.script,
                plan_sync,
            },
        )
    }

    async fn edit(
        &self,
        _user_id: i64,
        _input: a4n_api_contract::shooting_script::EditChapterShootingScriptInput,
    ) -> Result<a4n_api_contract::shooting_script::EditedChapterShootingScriptView, AppError> {
        Err(AppError::InvalidState {
            current: "测试 publication 只支持生成发布".into(),
            operation: "编辑测试拍摄剧本".into(),
        })
    }

    async fn delete(
        &self,
        _user_id: i64,
        _input: a4n_api_contract::shooting_script::DeleteChapterShootingScriptInput,
    ) -> Result<(), AppError> {
        Err(AppError::InvalidState {
            current: "测试 publication 只支持生成发布".into(),
            operation: "删除测试拍摄剧本".into(),
        })
    }
}
