use a4n_llm::provider::is_truncated_finish_reason;

pub(super) fn ensure_character_sync_response_not_truncated(
    finish_reason: Option<&str>,
    label: &str,
) -> Result<(), String> {
    if is_truncated_finish_reason(finish_reason) {
        return Err(format!(
            "模型生成{label}时达到当前角色同步输出上限，返回 JSON 被截断；请在设置页调大“角色同步” max_tokens，或减少本次同步角色/章节的输出量后重试"
        ));
    }
    Ok(())
}
