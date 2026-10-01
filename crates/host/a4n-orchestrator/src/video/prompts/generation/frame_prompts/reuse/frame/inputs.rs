use a4n_domain::video_prompt::ChapterVideoPromptRecord;
use std::collections::{BTreeMap, HashMap, HashSet};

use a4n_domain::prompt_language::PromptOutputLanguage;
use a4n_domain::prompt_language::metadata_matches_prompt_output_language;
use a4n_error::AppError;
use serde_json::Value;

use crate::video::OrchestratorContext;
use crate::video::prompts::json::{VideoFramePrompt, VideoSegmentPlan};
use crate::video::prompts::store::PromptWorkflowMode;

pub(in crate::video::prompts::generation::frame_prompts) async fn reusable_frame_rows_for_video_prompt_inputs(
    service: &OrchestratorContext,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: PromptWorkflowMode,
    prompt_type: &str,
    source_prompt_type: &str,
    video_execution_identity: &Value,
    prompt_output_language: PromptOutputLanguage,
) -> Result<Vec<ChapterVideoPromptRecord>, AppError> {
    let rows = a4n_db::video::read_models::chapter_video_prompt_workflow_records(
        service.database.connection(),
        project_id,
        chapter_id,
        prompt_type,
        workflow_mode.as_str(),
        true,
    )
    .await
    .map_err(AppError::Database)?;

    let mut by_frame = BTreeMap::<i32, ChapterVideoPromptRecord>::new();
    for row in rows {
        if !frame_prompt_row_matches_video_input_contract(
            &row,
            workflow_mode,
            prompt_type,
            source_prompt_type,
            video_execution_identity,
        ) {
            continue;
        }
        let row_matches_requested_language = metadata_matches_prompt_output_language(
            row.metadata_json.as_ref(),
            prompt_output_language,
        );
        match by_frame.get(&row.shot_number) {
            Some(existing)
                if row_matches_requested_language
                    && !metadata_matches_prompt_output_language(
                        existing.metadata_json.as_ref(),
                        prompt_output_language,
                    ) =>
            {
                by_frame.insert(row.shot_number, row);
            }
            Some(_) => {}
            None => {
                by_frame.insert(row.shot_number, row);
            }
        }
    }
    Ok(by_frame.into_values().collect())
}

pub(super) fn frame_prompt_row_matches_video_input_contract(
    row: &ChapterVideoPromptRecord,
    workflow_mode: PromptWorkflowMode,
    prompt_type: &str,
    source_prompt_type: &str,
    video_execution_identity: &Value,
) -> bool {
    if row.workflow_mode != workflow_mode.as_str() || row.prompt_type != prompt_type {
        return false;
    }
    let Some(metadata) = row.metadata_json.as_ref() else {
        return false;
    };
    let Some(expected_contract) = crate::video::prompts::records::expected_contract_version(
        workflow_mode.as_str(),
        prompt_type,
    ) else {
        return false;
    };
    if metadata.get("contract_version").and_then(Value::as_str) != Some(expected_contract) {
        return false;
    }
    if !crate::video::prompts::records::video_execution_segment_identity_matches(
        Some(metadata),
        video_execution_identity,
    ) {
        return false;
    }
    metadata.get("source_prompt_type").and_then(Value::as_str) == Some(source_prompt_type)
}

pub(in crate::video::prompts::generation::frame_prompts) fn validate_frame_prompts_for_video_prompt_inputs(
    frame_prompts: Vec<VideoFramePrompt>,
    segment_plan: &[VideoSegmentPlan],
    allowed_reference_ids: &HashSet<i64>,
    label: &str,
) -> Result<Vec<VideoFramePrompt>, AppError> {
    let segment_by_number = segment_plan
        .iter()
        .map(|segment| (segment.segment_number, segment))
        .collect::<HashMap<_, _>>();
    for frame in &frame_prompts {
        let Some(first_segment_number) = frame.segment_numbers.first().copied() else {
            return Err(AppError::InvalidArgument(format!(
                "已保存{label} #{} 缺少 segment_numbers，不能作为视频提示词输入",
                frame.frame_number
            )));
        };
        let Some(segment) = segment_by_number.get(&first_segment_number).copied() else {
            return Err(AppError::InvalidArgument(format!(
                "已保存{label} #{} 引用了不存在的片段 {}",
                frame.frame_number, first_segment_number
            )));
        };
        if frame.aspect_ratio != segment.aspect_ratio {
            return Err(AppError::InvalidArgument(format!(
                "已保存{label} #{} aspect_ratio 必须等于片段 {} 的画幅 {}，实际为 {}；请重新生成{}",
                frame.frame_number,
                first_segment_number,
                segment.aspect_ratio,
                frame.aspect_ratio,
                label
            )));
        }
        if let Some(image_id) = frame
            .reference_image_ids
            .iter()
            .find(|image_id| !allowed_reference_ids.contains(image_id))
        {
            return Err(AppError::InvalidArgument(format!(
                "已保存{label} #{} 引用了当前上下文不可用的图片 ID：{}；请重新生成{}",
                frame.frame_number, image_id, label
            )));
        }
    }
    Ok(frame_prompts)
}
