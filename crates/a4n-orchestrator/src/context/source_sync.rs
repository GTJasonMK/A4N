//! Source Sync 领域私有 owner。
//!
//! 持有 `DbHandle`，封装源文本同步预览、冲突决策落库、运行查询与派生项目脱离的数据库行为。
//! Galgame 场景再生成等跨域编排仍在 `source_sync/` 模块，但 DB 只经本 owner。

use a4n_core::error::AppError;
use a4n_core::models::{project::novel_project, source_sync::source_sync_run};
use a4n_db::DbHandle;
use a4n_db::source_sync::SourceSyncPreviewRecord;

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

    fn connection(&self) -> &sea_orm::DatabaseConnection {
        self.database.connection()
    }

    async fn create_source_sync_preview_records(
        &self,
        derived_project_id: i64,
    ) -> Result<SourceSyncPreviewRecord, AppError> {
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
            .map_err(AppError::Database)
    }

    async fn detach_derived_project_record(
        &self,
        project: novel_project::Model,
    ) -> Result<novel_project::Model, AppError> {
        a4n_db::source_sync::detach_derived_project_record(self.connection(), project)
            .await
            .map_err(source_sync_db_error)
    }
}

impl OrchestratorContext {
    pub async fn source_sync_preview_context_for_user_validated(
        &self,
        user_id: i64,
        derived_project_id: i64,
    ) -> Result<crate::project::ProjectFamilyContext, AppError> {
        self.derived_project_family_editor_context_validated(user_id, derived_project_id)
            .await
    }

    pub async fn preview_source_sync_for_user_validated(
        &self,
        user_id: i64,
        derived_project_id: i64,
    ) -> Result<crate::source_sync::SourceSyncPlan, AppError> {
        self.derived_project_family_editor_context_validated(user_id, derived_project_id)
            .await?;
        self.preview_source_sync(derived_project_id).await
    }

    pub async fn preview_source_sync(
        &self,
        derived_project_id: i64,
    ) -> Result<crate::source_sync::SourceSyncPlan, AppError> {
        let preview = self
            .source_sync
            .create_source_sync_preview_records(derived_project_id)
            .await?;
        Ok(crate::source_sync::SourceSyncPlan {
            run: preview.run,
            conflicts: preview.conflicts,
        })
    }

    pub async fn source_sync_run_by_id(
        &self,
        sync_run_id: i64,
    ) -> Result<Option<source_sync_run::Model>, AppError> {
        self.source_sync.source_sync_run_by_id(sync_run_id).await
    }

    pub async fn source_sync_run_for_user_validated(
        &self,
        user_id: i64,
        sync_run_id: i64,
    ) -> Result<source_sync_run::Model, AppError> {
        let run = self
            .source_sync_run_by_id(sync_run_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("源同步运行不存在: {sync_run_id}")))?;
        self.verify_project_permission(
            run.derived_project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.verify_project_permission(
            run.root_project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        Ok(run)
    }

    pub async fn run_source_sync_apply_for_user_validated(
        &self,
        user_id: i64,
        run: source_sync_run::Model,
        decisions: Vec<crate::source_sync::SourceSyncDecisionInput>,
        galgame_prompt_renderer: Option<&dyn a4n_prompt::rendering::PromptRenderer>,
    ) -> Result<source_sync_run::Model, AppError> {
        self.verify_project_permission(
            run.derived_project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.verify_project_permission(
            run.root_project_id,
            user_id,
            crate::collaboration::ProjectAccessRole::Editor,
        )
        .await?;
        self.run_source_sync_apply_validated(crate::source_sync::ApplySourceSyncApplicationInput {
            run,
            decisions,
            galgame_prompt_renderer,
        })
        .await
    }

    pub async fn detach_derived_project_record(
        &self,
        project: novel_project::Model,
    ) -> Result<novel_project::Model, AppError> {
        self.source_sync
            .detach_derived_project_record(project)
            .await
    }

    pub async fn detach_derived_project_for_user_validated(
        &self,
        user_id: i64,
        derived_project_id: i64,
    ) -> Result<novel_project::Model, AppError> {
        let family = self
            .derived_project_family_editor_context_validated(user_id, derived_project_id)
            .await?;
        self.detach_derived_project_record(family.project).await
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
    ) -> Result<a4n_core::models::generation::workflow_run::Model, AppError> {
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
