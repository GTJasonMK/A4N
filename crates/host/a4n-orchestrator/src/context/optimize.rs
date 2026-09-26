//! 内容优化领域私有 owner。
//!
//! 持有 `DbHandle`，封装优化会话、消息、Turn、补丁集与上下文读模型的数据库行为。
//! 分析/对话/补丁编排仍在 `optimize/` 模块，但 DB 只经本 owner。

use a4n_db::DbHandle;
use a4n_db::optimize::OptimizationSessionMessageInsertInput;
use a4n_db_models::models::ai::{
    optimization_patch, optimization_patch_set, optimization_session, optimization_session_message,
    optimization_turn, optimization_turn_event,
};
use a4n_db_models::models::chapter::chapter;
use a4n_error::AppError;
use std::collections::HashMap;

use super::OrchestratorContext;
use crate::optimize::optimize_db_error;
use crate::optimize::runtime_tools::optimization_runtime_tool_specs;
use a4n_domain::collaboration::ProjectAccessRole;

const PATCH_STATUS_PENDING: &str = "pending";
const PATCH_STATUS_REJECTED: &str = "rejected";
const PATCH_DECISION_REJECTED: &str = "rejected";
const PATCH_APPLY_STATUS_PENDING: &str = "pending";

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
        a4n_db::optimize::optimization_sessions(&self.database, project_id, chapter_id)
            .await
            .map_err(optimize_db_error)
    }

    async fn optimization_session_by_id(
        &self,
        session_id: i64,
    ) -> Result<Option<optimization_session::Model>, AppError> {
        a4n_db::optimize::optimization_session_by_id(&self.database, session_id)
            .await
            .map_err(optimize_db_error)
    }

    async fn insert_optimization_session_message(
        &self,
        input: OptimizationSessionMessageInsertInput,
    ) -> Result<optimization_session_message::Model, AppError> {
        a4n_db::optimize::insert_optimization_session_message(&self.database, input)
            .await
            .map_err(optimize_db_error)
    }

    async fn delete_optimization_session_with_messages(
        &self,
        session_id: i64,
    ) -> Result<u64, AppError> {
        a4n_db::optimize::delete_optimization_session_with_messages(&self.database, session_id)
            .await
            .map_err(optimize_db_error)
    }

    async fn optimization_session_messages(
        &self,
        session_id: i64,
    ) -> Result<Vec<optimization_session_message::Model>, AppError> {
        a4n_db::optimize::optimization_session_messages(&self.database, session_id)
            .await
            .map_err(optimize_db_error)
    }

    async fn optimization_chapter_by_id(
        &self,
        chapter_id: i64,
    ) -> Result<Option<chapter::Model>, AppError> {
        a4n_db::optimize::optimization_chapter_by_id(&self.database, chapter_id)
            .await
            .map_err(optimize_db_error)
    }

    async fn optimization_patch_sets(
        &self,
        session_id: i64,
    ) -> Result<Vec<optimization_patch_set::Model>, AppError> {
        a4n_db::optimize::optimization_patch_sets(&self.database, session_id)
            .await
            .map_err(optimize_db_error)
    }

    async fn optimization_patch_set_patches_for_sets(
        &self,
        patch_set_ids: &[i64],
    ) -> Result<Vec<optimization_patch::Model>, AppError> {
        a4n_db::optimize::optimization_patch_set_patches_for_sets(&self.database, patch_set_ids)
            .await
            .map_err(optimize_db_error)
    }

    async fn optimization_turns(
        &self,
        session_id: i64,
    ) -> Result<Vec<optimization_turn::Model>, AppError> {
        a4n_db::optimize::optimization_turns(&self.database, session_id)
            .await
            .map_err(optimize_db_error)
    }

    async fn optimization_turn_events_for_turns(
        &self,
        turn_ids: &[i64],
    ) -> Result<Vec<optimization_turn_event::Model>, AppError> {
        a4n_db::optimize::optimization_turn_events_for_turns(&self.database, turn_ids)
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

    pub(crate) async fn optimization_turns(
        &self,
        session_id: i64,
    ) -> Result<Vec<optimization_turn::Model>, AppError> {
        self.optimize.optimization_turns(session_id).await
    }

    pub(crate) async fn discard_pending_optimization_patch_sets(
        &self,
        session_id: i64,
    ) -> Result<(), AppError> {
        let transaction = self.database.begin().await?;
        let patch_sets = a4n_db::optimize::pending_optimization_patch_sets(
            &transaction,
            session_id,
            PATCH_STATUS_PENDING,
        )
        .await
        .map_err(optimize_db_error)?;
        let patch_set_ids = patch_sets
            .iter()
            .map(|patch_set| patch_set.id)
            .collect::<Vec<_>>();
        a4n_db::optimize::update_optimization_patch_sets_status(
            &transaction,
            &patch_set_ids,
            PATCH_STATUS_REJECTED,
            PATCH_DECISION_REJECTED,
            PATCH_APPLY_STATUS_PENDING,
        )
        .await
        .map_err(optimize_db_error)?;
        a4n_db::optimize::insert_optimization_session_message(
            &transaction,
            OptimizationSessionMessageInsertInput {
                session_id,
                role: "agent_event".to_string(),
                content: "用户已丢弃当前待应用局部补丁。".to_string(),
                tool_calls: None,
            },
        )
        .await
        .map_err(optimize_db_error)?;
        a4n_db::commit_transaction(transaction).await?;
        Ok(())
    }

    pub(crate) async fn discard_optimization_patch_set(
        &self,
        session_id: i64,
        patch_set_id: i64,
    ) -> Result<(), AppError> {
        let transaction = self.database.begin().await?;
        let patch_set = a4n_db::optimize::optimization_patch_set_by_id(&transaction, patch_set_id)
            .await
            .map_err(optimize_db_error)?
            .ok_or_else(|| AppError::NotFound(format!("优化补丁集不存在: {patch_set_id}")))?;
        if patch_set.session_id != session_id {
            return Err(AppError::Forbidden("优化补丁集不属于当前优化会话".into()));
        }
        if patch_set.status != PATCH_STATUS_PENDING {
            return Err(AppError::InvalidState {
                current: patch_set.status,
                operation: "拒绝优化补丁集".into(),
            });
        }
        a4n_db::optimize::update_optimization_patch_sets_status(
            &transaction,
            &[patch_set_id],
            PATCH_STATUS_REJECTED,
            PATCH_DECISION_REJECTED,
            PATCH_APPLY_STATUS_PENDING,
        )
        .await
        .map_err(optimize_db_error)?;
        a4n_db::optimize::insert_optimization_session_message(
            &transaction,
            OptimizationSessionMessageInsertInput {
                session_id,
                role: "agent_event".to_string(),
                content: "用户已拒绝当前正文补丁。".to_string(),
                tool_calls: None,
            },
        )
        .await
        .map_err(optimize_db_error)?;
        a4n_db::commit_transaction(transaction).await?;
        Ok(())
    }
}

