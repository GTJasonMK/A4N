//! Source Sync 领域私有 owner。
//!
//! 持有 `DbHandle`，封装源文本同步预览、冲突决策落库、运行查询与派生项目脱离的数据库行为。
//! Galgame 场景再生成等跨域编排仍在 `source_sync/` 模块，但 DB 只经本 owner。

use a4n_db::DbHandle;
use a4n_db::source_sync::SourceSyncPlanRecord;
use a4n_db_models::models::source_sync::source_sync_run;
use a4n_error::AppError;

use super::OrchestratorContext;
use crate::source_sync::source_sync_db_error;

/// Source Sync 领域 application owner。
#[derive(Clone)]
pub(super) struct SourceSyncApplication {
    database: DbHandle,
}

impl SourceSyncApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    fn connection(&self) -> &a4n_db::DbHandle {
        &self.database
    }

    async fn create_source_sync_preview_records(
        &self,
        derived_project_id: i64,
    ) -> Result<SourceSyncPlanRecord, AppError> {
        a4n_db::source_sync::create_source_sync_preview_records(
            self.connection(),
            derived_project_id,
        )
        .await
        .map_err(source_sync_db_error)
    }

    async fn source_sync_run_by_id(
        &self,
        sync_run_id: i64,
    ) -> Result<Option<source_sync_run::Model>, AppError> {
        a4n_db::source_sync::source_sync_run_by_id(self.connection(), sync_run_id)
            .await
            .map_err(AppError::database)
    }

    async fn detach_derived_project_record(&self, derived_project_id: i64) -> Result<(), AppError> {
        a4n_db::source_sync::detach_derived_project_record(self.connection(), derived_project_id)
            .await
            .map_err(source_sync_db_error)
    }
}

impl OrchestratorContext {
    pub(crate) async fn source_sync_preview_context_for_user_validated(
        &self,
        user_id: i64,
        derived_project_id: i64,
    ) -> Result<crate::project::ProjectFamilyContext, AppError> {
        self.derived_project_family_editor_context_validated(user_id, derived_project_id)
            .await
    }

    pub(crate) async fn preview_source_sync(
        &self,
        derived_project_id: i64,
    ) -> Result<SourceSyncPlanRecord, AppError> {
        self.source_sync
            .create_source_sync_preview_records(derived_project_id)
            .await
    }

    pub(crate) async fn source_sync_run_by_id(
        &self,
        sync_run_id: i64,
    ) -> Result<Option<source_sync_run::Model>, AppError> {
        self.source_sync.source_sync_run_by_id(sync_run_id).await
    }

    pub(crate) async fn source_sync_run_for_user_validated(
        &self,
        user_id: i64,
        sync_run_id: i64,
    ) -> Result<source_sync_run::Model, AppError> {
        let run = self
            .source_sync_run_by_id(sync_run_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("源同步运行不存在: {sync_run_id}")))?;
        let (derived_result, root_result) = futures::join!(
            self.verify_project_permission(
                run.derived_project_id,
                user_id,
                a4n_domain::collaboration::ProjectAccessRole::Editor,
            ),
            self.verify_project_permission(
                run.root_project_id,
                user_id,
                a4n_domain::collaboration::ProjectAccessRole::Editor,
            ),
        );
        derived_result?;
        root_result?;
        Ok(run)
    }

    pub(crate) async fn detach_derived_project_for_user_validated(
        &self,
        user_id: i64,
        derived_project_id: i64,
    ) -> Result<(), AppError> {
        let family = self
            .derived_project_family_editor_context_validated(user_id, derived_project_id)
            .await?;
        self.source_sync
            .detach_derived_project_record(family.project.id)
            .await
    }

    /// 源文本同步 apply：校验后创建 workflow_run（RUNNING）。
    pub(crate) async fn begin_source_sync_apply_workflow_validated(
        &self,
        sync_run_id: i64,
        derived_project_id: i64,
        root_project_id: i64,
        from_snapshot_id: i64,
        to_snapshot_id: i64,
        decisions: &serde_json::Value,
    ) -> Result<a4n_db::workflow::WorkflowRunSummary, AppError> {
        let input = crate::source_sync::source_sync_apply_workflow_run_input(
            sync_run_id,
            derived_project_id,
            root_project_id,
            from_snapshot_id,
            to_snapshot_id,
            decisions,
        )
        .map_err(AppError::InvalidArgument)?;
        self.begin_workflow_run_validated(input).await
    }
}
