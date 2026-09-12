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
) -> Result<(String, Vec<usize>), AppError> {
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
    let mut stable_prefix_lens = if stable_packs.is_empty() {
        Vec::new()
    } else {
        let (section, boundaries) = render_pack_section("稳定上下文", stable_packs);
        sections.push(section);
        boundaries
    };
    if let Some(contract) = manifest
        .output_contract
        .as_deref()
        .filter(|text| !text.trim().is_empty())
    {
        sections.push(format!("[输出契约]\n{contract}"));
        let boundary = sections.iter().map(String::len).sum::<usize>() + 2 * (sections.len() - 1);
        if let Some(last) = stable_prefix_lens.last_mut() {
            *last = boundary;
        } else {
            stable_prefix_lens.push(boundary);
        }
    }
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
        sections.push(render_pack_section("可变上下文", variable_packs).0);
    }
    if !manifest.dynamic_user_content.trim().is_empty() {
        sections.push(format!("[当前任务]\n{}", manifest.dynamic_user_content));
    }
    if sections.is_empty() {
        return Err(AppError::InvalidArgument(
            "LLM 上下文 manifest 没有可发送的用户内容".to_string(),
        ));
    }
    Ok((sections.join("\n\n"), stable_prefix_lens))
}
