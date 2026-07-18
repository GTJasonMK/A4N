use std::collections::HashMap;

use super::super::super::contracts::LlmContextPlan;
use super::requirements::PreviewPackContentKey;

pub(super) fn hydrate_selected_pack_content(
    plan: &mut LlmContextPlan,
    content_by_key: &HashMap<PreviewPackContentKey, String>,
) {
    for selected in &mut plan.selected_packs {
        selected.canonical_content_text = content_by_key
            .get(&(
                selected.pack_ref.pack_kind.clone(),
                selected.pack_ref.compression_level.clone(),
                selected.pack_ref.content_sha256.clone(),
            ))
            .cloned();
    }
}
