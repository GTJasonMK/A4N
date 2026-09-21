//! 视频 provider 生成开始时 generation_run 账本纯组装。

use a4n_db::generation::RecordStartedGenerationRunInput;
use a4n_domain::media::CreateGeneratedVideoRecordDraftInput;
use serde_json::{Map, Value, json};

/// 组装「视频 provider 生成已开始」的 generation_run 输入。
pub fn started_video_generation_run_input(
    video: &CreateGeneratedVideoRecordDraftInput,
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

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_domain::media::CreateGeneratedVideoRecordDraftInput;
    fn draft() -> CreateGeneratedVideoRecordDraftInput {
        CreateGeneratedVideoRecordDraftInput {
            project_id: 7,
            chapter_id: Some(1),
            video_type: a4n_domain::video_workflow::VIDEO_TYPE_CHAPTER.into(),
            workflow_mode: Some("boundary_frames".into()),
            prompt: "p".into(),
            negative_prompt: None,
            model: "m".into(),
            width: 1280,
            height: 720,
            duration_secs: 4.0,
            fps: 24,
            source_video_id: None,
            source_image_id: None,
            provider_job_id: None,
            metadata_json: Some(json!({
                "source": {"kind": "prompt"},
                "prompt_reference_image_ids": [1, 2],
            })),
        }
    }

    #[test]
    fn started_run_copies_project_model_and_media_refs() {
        let input = started_video_generation_run_input(
            &draft(),
            "generate_chapter_video",
            "章节视频 provider 生成",
            "backend".into(),
            Some("chapter_video_prompt"),
            json!({"preview": true}),
        );
        assert_eq!(input.project_id, 7);
        assert_eq!(input.workflow, "video_generation");
        assert_eq!(input.step_key, "generate_chapter_video");
        assert_eq!(input.model_kind, "video");
        assert_eq!(input.provider.as_deref(), Some("backend"));
        assert_eq!(input.model.as_deref(), Some("m"));
        assert!(input.source_refs_json.is_some());
        assert!(input.media_refs_json.is_some());
        assert_eq!(
            input
                .prompt_template_names_json
                .as_ref()
                .and_then(|v| v.as_array())
                .map(|a| a.len()),
            Some(1)
        );
    }
}
