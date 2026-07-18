use a4n_orchestrator::video::{
    CreateGeneratedVideoRecordDraftInput as CreateVideoRecord, RecordStartedGenerationRunInput,
};
use serde_json::{Map, Value, json};

pub(in crate::actions::video_generation) fn started_video_generation_run_input(
    video: &CreateVideoRecord,
    step_key: &str,
    title: &str,
    provider: String,
    prompt_template_name: Option<&str>,
    request_json: Value,
) -> RecordStartedGenerationRunInput {
    let prompt_template_names_json = prompt_template_name.map(|name| json!([name]));
    RecordStartedGenerationRunInput {
        project_id: video.project_id,
        workflow: "video_generation".to_string(),
        step_key: step_key.to_string(),
        title: title.to_string(),
        model_kind: "video".to_string(),
        provider: Some(provider),
        model: Some(video.model.clone()),
        prompt_template_names_json,
        prompt_template_hashes_json: None,
        rendered_messages_json: None,
        rendered_prompts_json: Some(json!({
            "prompt": video.prompt,
            "negative_prompt": video.negative_prompt,
        })),
        input_variables_json: None,
        source_refs_json: video_generation_source_refs(video.metadata_json.as_ref()),
        media_refs_json: video_generation_media_refs(video.metadata_json.as_ref()),
        request_json: Some(request_json),
        targets: Vec::new(),
    }
}

fn video_generation_source_refs(metadata_json: Option<&Value>) -> Option<Value> {
    metadata_json
        .and_then(|metadata| metadata.get("source"))
        .cloned()
        .map(|source| json!({ "source": source }))
}

fn video_generation_media_refs(metadata_json: Option<&Value>) -> Option<Value> {
    let metadata = metadata_json?;
    let mut media = Map::new();
    copy_json_field(metadata, &mut media, "material_reference_image_ids");
    copy_json_field(metadata, &mut media, "material_reference_image_count");
    copy_json_field(metadata, &mut media, "prompt_reference_image_ids");
    copy_json_field(metadata, &mut media, "prompt_reference_image_count");
    copy_json_field(metadata, &mut media, "provider_reference_image_ids");
    copy_json_field(metadata, &mut media, "provider_reference_image_count");
    copy_json_field(metadata, &mut media, "start_frame_image_id");
    copy_json_field(metadata, &mut media, "boundary_frame_image_ids");
    copy_json_field(metadata, &mut media, "boundary_frame_image_count");
    copy_json_field(metadata, &mut media, "reference_media");
    if media.is_empty() {
        None
    } else {
        Some(Value::Object(media))
    }
}

fn copy_json_field(source: &Value, target: &mut Map<String, Value>, key: &str) {
    if let Some(value) = source.get(key) {
        target.insert(key.to_string(), value.clone());
    }
}
