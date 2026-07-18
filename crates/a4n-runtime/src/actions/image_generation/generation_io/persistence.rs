use super::contracts::GeneratedImageSaveInput;
use super::files::persist_generated_image_file;
use super::metadata::{
    ImageGenerationTrace, attach_image_generation_trace, image_generation_media_refs,
};
use crate::AppState;
use crate::file_utils::remove_created_file_on_error;
use a4n_core::models::ai::generated_image;
use a4n_image::ImageGenerationResult;
use a4n_orchestrator::generation::{
    GenerationRunCheckpointContext, RecordCompletedGenerationRunInput,
    RecordCompletedGenerationRunWithCheckpointInput, generation_checkpoint_fingerprint_sha256,
};
use a4n_orchestrator::video::CreateGeneratedImageRecordDraftInput as CreateImageRecord;
use serde_json::json;

pub(in crate::actions::image_generation) async fn save_generated_image_result(
    state: &AppState,
    result: ImageGenerationResult,
    input: GeneratedImageSaveInput<'_>,
) -> Result<generated_image::Model, String> {
    let command = input.command.to_string();
    let provider = input.config.backend.clone();
    let request_json = input.request_json;
    let output_format = result.format.clone();
    let metadata_json = attach_image_generation_trace(
        input.metadata_json,
        ImageGenerationTrace {
            command: input.command,
            config: input.config,
            image_type: &input.image_type,
            prompt: &input.prompt,
            provider_model: &result.model,
            chapter_id: input.chapter_id,
            panel_id: input.panel_id.as_deref(),
            width: input.width,
            height: input.height,
            generated_at: input.created_at,
        },
    )?;
    let media_refs_json = metadata_json.as_ref().and_then(image_generation_media_refs);
    let image_type = input.image_type;
    let prompt = input.prompt;
    let model = result.model;
    let chapter_id = input.chapter_id;
    let panel_id = input.panel_id.clone();
    let call_key = format!(
        "{}:{}:chapter:{}:panel:{}",
        command,
        image_type,
        chapter_id
            .map(|value| value.to_string())
            .unwrap_or_else(|| "none".to_string()),
        panel_id.as_deref().unwrap_or("none")
    );
    let input_fingerprint_json = json!({
        "command": command.clone(),
        "project_id": input.project_id,
        "chapter_id": chapter_id,
        "panel_id": panel_id,
        "image_type": image_type.clone(),
        "prompt": prompt.clone(),
        "width": input.width,
        "height": input.height,
        "provider": provider.clone(),
        "model": model.clone(),
        "request": request_json.clone(),
        "media_refs": media_refs_json.clone(),
    });
    let input_fingerprint_sha256 =
        generation_checkpoint_fingerprint_sha256(&input_fingerprint_json)
            .map_err(|err| err.to_string())?;
    let (path, file_path, file_size) = persist_generated_image_file(
        &state.config.data_dir,
        input.project_id,
        input.chapter_id,
        &image_type,
        input.file_extra.as_deref(),
        &result.bytes,
        &result.format,
        input.file_context,
    )?;
    remove_created_file_on_error(&path, "已写出的图片文件", async move {
        let image_record = CreateImageRecord {
            project_id: input.project_id,
            image_type: image_type.clone(),
            file_path,
            prompt: prompt.clone(),
            model: model.clone(),
            width: input.width,
            height: input.height,
            file_size: Some(file_size),
            chapter_id,
            panel_id: input.panel_id,
            metadata_json,
            created_at: Some(input.created_at),
        };
        state
            .orchestrator
            .create_generated_image_record_with_generation_run_checkpoint(
                image_record,
                RecordCompletedGenerationRunWithCheckpointInput {
                    run: RecordCompletedGenerationRunInput {
                        project_id: input.project_id,
                        workflow: "image_generation".to_string(),
                        step_key: command,
                        title: format!("{image_type} 图片 provider 生成"),
                        model_kind: "image".to_string(),
                        provider: Some(provider),
                        model: Some(model.clone()),
                        prompt_template_names_json: None,
                        prompt_template_hashes_json: None,
                        rendered_messages_json: None,
                        rendered_prompts_json: Some(json!({
                            "prompt": prompt,
                        })),
                        input_variables_json: None,
                        source_refs_json: None,
                        media_refs_json: media_refs_json.clone(),
                        request_json: Some(request_json),
                        response_summary_json: Some(json!({
                            "format": output_format.clone(),
                            "file_size": file_size,
                        })),
                        targets: Vec::new(),
                    },
                    checkpoint: GenerationRunCheckpointContext {
                        workflow_run_id: input.workflow_run_id,
                        call_key: Some(call_key),
                        input_fingerprint_sha256: Some(input_fingerprint_sha256),
                        input_fingerprint_json: Some(input_fingerprint_json),
                        raw_response_json: Some(json!({
                            "format": output_format,
                            "model": model,
                            "file_size": file_size,
                        })),
                        parsed_output_json: None,
                        validation_json: None,
                        artifact_refs_json: None,
                        attempt: None,
                    },
                },
            )
            .await
            .map_err(|err| err.to_string())
    })
    .await
}
