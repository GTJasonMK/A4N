//! 内容优化领域私有 owner。
//!
//! 持有 `DbHandle`，封装优化会话、消息、Turn、补丁集与上下文读模型的数据库行为。
//! 分析/对话/补丁编排仍在 `optimize/` 模块，但 DB 只经本 owner。

use a4n_core::error::AppError;
use a4n_core::models::ai::{
    optimization_patch, optimization_patch_set, optimization_session, optimization_session_message,
    optimization_turn, optimization_turn_event,
};
use a4n_core::models::chapter::chapter;
use a4n_db::DbHandle;
use a4n_db::optimize::{OptimizationChapterPatchWriteInput, OptimizationSessionMessageInsertInput};

use super::OrchestratorContext;
use crate::collaboration::ProjectAccessRole;
use crate::optimize::optimize_db_error;
use crate::optimize::runtime_tools::optimization_runtime_tool_specs;

/// 内容优化领域 application owner。
#[derive(Clone)]
pub(super) struct OptimizeApplication {
    database: DbHandle,
}

impl OptimizeApplication {
    pub(super) fn new(database: DbHandle) -> Self {
        Self { database }
    }

    async fn optimization_sessions(
        &self,
        project_id: i64,
        chapter_id: Option<i64>,
    ) -> Result<Vec<optimization_session::Model>, AppError> {
        a4n_db::optimize::optimization_sessions(self.database.connection(), project_id, chapter_id)
            .await
            .map_err(optimize_db_error)
    }

    async fn optimization_session_by_id(
        &self,
        session_id: i64,
    ) -> Result<Option<optimization_session::Model>, AppError> {
        a4n_db::optimize::optimization_session_by_id(self.database.connection(), session_id)
            .await
            .map_err(optimize_db_error)
    }

    async fn insert_optimization_session_message(
        &self,
        input: OptimizationSessionMessageInsertInput,
    ) -> Result<optimization_session_message::Model, AppError> {
        a4n_db::optimize::insert_optimization_session_message(self.database.connection(), input)
            .await
            .map_err(optimize_db_error)
    }

    async fn delete_optimization_session_with_messages(
        &self,
        session_id: i64,
    ) -> Result<u64, AppError> {
        a4n_db::optimize::delete_optimization_session_with_messages(
            self.database.connection(),
            session_id,
        )
        .await
        .map_err(optimize_db_error)
    }

    async fn optimization_session_messages(
        &self,
        session_id: i64,
    ) -> Result<Vec<optimization_session_message::Model>, AppError> {
        a4n_db::optimize::optimization_session_messages(self.database.connection(), session_id)
            .await
            .map_err(optimize_db_error)
    }

    async fn optimization_chapter_by_id(
        &self,
        chapter_id: i64,
    ) -> Result<Option<chapter::Model>, AppError> {
        a4n_db::optimize::optimization_chapter_by_id(self.database.connection(), chapter_id)
            .await
            .map_err(optimize_db_error)
    }

    async fn write_optimization_chapter_patch_content(
        &self,
        input: OptimizationChapterPatchWriteInput,
    ) -> Result<chapter::Model, AppError> {
        a4n_db::optimize::write_optimization_chapter_patch_content(
            self.database.connection(),
            input,
        )
        .await
        .map_err(optimize_db_error)
    }

    async fn optimization_patch_sets(
        &self,
        session_id: i64,
    ) -> Result<Vec<optimization_patch_set::Model>, AppError> {
        a4n_db::optimize::optimization_patch_sets(self.database.connection(), session_id)
            .await
            .map_err(optimize_db_error)
    }

    async fn optimization_patch_set_patches(
        &self,
        patch_set_id: i64,
    ) -> Result<Vec<optimization_patch::Model>, AppError> {
        a4n_db::optimize::optimization_patch_set_patches(self.database.connection(), patch_set_id)
            .await
            .map_err(optimize_db_error)
    }

    async fn matching_pending_optimization_patch_sets(
        &self,
        session_id: i64,
        base_content_hash: &str,
        pending_status: &str,
    ) -> Result<Vec<optimization_patch_set::Model>, AppError> {
        a4n_db::optimize::matching_pending_optimization_patch_sets(
            self.database.connection(),
            session_id,
            base_content_hash,
            pending_status,
        )
        .await
        .map_err(optimize_db_error)
    }

    async fn pending_optimization_patch_sets(
        &self,
        session_id: i64,
        pending_status: &str,
    ) -> Result<Vec<optimization_patch_set::Model>, AppError> {
        a4n_db::optimize::pending_optimization_patch_sets(
            self.database.connection(),
            session_id,
            pending_status,
        )
        .await
        .map_err(optimize_db_error)
    }

