use a4n_error::AppError;

use super::super::contracts::{LlmContextManifest, PlannedContextPack};
mod section;
use section::render_pack_section;

pub fn render_context_manifest_user_content(
    manifest: &LlmContextManifest,
    selected_packs: &[PlannedContextPack],
) -> Result<String, AppError> {
    render_context_manifest_user_content_with_cache_boundary(manifest, selected_packs)
        .map(|(content, _)| content)
}

pub(crate) fn render_context_manifest_user_content_with_cache_boundary(
    manifest: &LlmContextManifest,
    selected_packs: &[PlannedContextPack],
) -> Result<(String, Option<usize>), AppError> {
    let mut sections = Vec::new();
    let stable_packs = selected_packs
        .iter()
        .filter(|pack| pack.stable_prefix)
        .filter_map(|pack| {
            pack.canonical_content_text
                .as_deref()
                .map(|content| (pack, content))
        })
        .collect::<Vec<_>>();
    let stable_prefix_len = if stable_packs.is_empty() {
        None
    } else {
        let section = render_pack_section("稳定上下文", stable_packs);
        let len = section.len();
        sections.push(section);
        Some(len)
    };
    let variable_packs = selected_packs
        .iter()
        .filter(|pack| !pack.stable_prefix)
        .filter_map(|pack| {
            pack.canonical_content_text
                .as_deref()
                .map(|content| (pack, content))
        })
        .collect::<Vec<_>>();
    if !variable_packs.is_empty() {
        sections.push(render_pack_section("可变上下文", variable_packs));
    }
    if !manifest.dynamic_user_content.trim().is_empty() {
        sections.push(format!("[当前任务]\n{}", manifest.dynamic_user_content));
    }
    if let Some(output_contract) = manifest.output_contract.as_deref()
        && !output_contract.trim().is_empty()
    {
        sections.push(format!("[输出契约]\n{output_contract}"));
    }
    if sections.is_empty() {
        return Err(AppError::InvalidArgument(
            "LLM 上下文 manifest 没有可发送的用户内容".to_string(),
        ));
    }
    Ok((sections.join("\n\n"), stable_prefix_len))
}
