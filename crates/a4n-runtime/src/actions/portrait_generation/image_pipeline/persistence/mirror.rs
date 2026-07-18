use serde_json::json;

use a4n_orchestrator::generation::{
    GenerationRunCheckpointContext, GenerationRunTraceTarget, RecordCompletedGenerationRunInput,
    RecordCompletedGenerationRunWithCheckpointInput, generation_checkpoint_fingerprint_sha256,
};

use crate::AppState;
use crate::actions::app_error_message;

use super::super::contracts::PortraitImageMirror;

mod metadata;
mod orchestration;
mod record;
mod rollback;

use metadata::portrait_mirror_metadata;
pub(in crate::actions::portrait_generation) use orchestration::mirror_or_rollback;
use record::portrait_image_record;
pub(in crate::actions::portrait_generation) use rollback::rollback_failed_portrait_creation;

async fn mirror_portrait_to_generated_image(
    state: &AppState,
    input: PortraitImageMirror<'_>,
) -> Result<(), String> {
    let now = chrono::Utc::now().naive_utc();
    let metadata_json = portrait_mirror_metadata(&input, &now.to_string());
    let image_record = portrait_image_record(&input, metadata_json, now);
    if let Some(trace) = input.provider_trace {
        let media_refs_json = if input.reference_image_ids.is_empty() {
            None
        } else {
            Some(json!({
                "reference_image_ids": input.reference_image_ids}))
        };
        let input_variables_json = json!({
            "profile_id": input.profile_id,
            "character_portrait_id": input.portrait_id,
            "portrait_type": input.portrait_type,
            "expression": input.expression,
            "pose": input.pose,
            "variant_kind": input.variant_kind,
            "auto_generated": input.auto_generated});
        let source_refs_json = json!({
            "character_portrait_id": input.portrait_id,
            "profile_id": input.profile_id});
        let input_fingerprint_json = json!({
            "command": "generate_portrait_image",
            "project_id": input.project_id,
            "profile_id": input.profile_id,
            "portrait_type": input.portrait_type,
            "expression": input.expression,
            "pose": input.pose,
            "variant_kind": input.variant_kind,
            "auto_generated": input.auto_generated,
            "prompt": input.prompt,
            "width": input.width,
            "height": input.height,
            "provider": trace.provider,
            "model": input.model,
            "request": trace.request_json,
            "media_refs": media_refs_json.clone()});
        state
            .orchestrator
            .create_generated_image_record_with_generation_run_checkpoint(
                image_record,
                RecordCompletedGenerationRunWithCheckpointInput {
                    run: RecordCompletedGenerationRunInput {
                        project_id: input.project_id,
                        workflow: "portrait_generation".to_string(),
                        step_key: "generate_portrait_image".to_string(),
                        title: "人物立绘图片生成".to_string(),
                        model_kind: "image".to_string(),
                        provider: Some(trace.provider.to_string()),
                        model: Some(input.model.to_string()),
                        prompt_template_names_json: None,
                        prompt_template_hashes_json: None,
                        rendered_messages_json: None,
                        rendered_prompts_json: Some(json!({
                            "prompt": input.prompt})),
                        input_variables_json: Some(input_variables_json),
                        source_refs_json: Some(source_refs_json),
                        media_refs_json,
                        request_json: Some(trace.request_json.clone()),
                        response_summary_json: Some(json!({
                            "character_portrait_id": input.portrait_id,
                            "format": trace.output_format,
                            "file_size": input.file_size})),
                        targets: vec![GenerationRunTraceTarget {
                            target_kind: "character_portrait".to_string(),
                            target_table: "character_portrait".to_string(),
                            target_id: input.portrait_id,
                        }],
                    },
                    checkpoint: GenerationRunCheckpointContext {
                        workflow_run_id: input.workflow_run_id,
                        call_key: Some(format!(
                            "generate_portrait_image:profile:{}:{}:{}:{}",
                            input.profile_id,
                            input.portrait_type,
                            input.expression.unwrap_or("none"),
                            input.pose.unwrap_or("none")
                        )),
                        input_fingerprint_sha256: Some(
                            generation_checkpoint_fingerprint_sha256(&input_fingerprint_json)
                                .map_err(|err| err.to_string())?,
                        ),
                        input_fingerprint_json: Some(input_fingerprint_json),
                        raw_response_json: Some(json!({
                            "format": trace.output_format,
                            "model": input.model,
                            "file_size": input.file_size})),
                        parsed_output_json: Some(json!({
                            "character_portrait_id": input.portrait_id,
                            "profile_id": input.profile_id,
                            "portrait_type": input.portrait_type,
                            "image_path": input.image_path,
                            "width": input.width,
                            "height": input.height})),
                        validation_json: None,
                        artifact_refs_json: Some(json!({
                            "targets": [{
                                "target_kind": "character_portrait",
                                "target_table": "character_portrait",
                                "target_id": input.portrait_id}]})),
                        attempt: None,
                    },
                },
            )
            .await
            .map_err(app_error_message)?;
    } else {
        state
            .orchestrator
            .create_generated_image_record(image_record)
            .await
            .map_err(app_error_message)?;
    }
    Ok(())
}
