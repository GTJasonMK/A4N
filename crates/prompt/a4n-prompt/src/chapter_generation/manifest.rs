//! 章节生成上下文 manifest。

use a4n_error::AppError;
use a4n_llm_context::{LlmContextLedger, LlmContextManifest};

use super::message::{normalize_context_before_section, required_generation_outline_text};

mod context_before;
mod outline;

pub(super) fn compact_required_text(
    text: &str,
    head_char_limit: usize,
    tail_char_limit: usize,
) -> String {
    let chars = text.trim().chars().collect::<Vec<_>>();
    if chars.len() <= head_char_limit.saturating_add(tail_char_limit) {
        return chars.into_iter().collect();
    }
    let head = chars.iter().take(head_char_limit).collect::<String>();
    let tail = chars
        .iter()
        .skip(chars.len().saturating_sub(tail_char_limit))
        .collect::<String>();
    format!("{head}\n\n[中间内容已按模型上下文预算压缩]\n\n{tail}")
}

pub(super) fn build_chapter_generation_context_manifest(
    chapter_outline: &str,
    context_before: Option<&str>,
    dynamic_user_content: String,
) -> Result<LlmContextManifest, AppError> {
    let outline = required_generation_outline_text(chapter_outline)?;
    let mut ledger = LlmContextLedger::new();
    ledger.add_pack(outline::chapter_outline_pack(outline))?;

    if let Some(context_before) = normalize_context_before_section(context_before) {
        ledger.add_pack(context_before::context_before_pack(&context_before))?;
    }

    Ok(ledger.into_manifest(
        dynamic_user_content,
        Some(
            "章节生成必须只返回章节正文纯文本，不要输出 JSON、Markdown、代码块、解释文字或标题标记。"
                .to_string(),
        ),
    ))
}