    async fn update_optimization_patch_set_status(
        &self,
        patch_set: optimization_patch_set::Model,
        patch_set_status: &str,
        patch_decision: &str,
        patch_apply_status: &str,
    ) -> Result<(), AppError> {
        a4n_db::optimize::update_optimization_patch_set_status(
            self.database.connection(),
            patch_set,
            patch_set_status,
            patch_decision,
            patch_apply_status,
        )
        .await
        .map_err(optimize_db_error)
    }

    async fn optimization_turns(
        &self,
        session_id: i64,
    ) -> Result<Vec<optimization_turn::Model>, AppError> {
        a4n_db::optimize::optimization_turns(self.database.connection(), session_id)
            .await
            .map_err(optimize_db_error)
    }

    async fn optimization_turn_events(
        &self,
        turn_id: i64,
    ) -> Result<Vec<optimization_turn_event::Model>, AppError> {
        a4n_db::optimize::optimization_turn_events(self.database.connection(), turn_id)
            .await
            .map_err(optimize_db_error)
    }
}

impl OrchestratorContext {
    pub(crate) async fn optimization_session_by_id(
        &self,
        session_id: i64,
    ) -> Result<Option<optimization_session::Model>, AppError> {
        self.optimize.optimization_session_by_id(session_id).await
    }

    pub(crate) async fn insert_optimization_session_message(
        &self,
        input: OptimizationSessionMessageInsertInput,
    ) -> Result<optimization_session_message::Model, AppError> {
        self.optimize
            .insert_optimization_session_message(input)
            .await
    }

    pub(crate) async fn optimization_session_messages(
        &self,
        session_id: i64,
    ) -> Result<Vec<optimization_session_message::Model>, AppError> {
        self.optimize
            .optimization_session_messages(session_id)
            .await
    }

    pub(crate) async fn optimization_chapter_by_id(
        &self,
        chapter_id: i64,
    ) -> Result<Option<chapter::Model>, AppError> {
        self.optimize.optimization_chapter_by_id(chapter_id).await
    }

    pub(crate) async fn write_optimization_chapter_patch_content(
        &self,
        input: OptimizationChapterPatchWriteInput,
    ) -> Result<chapter::Model, AppError> {
        self.optimize
            .write_optimization_chapter_patch_content(input)
            .await
    }

    pub(crate) async fn matching_pending_optimization_patch_sets(
        &self,
        session_id: i64,
        base_content_hash: &str,
        pending_status: &str,
    ) -> Result<Vec<optimization_patch_set::Model>, AppError> {
        self.optimize
            .matching_pending_optimization_patch_sets(session_id, base_content_hash, pending_status)
            .await
    }

    pub(crate) async fn pending_optimization_patch_sets(
        &self,
        session_id: i64,
        pending_status: &str,
    ) -> Result<Vec<optimization_patch_set::Model>, AppError> {
        self.optimize
            .pending_optimization_patch_sets(session_id, pending_status)
            .await
    }

    pub(crate) async fn update_optimization_patch_set_status(
        &self,
        patch_set: optimization_patch_set::Model,
        patch_set_status: &str,
        patch_decision: &str,
        patch_apply_status: &str,
    ) -> Result<(), AppError> {
        self.optimize
            .update_optimization_patch_set_status(
                patch_set,
                patch_set_status,
                patch_decision,
                patch_apply_status,
            )
            .await
    }

    pub(crate) async fn optimization_turns(
        &self,
        session_id: i64,
    ) -> Result<Vec<optimization_turn::Model>, AppError> {
        self.optimize.optimization_turns(session_id).await
    }
}

