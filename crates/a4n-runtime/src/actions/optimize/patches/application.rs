use super::{
    super::{
        ApplyOptimizationPatchesInput, ApplyOptimizationPatchesResponse,
        access::{find_chapter_by_id, optimize_session},
        workflow::{OptimizeCommandError, record_optimize_command_result},
    },
    scope::apply_optimization_patches_task_scope,
    status::mark_matching_patch_sets_applied,
};
use crate::AppState;
use a4n_orchestrator::optimize::{
    OptimizationChapterPatchWriteInput, OptimizationPatch, OptimizeProgress,
    apply_optimization_patches_to_content, optimization_content_hash,
};

pub async fn apply_optimization_patches(
    state: &AppState,
    user_id: i64,
    input: ApplyOptimizationPatchesInput,
    mut progress: Option<OptimizeProgress<'_>>,
) -> Result<ApplyOptimizationPatchesResponse, String> {
    let session_id = input.session_id;
    apply_optimization_patches_task_scope(state, user_id, &input).await?;
    let result = async {
        if let Some(progress) = progress.as_mut() {
            progress(0, 3, "正在校验局部补丁").map_err(OptimizeCommandError::from_app)?;
        }
        if !input.confirmed_apply {
            return Err(OptimizeCommandError::from_message(
                "应用局部补丁必须经过用户确认",
            ));
        }
        if input.patches.is_empty() {
            return Err(OptimizeCommandError::from_message("应用补丁列表不能为空"));
        }
        if let Some(progress) = progress.as_mut() {
            progress(1, 3, "正在写入局部补丁结果").map_err(OptimizeCommandError::from_app)?;
        }
        let written = write_patches_to_session_chapter(
            state,
            input.session_id,
            &input.base_content_hash,
            &input.patches,
        )
        .await
        .map_err(OptimizeCommandError::from_message)?;
        crate::workflow::postprocess_chapter_after_content_change(state, written.chapter_id)
            .await
            .map_err(OptimizeCommandError::from_app)?;
        let updated = state.chapter.get(written.chapter_id).await.map_err(|err| {
            OptimizeCommandError::from_message(format!("应用局部补丁后刷新章节失败: {err}"))
        })?;
        let content_hash = optimization_content_hash(&updated.content);
        state
            .record_patches_applied(
                input.session_id,
                updated.id,
                updated.word_count,
                &content_hash,
                &input.patches,
            )
            .await
            .map_err(OptimizeCommandError::from_app)?;
        mark_matching_patch_sets_applied(state, input.session_id, &input.base_content_hash)
            .await
            .map_err(OptimizeCommandError::from_message)?;
        if let Some(progress) = progress.as_mut() {
            progress(3, 3, "局部补丁已应用").map_err(OptimizeCommandError::from_app)?;
        }
        Ok(ApplyOptimizationPatchesResponse {
            session_id: input.session_id,
            chapter_id: updated.id,
            applied_count: input.patches.len(),
            word_count: updated.word_count,
            content_hash,
        })
    }
    .await;
    record_optimize_command_result(state, session_id, "应用局部补丁", result).await
}

struct WrittenChapterPatch {
    chapter_id: i64,
}

async fn write_patches_to_session_chapter(
    state: &AppState,
    session_id: i64,
    base_content_hash: &str,
    patches: &[OptimizationPatch],
) -> Result<WrittenChapterPatch, String> {
    if patches.is_empty() {
        return Err("应用补丁列表不能为空".to_string());
    }
    let session = optimize_session(state, session_id).await?;
    let chapter = find_chapter_by_id(state, session.chapter_id).await?;
    if chapter.status == "deleted" {
        return Err("已删除章节不能应用局部补丁".to_string());
    }
    if chapter.status == "generating" {
        return Err(format!(
            "状态无效：当前状态 {}，不允许操作 {}",
            chapter.status, "应用局部补丁"
        ));
    }
    let next_content =
        apply_optimization_patches_to_content(&chapter.content, base_content_hash, patches)
            .map_err(|err| err.to_string())?;
    let word_count = i32::try_from(next_content.chars().count())
        .map_err(|_| "应用补丁后的章节正文字符数超出 i32 范围".to_string())?;
    let updated = state
        .orchestrator
        .write_optimization_chapter_patch_content(OptimizationChapterPatchWriteInput {
            chapter,
            content: next_content,
            word_count,
        })
        .await
        .map_err(|err| err.to_string())?;
    Ok(WrittenChapterPatch {
        chapter_id: updated.id,
    })
}
