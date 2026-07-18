use a4n_core::error::AppError;
use a4n_db::optimize::OptimizationSessionUpdateInput;
use serde_json::json;

use super::super::patch::OptimizationPatch;
use super::super::{OrchestratorContext, optimize_db_error};

impl OrchestratorContext {
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
        let patch_count = patches.len();
        let patch_count_i32 = i32::try_from(patch_count)
            .map_err(|_| AppError::InvalidArgument("本次应用补丁数量超出 i32 范围".to_string()))?;
        let next_fixes_applied = session
            .fixes_applied
            .checked_add(patch_count_i32)
            .ok_or_else(|| AppError::InvalidArgument("优化修复计数超出 i32 范围".to_string()))?;
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
        a4n_db::optimize::update_optimization_session(
            self.db.connection(),
            OptimizationSessionUpdateInput {
                session,
                status: Some("waiting_user".to_string()),
                issues: None,
                agent_permissions: None,
                agent_instruction: None,
                fixes_applied: Some(next_fixes_applied),
                summary: Some(Some(format!(
                    "已应用 {patch_count} 个局部补丁，当前字数 {word_count}"
                ))),
            },
        )
        .await
        .map_err(optimize_db_error)?;
        Ok(())
    }
}