impl OrchestratorContext {
    pub async fn optimize_sessions_for_user(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: Option<i64>,
    ) -> Result<Vec<optimization_session::Model>, AppError> {
        self.verify_project_permission(project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        self.optimize
            .optimization_sessions(project_id, chapter_id)
            .await
    }

    pub async fn optimize_session_for_user(
        &self,
        user_id: i64,
        session_id: i64,
    ) -> Result<optimization_session::Model, AppError> {
        let session = self.get_session(session_id).await?;
        self.verify_project_permission(session.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        Ok(session)
    }

    pub async fn optimize_turns_for_user(
        &self,
        user_id: i64,
        session_id: i64,
    ) -> Result<
        Vec<(
            optimization_turn::Model,
            Vec<optimization_turn_event::Model>,
        )>,
        AppError,
    > {
        self.optimize_session_for_user(user_id, session_id).await?;
        let turns = self.optimize.optimization_turns(session_id).await?;
        let mut output = Vec::with_capacity(turns.len());
        for turn in turns {
            let events = self.optimize.optimization_turn_events(turn.id).await?;
            output.push((turn, events));
        }
        Ok(output)
    }

    pub async fn optimize_patch_sets_for_user(
        &self,
        user_id: i64,
        session_id: i64,
    ) -> Result<
        Vec<(
            optimization_patch_set::Model,
            Vec<optimization_patch::Model>,
        )>,
        AppError,
    > {
        self.optimize_session_for_user(user_id, session_id).await?;
        let patch_sets = self.optimize.optimization_patch_sets(session_id).await?;
        let mut output = Vec::with_capacity(patch_sets.len());
        for patch_set in patch_sets {
            let patches = self
                .optimize
                .optimization_patch_set_patches(patch_set.id)
                .await?;
            output.push((patch_set, patches));
        }
        Ok(output)
    }

    pub async fn delete_optimize_session_for_user_validated(
        &self,
        user_id: i64,
        session_id: i64,
    ) -> Result<(), AppError> {
        self.optimize_session_for_user(user_id, session_id).await?;
        let rows_affected = self
            .optimize
            .delete_optimization_session_with_messages(session_id)
            .await?;
        if rows_affected != 1 {
            return Err(AppError::NotFound(format!("优化会话不存在: {session_id}")));
        }
        Ok(())
    }

    pub async fn record_optimize_agent_event_for_user_validated(
        &self,
        user_id: i64,
        session_id: i64,
        content: &str,
    ) -> Result<(), AppError> {
        self.optimize_session_for_user(user_id, session_id).await?;
        let content = content.trim();
        if content.is_empty() {
            return Err(AppError::InvalidArgument(
                "Agent 事件内容不能为空".to_string(),
            ));
        }
        self.optimize
            .insert_optimization_session_message(OptimizationSessionMessageInsertInput {
                session_id,
                role: "agent_event".to_string(),
                content: content.to_string(),
                tool_calls: None,
            })
            .await?;
        Ok(())
    }
}

impl OrchestratorContext {
    /// optimize 域编排短名：工具规格。
    pub(crate) fn tool_specs(&self) -> Vec<serde_json::Value> {
        optimization_runtime_tool_specs()
    }

    /// optimize 域编排短名：取会话，不存在则 NotFound。
    pub(crate) async fn get_session(
        &self,
        session_id: i64,
    ) -> Result<optimization_session::Model, AppError> {
        self.optimization_session_by_id(session_id)
            .await?
            .ok_or_else(|| AppError::NotFound("优化会话不存在".into()))
    }

    /// optimize 域编排短名：会话消息列表。
    pub(crate) async fn session_messages(
        &self,
        session_id: i64,
    ) -> Result<Vec<optimization_session_message::Model>, AppError> {
        self.optimization_session_messages(session_id).await
    }

    pub(crate) async fn load_optimize_chapter_for_operation(
        &self,
        session: &optimization_session::Model,
        operation: &str,
        deleted_message: &str,
        empty_content_message: Option<&str>,
    ) -> Result<chapter::Model, AppError> {
        let chapter = self
            .optimization_chapter_by_id(session.chapter_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("章节不存在: {}", session.chapter_id)))?;
        if chapter.status == "deleted" {
            return Err(AppError::InvalidArgument(deleted_message.to_string()));
        }
        if chapter.status == "generating" {
            return Err(AppError::InvalidState {
                current: chapter.status,
                operation: operation.to_string(),
            });
        }
        if let Some(message) = empty_content_message
            && chapter.content.trim().is_empty()
        {
            return Err(AppError::InvalidArgument(message.to_string()));
        }
        Ok(chapter)
    }

    /// 内容优化会话：校验后创建 workflow_run（RUNNING）。
    pub(crate) async fn begin_optimize_session_workflow_validated(
        &self,
        session_id: i64,
        workflow_kind: &str,
        mode: &str,
        project_id: i64,
        chapter_id: i64,
        active_task_id: Option<String>,
    ) -> Result<a4n_core::models::generation::workflow_run::Model, AppError> {
        let mut input = crate::optimize::optimize_session_workflow_run_input(
            session_id,
            workflow_kind,
            mode,
            project_id,
            chapter_id,
        )
        .map_err(AppError::InvalidArgument)?;
        input.active_task_id = active_task_id;
        self.begin_workflow_run_validated(input).await
    }
}
