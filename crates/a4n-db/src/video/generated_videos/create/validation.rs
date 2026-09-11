use super::super::CreateGeneratedVideoRecordInput;
use crate::video::{
    CreateGeneratedVideoRecordDraftInput, VideoDbError,
    ensure_video_chapter_belongs_to_project_record, generated_image_by_id, generated_video_by_id,
    validate_positive_video_i64,
};
use a4n_domain::video_workflow::is_generated_video_workflow_mode;
use sea_orm::ConnectionTrait;

pub(super) async fn validate_create_generated_video_record_input<C>(
    db: &C,
    input: CreateGeneratedVideoRecordDraftInput,
) -> Result<CreateGeneratedVideoRecordInput, VideoDbError>
where
    C: ConnectionTrait,
{
    let project_id = validate_positive_video_i64(input.project_id, "project_id")?;
    if let Some(chapter_id) = input.chapter_id {
        let chapter_id = validate_positive_video_i64(chapter_id, "chapter_id")?;
        ensure_video_chapter_belongs_to_project_record(db, project_id, chapter_id).await?;
    } else {
        crate::video::generated_images::video_project_record(db, project_id).await?;
    }
    if let Some(source_video_id) = input.source_video_id {
        let source_video_id = validate_positive_video_i64(source_video_id, "source_video_id")?;
        let source = generated_video_by_id(db, source_video_id)
            .await
            .map_err(VideoDbError::Database)?
            .ok_or_else(|| VideoDbError::NotFound("视频不存在".into()))?;
        if source.project_id != project_id {
            return Err(VideoDbError::InvalidArgument(format!(
                "源视频 #{source_video_id} 不属于项目 {project_id}"
            )));
        }
    }
    if let Some(source_image_id) = input.source_image_id {
        let source_image_id = validate_positive_video_i64(source_image_id, "source_image_id")?;
        let source = generated_image_by_id(db, source_image_id)
            .await
            .map_err(VideoDbError::Database)?
            .ok_or_else(|| VideoDbError::NotFound(format!("源图片不存在: {source_image_id}")))?;
        if source.project_id != project_id {
            return Err(VideoDbError::InvalidArgument(format!(
                "源图片 #{source_image_id} 不属于项目 {project_id}"
            )));
        }
    }
    let workflow_mode = validate_video_workflow_mode(input.workflow_mode)?;
    Ok(CreateGeneratedVideoRecordInput {
        project_id,
        chapter_id: input.chapter_id,
        video_type: input.video_type,
        workflow_mode,
        prompt: input.prompt,
        negative_prompt: input.negative_prompt,
        model: input.model,
        width: input.width,
        height: input.height,
        duration_secs: input.duration_secs,
        fps: input.fps,
        source_video_id: input.source_video_id,
        source_image_id: input.source_image_id,
        provider_job_id: normalize_video_provider_job_id(input.provider_job_id.as_deref())?,
        metadata_json: input.metadata_json,
    })
}

fn validate_video_workflow_mode(workflow_mode: Option<String>) -> Result<String, VideoDbError> {
    let Some(mode) = workflow_mode
        .map(|mode| mode.trim().to_string())
        .filter(|mode| !mode.is_empty())
    else {
        return Err(VideoDbError::InvalidArgument(
            "视频记录缺少 workflow_mode".to_string(),
        ));
    };
    if is_generated_video_workflow_mode(&mode) {
        Ok(mode)
    } else {
        Err(VideoDbError::InvalidArgument(format!(
            "视频记录 workflow_mode 无效: {mode}"
        )))
    }
}

fn normalize_video_provider_job_id(value: Option<&str>) -> Result<Option<String>, VideoDbError> {
    value
        .map(str::trim)
        .map(|value| {
            if value.is_empty() {
                Err(VideoDbError::InvalidArgument(
                    "provider_job_id 不能为空".to_string(),
                ))
            } else {
                Ok(value.to_string())
            }
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_domain::video_workflow::{
        WORKFLOW_MODE_CANVAS_ASSEMBLY, WORKFLOW_MODE_CANVAS_TRIM, WORKFLOW_MODE_CANVAS_VIDEO,
        WORKFLOW_MODE_MIXED,
    };

    #[test]
    fn generated_video_workflow_mode_accepts_current_modes_and_rejects_missing_or_unknown() {
        for mode in [
            "boundary_frames",
            "first_frame_chain",
            "reference_media",
            "chat_video",
            "storyboard",
            WORKFLOW_MODE_MIXED,
            WORKFLOW_MODE_CANVAS_VIDEO,
            WORKFLOW_MODE_CANVAS_TRIM,
            WORKFLOW_MODE_CANVAS_ASSEMBLY,
        ] {
            assert_eq!(
                validate_video_workflow_mode(Some(format!("  {mode}  ")))
                    .expect("current workflow mode"),
                mode
            );
        }

        assert!(matches!(
            validate_video_workflow_mode(None),
            Err(VideoDbError::InvalidArgument(message))
                if message == "视频记录缺少 workflow_mode"
        ));
        assert!(matches!(
            validate_video_workflow_mode(Some("legacy".to_string())),
            Err(VideoDbError::InvalidArgument(message))
                if message == "视频记录 workflow_mode 无效: legacy"
        ));
    }

    #[test]
    fn generated_video_provider_job_id_normalizes_or_rejects_blank_values() {
        assert_eq!(normalize_video_provider_job_id(None).unwrap(), None);
        assert_eq!(
            normalize_video_provider_job_id(Some("  job-7  ")).unwrap(),
            Some("job-7".to_string())
        );
        assert!(matches!(
            normalize_video_provider_job_id(Some("  ")),
            Err(VideoDbError::InvalidArgument(message))
                if message == "provider_job_id 不能为空"
        ));
    }
}