impl OrchestratorContext {
    pub(crate) async fn optimize_sessions_for_user(
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

    pub(crate) async fn optimize_session_for_user(
        &self,
        user_id: i64,
        session_id: i64,
    ) -> Result<optimization_session::Model, AppError> {
        let session = self.get_session(session_id).await?;
        self.verify_project_permission(session.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        Ok(session)
    }

    pub(crate) async fn optimize_turns_for_user(
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
        let turn_ids = turns.iter().map(|turn| turn.id).collect::<Vec<_>>();
        let events = self
            .optimize
            .optimization_turn_events_for_turns(&turn_ids)
            .await?;
        let mut events_by_turn = HashMap::<i64, Vec<optimization_turn_event::Model>>::new();
        for event in events {
            events_by_turn.entry(event.turn_id).or_default().push(event);
        }
        Ok(turns
            .into_iter()
            .map(|turn| {
                let events = events_by_turn.remove(&turn.id).unwrap_or_default();
                (turn, events)
            })
            .collect())
    }

    pub(crate) async fn optimize_patch_sets_for_user(
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
        let patch_set_ids = patch_sets
            .iter()
            .map(|patch_set| patch_set.id)
            .collect::<Vec<_>>();
        let patches = self
            .optimize
            .optimization_patch_set_patches_for_sets(&patch_set_ids)
            .await?;
        let mut patches_by_set = HashMap::<i64, Vec<optimization_patch::Model>>::new();
        for patch in patches {
            patches_by_set
                .entry(patch.patch_set_id)
                .or_default()
                .push(patch);
        }
        Ok(patch_sets
            .into_iter()
            .map(|patch_set| {
                let patches = patches_by_set.remove(&patch_set.id).unwrap_or_default();
                (patch_set, patches)
            })
            .collect())
    }

    pub(crate) async fn delete_optimize_session_for_user_validated(
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

    pub(crate) async fn record_optimize_agent_event_for_user_validated(
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
    ) -> Result<a4n_db_models::models::generation::workflow_run::Model, AppError> {
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
