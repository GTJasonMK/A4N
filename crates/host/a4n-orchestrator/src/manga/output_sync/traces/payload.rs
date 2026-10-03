use a4n_db::generation::{
    GenerationRunCheckpointContext, generation_checkpoint_fingerprint_sha256,
};
use a4n_db::manga::MANGA_PAGE_PROMPT_TYPE;
use a4n_domain::generation::{GenerationRunTraceTarget, RecordCompletedGenerationRunInput};
use a4n_domain::manga::MangaPagePromptRecord;
use a4n_error::AppError;
use serde_json::json;

use super::super::super::layout_rules::{manga_page_generation_layout, manga_page_pipeline_layout};
use super::super::super::pipeline::prompt::MangaPromptGenerationTrace;
use super::contracts::MangaPageGenerationTraceRecord;

pub(super) fn build_manga_page_generation_trace_record(
    row: &MangaPagePromptRecord,
    trace: MangaPromptGenerationTrace,
) -> Result<MangaPageGenerationTraceRecord, AppError> {
    let (source, reference_bindings) = if row
        .layout
        .as_ref()
        .and_then(|layout| layout.get("schema_version"))
        .and_then(serde_json::Value::as_str)
        == Some("manga_page_generation_v4")
    {
        let layout = manga_page_pipeline_layout(row).map_err(AppError::InvalidArgument)?;
        (layout.source, layout.reference_bindings)
    } else {
        let layout = manga_page_generation_layout(row).map_err(AppError::InvalidArgument)?;
        (layout.source, layout.reference_bindings)
    };
    let source_refs_json = Some(json!({ "source": source }));
    let media_refs_json = Some(json!({
        "reference_bindings": reference_bindings,
    }));
    let input_fingerprint_json = trace.input_fingerprint_json.clone();
    let output_json = json!({
        "chapter_manga_prompt_id": row.id,
        "chapter_id": row.chapter_id,
        "page_number": row.page_number,
        "image_prompt": row.image_prompt.clone(),
        "dialogue_text": row.dialogue_text.clone(),
        "characters_in_frame": row.characters_in_frame.clone(),
        "layout": row.layout.clone(),
        "status": row.status.clone(),
    });
    let artifact_refs_json = json!({
        "targets": [{
            "target_kind": MANGA_PAGE_PROMPT_TYPE,
            "target_table": "chapter_manga_prompt",
            "target_id": row.id,
        }],
    });

    Ok(MangaPageGenerationTraceRecord {
        run: RecordCompletedGenerationRunInput {
            project_id: row.project_id,
            workflow: "manga_prompt_generation".to_string(),
            step_key: trace.prompt_name.clone(),
            title: "漫画页面提示词生成".to_string(),
            model_kind: "llm".to_string(),
            provider: None,
            model: None,
            prompt_template_names_json: Some(json!([trace.prompt_name])),
            prompt_template_hashes_json: None,
            rendered_messages_json: Some(trace.messages_json.clone()),
            rendered_prompts_json: None,
            input_variables_json: None,
            source_refs_json: source_refs_json.clone(),
            media_refs_json: media_refs_json.clone(),
            request_json: Some(trace.request_json.clone()),
            response_summary_json: Some(json!({
                "chapter_manga_prompt_id": row.id,
                "page_number": row.page_number,
                "output": trace.response_text.clone(),
                "llm_usage": trace.usage.clone(),
            })),
            targets: vec![GenerationRunTraceTarget {
                target_kind: MANGA_PAGE_PROMPT_TYPE.to_string(),
                target_table: "chapter_manga_prompt".to_string(),
                target_id: row.id,
            }],
        },
        checkpoint: GenerationRunCheckpointContext {
            workflow_run_id: Some(trace.workflow_run_id),
            call_key: Some(trace.call_key),
            input_fingerprint_sha256: Some(generation_checkpoint_fingerprint_sha256(
                &input_fingerprint_json,
            )?),
            input_fingerprint_json: Some(input_fingerprint_json),
            raw_response_json: Some(json!({
                "text": trace.response_text,
                "usage": trace.usage,
            })),
            parsed_output_json: Some(output_json),
            validation_json: None,
            artifact_refs_json: Some(artifact_refs_json),
            attempt: None,
        },
    })
}
