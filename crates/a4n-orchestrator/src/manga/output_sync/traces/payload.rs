use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_manga_prompt;
use a4n_db::generation::{
    GenerationRunCheckpointContext, GenerationRunTraceTarget, RecordCompletedGenerationRunInput,
    generation_checkpoint_fingerprint_sha256,
};
use serde_json::json;

use super::super::super::pipeline_prompt::MangaPromptGenerationTrace;
use super::contracts::MangaGenerationTraceRecord;

pub(super) fn build_manga_generation_trace_record(
    row: &chapter_manga_prompt::Model,
    trace: MangaPromptGenerationTrace,
) -> Result<MangaGenerationTraceRecord, AppError> {
    let source_refs_json = row
        .layout
        .as_ref()
        .and_then(|layout| layout.get("source"))
        .cloned()
        .map(|source| json!({ "source": source }));
    let media_refs_json = row.layout.as_ref().and_then(|layout| {
        layout.get("reference_bindings").cloned().map(|bindings| {
            json!({
                "reference_bindings": bindings,
            })
        })
    });
    let call_key = trace.call_key.clone().ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "漫画 LLM trace 缺少调用级 call_key: {}",
            trace.prompt_name
        ))
    })?;
    let input_fingerprint_json = trace.input_fingerprint_json.clone().ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "漫画 LLM trace 缺少调用级 input_fingerprint_json: {}",
            trace.prompt_name
        ))
    })?;
    let output_json = json!({
        "chapter_manga_prompt_id": row.id,
        "chapter_id": row.chapter_id,
        "prompt_type": row.prompt_type.clone(),
        "page_number": row.page_number,
        "panel_number": row.panel_number,
        "image_prompt": row.image_prompt.clone(),
        "dialogue_text": row.dialogue_text.clone(),
        "characters_in_frame": row.characters_in_frame.clone(),
        "layout": row.layout.clone(),
        "camera_angle": row.camera_angle.clone(),
        "dialogue_position": row.dialogue_position.clone(),
        "status": row.status.clone(),
    });
    let artifact_refs_json = json!({
        "targets": [{
            "target_kind": row.prompt_type.clone(),
            "target_table": "chapter_manga_prompt",
            "target_id": row.id,
        }],
    });

    Ok(MangaGenerationTraceRecord {
        run: RecordCompletedGenerationRunInput {
            project_id: row.project_id,
            workflow: "manga_prompt_generation".to_string(),
            step_key: trace.prompt_name.clone(),
            title: format!("漫画提示词生成: {}", row.prompt_type),
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
                "prompt_type": row.prompt_type,
                "page_number": row.page_number,
                "output": trace.response_text.clone(),
                "llm_usage": trace.usage.clone(),
            })),
            targets: vec![GenerationRunTraceTarget {
                target_kind: row.prompt_type.clone(),
                target_table: "chapter_manga_prompt".to_string(),
                target_id: row.id,
            }],
        },
        checkpoint: GenerationRunCheckpointContext {
            workflow_run_id: trace.workflow_run_id,
            call_key: Some(call_key),
            input_fingerprint_sha256: Some(generation_checkpoint_fingerprint_sha256(
                &input_fingerprint_json,
            )?),
            input_fingerprint_json: Some(input_fingerprint_json),
            raw_response_json: Some(json!({
                "text": trace.response_text,
                "usage": trace.usage,
                "context_manifest_summary": trace.context_manifest_summary_json,
            })),
            parsed_output_json: Some(output_json),
            validation_json: None,
            artifact_refs_json: Some(artifact_refs_json),
            attempt: None,
        },
    })
}
