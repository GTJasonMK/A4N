use a4n_core::error::AppError;
use a4n_llm::context::LlmContextManifest;

use a4n_llm::context::LlmContextLedger;

use super::super::contracts::MangaContextManifestInput;
use super::packs::{
    add_manga_previous_output_pack, add_manga_prompt_rule_packs, add_manga_reference_assets_pack,
    add_manga_source_pack,
};

pub(in crate::manga) async fn build_manga_context_manifest(
    input: MangaContextManifestInput<'_>,
) -> Result<LlmContextManifest, AppError> {
    let mut ledger = LlmContextLedger::new();
    add_manga_source_pack(&mut ledger, input.chapter_content, input.source_spans)?;
    add_manga_prompt_rule_packs(
        &mut ledger,
        input.prompt_renderer,
        input.project_id,
        input.variables,
        input.prompt_names,
    )
    .await?;
    for previous_output in input.previous_outputs {
        add_manga_previous_output_pack(&mut ledger, previous_output)?;
    }
    if let Some(reference_assets) = input.reference_assets {
        add_manga_reference_assets_pack(&mut ledger, reference_assets)?;
    }
    Ok(ledger.into_manifest(input.dynamic_user_content, None))
}
