use crate::generation::{
    GenerationRunTraceTarget, RecordStartedGenerationRunInput, record_started_generation_run,
};
use crate::video::{CreateGeneratedVideoRecordInput, VideoDbError};
use a4n_core::models::ai::generated_video;
use sea_orm::ConnectionTrait;

pub(super) fn validate_generation_run_for_video(
    input: &CreateGeneratedVideoRecordInput,
    generation_run: &RecordStartedGenerationRunInput,
) -> Result<(), VideoDbError> {
    if generation_run.project_id != input.project_id {
        return Err(VideoDbError::InvalidArgument(
            "视频生成运行记录 project_id 与视频资产不一致".to_string(),
        ));
    }
    if !generation_run.targets.is_empty() {
        return Err(VideoDbError::InvalidArgument(
            "视频生成运行记录目标由视频服务绑定，调用方不能预填 targets".to_string(),
        ));
    }
    Ok(())
}

pub(super) async fn record_video_generation_run<C>(
    db: &C,
    video: &generated_video::Model,
    mut generation_run: RecordStartedGenerationRunInput,
) -> Result<(), VideoDbError>
where
    C: ConnectionTrait,
{
    generation_run.targets = vec![GenerationRunTraceTarget {
        target_kind: video.video_type.clone(),
        target_table: "generated_video".to_string(),
        target_id: video.id,
    }];
    record_started_generation_run(db, generation_run)
        .await
        .map_err(VideoDbError::from)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn video_input(project_id: i64) -> CreateGeneratedVideoRecordInput {
        CreateGeneratedVideoRecordInput {
            project_id,
            chapter_id: None,
            video_type: generated_video::VIDEO_TYPE_CANVAS.to_string(),
            workflow_mode: a4n_core::video_workflow::WORKFLOW_MODE_CANVAS_VIDEO.to_string(),
            prompt: "prompt".to_string(),
            negative_prompt: None,
            model: "test".to_string(),
            width: 1280,
            height: 720,
            duration_secs: 5.0,
            fps: 24,
            source_video_id: None,
            source_image_id: None,
            provider_job_id: None,
            metadata_json: None,
        }
    }

    fn generation_run(project_id: i64) -> RecordStartedGenerationRunInput {
        RecordStartedGenerationRunInput {
            project_id,
            workflow: "video_generation".to_string(),
            step_key: "provider_request".to_string(),
            title: "视频生成".to_string(),
            model_kind: "video".to_string(),
            provider: None,
            model: None,
            prompt_template_names_json: None,
            prompt_template_hashes_json: None,
            rendered_messages_json: None,
            rendered_prompts_json: None,
            input_variables_json: None,
            source_refs_json: None,
            media_refs_json: None,
            request_json: None,
            targets: Vec::new(),
        }
    }

    #[test]
    fn generated_video_generation_run_rejects_scope_mismatch_or_prefilled_targets() {
        let input = video_input(7);
        let error = validate_generation_run_for_video(&input, &generation_run(8)).unwrap_err();
        assert!(matches!(
            error,
            VideoDbError::InvalidArgument(message)
                if message == "视频生成运行记录 project_id 与视频资产不一致"
        ));

        let mut run = generation_run(7);
        run.targets.push(GenerationRunTraceTarget {
            target_kind: "generated_video".to_string(),
            target_table: "generated_video".to_string(),
            target_id: 1,
        });
        let error = validate_generation_run_for_video(&input, &run).unwrap_err();
        assert!(matches!(
            error,
            VideoDbError::InvalidArgument(message)
                if message == "视频生成运行记录目标由视频服务绑定，调用方不能预填 targets"
        ));
    }
}
