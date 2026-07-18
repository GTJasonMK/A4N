use a4n_core::error::AppError;
use serde_json::json;

use crate::optimize::streaming::{OptimizeProgress, report_optimize_progress};
use crate::optimize::turns::{EVENT_KIND_ASSISTANT_UPDATE, EVENT_STATUS_COMPLETED};
use crate::optimize::{OrchestratorContext, PatchPromptBundle};
use a4n_prompt::rendering::PromptRenderer;

use super::PreparedPatchGeneration;

pub(super) struct PatchGenerationPreparationInput<'a, 'progress> {
    pub(super) session_id: i64,
    pub(super) issue_ids: Option<Vec<String>>,
    pub(super) prompt_renderer: &'a dyn PromptRenderer,
    pub(super) agent_instruction_override: Option<&'a str>,
    pub(super) progress: &'a mut Option<OptimizeProgress<'progress>>,
}

pub(super) async fn prepare_patch_generation(
    service: &OrchestratorContext,
    input: PatchGenerationPreparationInput<'_, '_>,
) -> Result<PreparedPatchGeneration, AppError> {
    let PatchGenerationPreparationInput {
        session_id,
        issue_ids,
        prompt_renderer,
        agent_instruction_override,
        progress,
    } = input;
    report_optimize_progress(progress, 0, 4, "正在读取章节与问题上下文")?;
    let base = service
        .load_patch_request_base(session_id, agent_instruction_override, "生成局部修复补丁")
        .await?;
    if let Some(instruction) = &base.override_instruction {
        service
            .save_message(base.session.id, "user", instruction)
            .await?;
    }
    report_optimize_progress(progress, 1, 4, "正在渲染局部补丁提示词")?;
    let PatchPromptBundle {
        request,
        user_message,
        base_content_hash,
        issue_count,
        permissions,
    } = service
        .build_patch_prompt_bundle(&base, issue_ids, prompt_renderer)
        .await?;
    let turn_text = base
        .override_instruction
        .as_deref()
        .unwrap_or("生成局部修复补丁");
    let turn = service
        .create_turn(base.session.id, turn_text, &permissions)
        .await?;
    let session = base.session;
    let chapter = base.chapter;
    service
        .save_agent_event(
            session.id,
            &format!(
                "准备生成局部修复补丁，覆盖 {} 个问题。补丁只会生成提案，不会自动写回正文。",
                issue_count
            ),
        )
        .await?;

    service
        .save_message(session.id, "user", &user_message)
        .await?;
    service
        .save_agent_event(session.id, "调用 LLM 生成局部修复补丁。")
        .await?;
    service
        .append_turn_event(
            turn.id,
            EVENT_KIND_ASSISTANT_UPDATE,
            EVENT_STATUS_COMPLETED,
            "根据当前问题列表生成局部正文补丁，补丁只会进入审阅状态。",
            Some(json!({ "issue_count": issue_count })),
        )
        .await?;

    Ok(PreparedPatchGeneration {
        session,
        chapter,
        turn_id: turn.id,
        request,
        base_content_hash,
    })
}
