use super::{OptimizeService, is_cancelled_optimize_error, patch::OptimizationPatch};
use a4n_core::error::AppError;
use a4n_core::models::ai::{optimization_session, optimization_session_message};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, Set};
use serde_json::{Value, json};

impl OptimizeService {
    pub async fn record_agent_event(&self, session_id: i64, content: &str) -> Result<(), AppError> {
        self.save_agent_event(session_id, content).await?;
        Ok(())
    }

    pub async fn record_llm_request(
        &self,
        session_id: i64,
        phase: &str,
        content: &str,
    ) -> Result<(), AppError> {
        let payload = serde_json::to_string_pretty(&json!({
            "phase": phase,
            "content": content,
        }))?;
        self.save_message(session_id, "llm_request", &payload)
            .await?;
        Ok(())
    }

    pub async fn record_llm_response(
        &self,
        session_id: i64,
        phase: &str,
        content: &str,
    ) -> Result<(), AppError> {
        let payload = serde_json::to_string_pretty(&json!({
            "phase": phase,
            "content": content,
        }))?;
        self.save_message(session_id, "llm_response", &payload)
            .await?;
        Ok(())
    }

    pub async fn record_command_error(
        &self,
        session_id: i64,
        phase: &str,
        err: &str,
    ) -> Result<(), AppError> {
        self.save_agent_event(session_id, &format!("{phase}失败或停止：{err}"))
            .await?;
        if is_cancelled_optimize_error(err) {
            return self.mark_session_cancelled_after_stop(session_id).await;
        }
        self.release_in_progress_session_after_failure(session_id, phase, err)
            .await
    }

    pub async fn record_patches_applied(
        &self,
        session_id: i64,
        chapter_id: i64,
        word_count: i32,
        content_hash: &str,
        patches: &[OptimizationPatch],
    ) -> Result<(), AppError> {
        if patches.is_empty() {
            return Err(AppError::InvalidArgument(
                "应用补丁列表不能为空".to_string(),
            ));
        }
        let session = self.get_session(session_id).await?;
        let content = serde_json::to_string_pretty(&json!({
            "tool": "apply_chapter_patches",
            "chapter_id": chapter_id,
            "applied": true,
            "patch_count": patches.len(),
            "word_count": word_count,
            "content_hash": content_hash,
            "patch_ids": patches.iter().map(|patch| patch.patch_id.as_str()).collect::<Vec<_>>(),
        }))?;
        self.save_message_with_tool_calls(
            session_id,
            "tool",
            &content,
            Some(json!({
                "tool": "apply_chapter_patches",
                "chapter_id": chapter_id,
            })),
        )
        .await?;
        self.save_agent_event(
            session_id,
            &format!(
                "已应用 {} 个局部补丁到章节正文，当前字数 {word_count}。",
                patches.len()
            ),
        )
        .await?;

        let patch_count = patches.len();
        let next_fixes_applied = session.fixes_applied.saturating_add(patch_count as i32);
        let mut active: optimization_session::ActiveModel = session.into();
        active.status = Set("waiting_user".to_string());
        active.fixes_applied = Set(next_fixes_applied);
        active.summary = Set(Some(format!(
            "已应用 {patch_count} 个局部补丁，当前字数 {word_count}"
        )));
        active.updated_at = Set(Utc::now().naive_utc());
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }

    pub(super) async fn release_in_progress_session_after_failure(
        &self,
        session_id: i64,
        phase: &str,
        err: &str,
    ) -> Result<(), AppError> {
        if is_cancelled_optimize_error(err) {
            return Ok(());
        }
        let session = self.get_session(session_id).await?;
        if !matches!(session.status.as_str(), "analyzing" | "fixing") {
            return Ok(());
        }

        let mut active: optimization_session::ActiveModel = session.into();
        active.status = Set("waiting_user".to_string());
        active.summary = Set(Some(format!("{phase}失败：{err}")));
        active.updated_at = Set(Utc::now().naive_utc());
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }

    async fn mark_session_cancelled_after_stop(&self, session_id: i64) -> Result<(), AppError> {
        let session = self.get_session(session_id).await?;
        if session.status == "cancelled" {
            return Ok(());
        }
        self.save_agent_event(session_id, "会话已取消，当前运行不会继续写入结果。")
            .await?;
        let mut active: optimization_session::ActiveModel = session.into();
        active.status = Set("cancelled".to_string());
        active.updated_at = Set(Utc::now().naive_utc());
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }

    /// 取消会话
    pub async fn cancel_session(&self, session_id: i64) -> Result<(), AppError> {
        let session = self.get_session(session_id).await?;
        self.save_agent_event(session_id, "会话已取消，当前运行不会继续写入结果。")
            .await?;
        let mut active: optimization_session::ActiveModel = session.into();
        active.status = Set("cancelled".to_string());
        active.updated_at = Set(Utc::now().naive_utc());
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }

    /// 删除单个优化会话及其消息记录。
    pub async fn delete_session(&self, session_id: i64) -> Result<(), AppError> {
        self.get_session(session_id).await?;
        optimization_session_message::Entity::delete_many()
            .filter(optimization_session_message::Column::SessionId.eq(session_id))
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let result = optimization_session::Entity::delete_many()
            .filter(optimization_session::Column::Id.eq(session_id))
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        if result.rows_affected != 1 {
            return Err(AppError::NotFound(format!("优化会话不存在: {session_id}")));
        }
        Ok(())
    }

    pub(super) async fn save_message(
        &self,
        session_id: i64,
        role: &str,
        content: &str,
    ) -> Result<optimization_session_message::Model, AppError> {
        self.save_message_with_tool_calls(session_id, role, content, None)
            .await
    }

    pub(super) async fn save_agent_event(
        &self,
        session_id: i64,
        content: &str,
    ) -> Result<optimization_session_message::Model, AppError> {
        self.save_message(session_id, "agent_event", content).await
    }

    pub(super) async fn save_message_with_tool_calls(
        &self,
        session_id: i64,
        role: &str,
        content: &str,
        tool_calls: Option<Value>,
    ) -> Result<optimization_session_message::Model, AppError> {
        optimization_session_message::ActiveModel {
            session_id: Set(session_id),
            role: Set(role.to_string()),
            content: Set(content.to_string()),
            tool_calls: Set(tool_calls),
            created_at: Set(Utc::now().naive_utc()),
            ..Default::default()
        }
        .insert(self.db.as_ref())
        .await
        .map_err(AppError::Database)
    }
}
