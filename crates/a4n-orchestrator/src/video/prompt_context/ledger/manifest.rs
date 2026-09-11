use std::collections::HashMap;

use a4n_error::AppError;
use serde_json::{Value, json};

use super::{
    ChapterVideoSourceContext, LlmContextLedger, LlmContextManifest, PromptRenderer,
    VIDEO_CONTEXT_LEDGER_VERSION, VideoContextManifestInput, add_builder_rules_pack,
    add_chapter_source_pack, add_optional_value_pack, add_previous_output_pack,
    add_prompt_rule_hash_pack, add_prompt_rule_packs, add_public_chapter_source_pack,
};

pub(in crate::video) async fn build_video_context_manifest(
    input: VideoContextManifestInput<'_>,
) -> Result<LlmContextManifest, AppError> {
    let VideoContextManifestInput {
        prompt_renderer,
        project_id,
        variables,
        source,
        source_spans,
        prompt_names,
        previous_outputs,
        project_context,
        asset_context,
        dynamic_user_content,
    } = input;
    let mut ledger = LlmContextLedger::new();
    add_chapter_source_pack(&mut ledger, source, source_spans)?;
    add_builder_rules_pack(&mut ledger, prompt_renderer, project_id).await?;
    add_prompt_rule_packs(
        &mut ledger,
        prompt_renderer,
        project_id,
        variables,
        prompt_names,
    )
    .await?;
    for previous_output in previous_outputs {
        add_previous_output_pack(&mut ledger, previous_output)?;
    }
    if let Some(project_context) = project_context {
        add_optional_value_pack(
            &mut ledger,
            "video_project_context",
            json!({"ledger_version": VIDEO_CONTEXT_LEDGER_VERSION, "kind": "project_context"}),
            project_context,
        )?;
    }
    if let Some(asset_context) = asset_context {
        add_optional_value_pack(
            &mut ledger,
            "video_asset_bindings",
            json!({"ledger_version": VIDEO_CONTEXT_LEDGER_VERSION, "kind": "asset_context"}),
            asset_context,
        )?;
    }
    Ok(ledger.into_manifest(dynamic_user_content, None))
}

pub(in crate::video) async fn build_final_video_context_manifest(
    prompt_renderer: &dyn PromptRenderer,
    project_id: i64,
    variables: &HashMap<String, Value>,
    source: &ChapterVideoSourceContext<'_>,
    prompt_names: &[&str],
    dynamic_user_content: String,
) -> Result<LlmContextManifest, AppError> {
    let mut ledger = LlmContextLedger::new();
    add_public_chapter_source_pack(&mut ledger, source)?;
    add_prompt_rule_hash_pack(
        &mut ledger,
        prompt_renderer,
        project_id,
        variables,
        prompt_names,
    )
    .await?;
    Ok(ledger.into_manifest(dynamic_user_content, None))
}
