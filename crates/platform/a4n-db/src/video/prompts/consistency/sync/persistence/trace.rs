use a4n_db_models::models::chapter::chapter_video_prompt;
use sea_orm::ConnectionTrait;
use serde_json::json;

use crate::video::VideoDbError;
use a4n_domain::video_prompt::PreparedVideoPromptGenerationTrace;

pub(in crate::video::prompts::consistency::sync) async fn record_chapter_video_prompt_generation_run<
    C,
>(
    db: &C,
    row: &chapter_video_prompt::Model,
    trace: PreparedVideoPromptGenerationTrace,
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    let source_refs_json = row
        .metadata_json
        .as_ref()
        .and_then(|metadata| metadata.get("source"))
        .cloned()
        .map(|source| json!({ "source": source }));
    let media_refs_json = row.reference_image_ids_json.as_ref().map(|references| {
        json!({
            "reference_image_ids": references,
        })
    });
    let call_key = format!(
        "{}:{}:{}:{}",
        trace.prompt_name, row.workflow_mode, row.prompt_type, row.shot_number
    );
    let input_fingerprint_json = json!({
        "prompt_name": trace.prompt_name.clone(),
        "workflow_mode": row.workflow_mode.clone(),
        "prompt_type": row.prompt_type.clone(),
        "shot_number": row.shot_number,
        "chapter_id": row.chapter_id,
        "request": trace.request_json.clone(),
        "source_refs": source_refs_json.clone(),
        "media_refs": media_refs_json.clone(),
    });
    let input_fingerprint_sha256 =
        crate::generation::generation_checkpoint_fingerprint_sha256(&input_fingerprint_json)
            .map_err(|err| {
                VideoDbError::InvalidArgument(format!(
                    "视频提示词 generation checkpoint 指纹序列化失败: {err}"
                ))
            })?;
    let output_json = json!({
        "chapter_video_prompt_id": row.id,
        "chapter_id": row.chapter_id,
        "workflow_mode": row.workflow_mode.clone(),
        "prompt_type": row.prompt_type.clone(),
        "shot_number": row.shot_number,
        "title": row.title.clone(),
        "scene_summary": row.scene_summary.clone(),
        "visual_prompt": row.visual_prompt.clone(),
        "motion_prompt": row.motion_prompt.clone(),
        "negative_prompt": row.negative_prompt.clone(),
        "duration_secs": row.duration_secs,
        "aspect_ratio": row.aspect_ratio.clone(),
        "reference_image_ids": row.reference_image_ids_json.clone(),
        "metadata": row.metadata_json.clone(),
    });
    let artifact_refs_json = json!({
        "targets": [{
            "target_kind": row.prompt_type.clone(),
            "target_table": "chapter_video_prompt",
            "target_id": row.id,
        }],
    });
    crate::generation::record_completed_generation_run_with_checkpoint_in(
        db,
        crate::generation::RecordCompletedGenerationRunInput {
            project_id: row.project_id,
            workflow: "video_prompt_generation".to_string(),
            step_key: trace.prompt_name.clone(),
            title: format!("视频提示词生成: {}", row.prompt_type),
            model_kind: "llm".to_string(),
            provider: None,
            model: None,
            prompt_template_names_json: Some(json!([trace.prompt_name])),
            prompt_template_hashes_json: None,
            rendered_messages_json: Some(trace.rendered_messages_json),
            rendered_prompts_json: None,
            input_variables_json: None,
            source_refs_json: source_refs_json.clone(),
            media_refs_json: media_refs_json.clone(),
            request_json: Some(trace.request_json),
            response_summary_json: Some(json!({
                "chapter_video_prompt_id": row.id,
                "prompt_type": row.prompt_type,
                "shot_number": row.shot_number,
                "output": trace.response_text.clone(),
                "llm_usage": trace.usage_json.clone(),
            })),
            targets: vec![crate::generation::GenerationRunTraceTarget {
                target_kind: row.prompt_type.clone(),
                target_table: "chapter_video_prompt".to_string(),
                target_id: row.id,
            }],
        },
        crate::generation::GenerationRunCheckpointContext {
            workflow_run_id: trace.workflow_run_id,
            call_key: Some(call_key),
            input_fingerprint_sha256: Some(input_fingerprint_sha256),
            input_fingerprint_json: Some(input_fingerprint_json),
            raw_response_json: Some(json!({
                "text": trace.response_text,
                "usage": trace.usage_json,
            })),
            parsed_output_json: Some(output_json),
            validation_json: None,
            artifact_refs_json: Some(artifact_refs_json),
            attempt: None,
        },
    )
    .await?;
    Ok(())
}
