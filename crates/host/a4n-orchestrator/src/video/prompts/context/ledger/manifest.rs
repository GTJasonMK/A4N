use std::collections::HashMap;

use a4n_error::AppError;
use serde_json::{Value, json};

use super::{
    ChapterVideoSourceContext, LlmContextLedger, LlmContextManifest, PromptRenderer,
    VIDEO_CONTEXT_LEDGER_VERSION, VideoContextManifestInput, add_chapter_source_pack,
    add_optional_value_pack, add_previous_output_pack, record_prompt_dependencies,
};

pub(in crate::video) async fn build_video_context_manifest(
    input: VideoContextManifestInput<'_>,
) -> Result<LlmContextManifest, AppError> {
    let VideoContextManifestInput {
        prompt_renderer,
        project_id,
        variables,
        source,
        prompt_names,
        previous_outputs,
        project_context,
        dynamic_user_content,
    } = input;
    let mut ledger = LlmContextLedger::new();
    add_chapter_source_pack(&mut ledger, source)?;
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
    let mut manifest = ledger.into_manifest(dynamic_user_content, None);
    record_prompt_dependencies(
        &mut manifest,
        prompt_renderer,
        project_id,
        variables,
        prompt_names,
    )
    .await?;
    Ok(manifest)
}

pub(in crate::video) async fn build_final_video_context_manifest(
    prompt_renderer: &dyn PromptRenderer,
    project_id: i64,
    variables: &HashMap<String, Value>,
    source: &ChapterVideoSourceContext<'_>,
    prompt_names: &[&str],
    dynamic_user_content: String,
) -> Result<LlmContextManifest, AppError> {
    build_video_context_manifest(VideoContextManifestInput {
        prompt_renderer,
        project_id,
        variables,
        source,
        prompt_names,
        previous_outputs: &[],
        project_context: None,
        dynamic_user_content,
    })
    .await
}
