//! 章节视频提示词工作流指纹与 begin 输入纯规则（无 DB）。

use a4n_core::source_spans::source_text_hash as source_content_sha256;
use serde_json::{Value, json};

use super::{ChapterVideoPromptOptions, ChapterVideoSource};
use crate::workflow::{UpsertWorkflowRunInput, WORKFLOW_STATUS_RUNNING};

/// 章节视频提示词 exclusive workflow business key。
pub fn video_prompt_workflow_business_key(
    workflow_kind: &str,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: &str,
) -> String {
    format!("video_prompt:{workflow_kind}:{project_id}:{chapter_id}:{workflow_mode}")
}

/// 章节视频提示词工作流输入指纹。
pub fn video_prompt_workflow_fingerprint(
    workflow_kind: &str,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: &str,
    source: &ChapterVideoSource,
    options: &ChapterVideoPromptOptions,
) -> Value {
    json!({
        "schema_version": 1,
        "workflow_kind": workflow_kind,
        "project_id": project_id,
        "chapter_id": chapter_id,
        "workflow_mode": workflow_mode,
        "source": {
            "content_sha256": source_content_sha256(&source.content),
            "content_chars": source.content.chars().count(),
            "summary": source.summary,
            "analysis": source.analysis,
        },
        "options": {
            "style": &options.style,
            "prompt_output_language": options.prompt_output_language.as_str(),
            "prompt_batch_size": options.prompt_batch_size,
            "duration_secs": options.duration_secs,
            "duration_secs_options": &options.duration_secs_options,
            "aspect_ratio": &options.aspect_ratio,
            "reference_image_ids": &options.reference_image_ids,
            "video_library_image_ids": &options.video_library_image_ids,
            "duration_capability": &options.duration_capability,
            "regenerate_existing": options.regenerate_existing,
        },
    })
}

/// 构造章节视频提示词 begin 输入。
pub fn video_prompt_workflow_run_input(
    workflow_kind: &str,
    project_id: i64,
    chapter_id: i64,
    workflow_mode: &str,
    workflow_step_key: &str,
    source: &ChapterVideoSource,
    options: &ChapterVideoPromptOptions,
) -> Result<(UpsertWorkflowRunInput, Value), String> {
    if project_id <= 0 {
        return Err("项目 ID 必须大于 0".to_string());
    }
    if chapter_id <= 0 {
        return Err("章节 ID 必须大于 0".to_string());
    }
    if workflow_kind.trim().is_empty() {
        return Err("workflow_kind 不能为空".to_string());
    }
    if workflow_mode.trim().is_empty() {
        return Err("workflow_mode 不能为空".to_string());
    }
    if workflow_step_key.trim().is_empty() {
        return Err("workflow_step_key 不能为空".to_string());
    }
    let fingerprint = video_prompt_workflow_fingerprint(
        workflow_kind,
        project_id,
        chapter_id,
        workflow_mode,
        source,
        options,
    );
    let input = UpsertWorkflowRunInput {
        business_key: video_prompt_workflow_business_key(
            workflow_kind,
            project_id,
            chapter_id,
            workflow_mode,
        ),
        workflow_kind: workflow_kind.to_string(),
        project_id: Some(project_id),
        root_project_id: None,
        source_snapshot_id: None,
        scope_table: Some("chapter".to_string()),
        scope_id: Some(chapter_id),
        chapter_id: Some(chapter_id),
        mode: Some(workflow_mode.to_string()),
        input_fingerprint_json: Some(fingerprint.clone()),
        status: WORKFLOW_STATUS_RUNNING.to_string(),
        current_step_key: Some(workflow_step_key.to_string()),
        active_task_id: None,
        can_resume: false,
        resume_block_reason: None,
        error_message: None,
    };
    Ok((input, fingerprint))
}

/// 视频提示词优化 exclusive workflow business key。
pub fn video_prompt_optimization_business_key(project_id: i64, prompt_id: i64) -> String {
    format!("video_prompt_optimization:{project_id}:{prompt_id}")
}

/// 构造视频提示词优化 begin 输入。
pub fn video_prompt_optimization_workflow_run_input(
    project_id: i64,
    prompt_id: i64,
    chapter_id: i64,
    workflow_mode: &str,
    current_step_key: &str,
    input_fingerprint_json: Value,
) -> Result<UpsertWorkflowRunInput, String> {
    if project_id <= 0 {
        return Err("项目 ID 必须大于 0".to_string());
    }
    if prompt_id <= 0 {
        return Err("提示词 ID 必须大于 0".to_string());
    }
    if chapter_id <= 0 {
        return Err("章节 ID 必须大于 0".to_string());
    }
    if current_step_key.trim().is_empty() {
        return Err("current_step_key 不能为空".to_string());
    }
    Ok(UpsertWorkflowRunInput {
        business_key: video_prompt_optimization_business_key(project_id, prompt_id),
        workflow_kind: "video_prompt_optimization".to_string(),
        project_id: Some(project_id),
        root_project_id: None,
        source_snapshot_id: None,
        scope_table: Some("chapter_video_prompt".to_string()),
        scope_id: Some(prompt_id),
        chapter_id: Some(chapter_id),
        mode: Some(workflow_mode.to_string()),
        input_fingerprint_json: Some(input_fingerprint_json),
        status: WORKFLOW_STATUS_RUNNING.to_string(),
        current_step_key: Some(current_step_key.to_string()),
        active_task_id: None,
        can_resume: true,
        resume_block_reason: None,
        error_message: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_core::prompt_language::PromptOutputLanguage;

    fn sample_options() -> ChapterVideoPromptOptions {
        ChapterVideoPromptOptions {
            style: "anime".into(),
            prompt_output_language: PromptOutputLanguage::ZhCn,
            prompt_batch_size: 1,
            target_shot_number: None,
            duration_secs: 5,
            duration_secs_options: vec![5],
            aspect_ratio: "16:9".into(),
            reference_image_ids: vec![],
            video_library_image_ids: vec![],
            duration_capability: None,
            regenerate_existing: false,
            workflow_run_id: None,
        }
    }

    #[test]
    fn business_key_includes_mode() {
        assert_eq!(
            video_prompt_workflow_business_key("storyboard", 1, 2, "storyboard"),
            "video_prompt:storyboard:1:2:storyboard"
        );
    }

    #[test]
    fn begin_input_requires_positive_ids() {
        let source = ChapterVideoSource {
            content: "hi".into(),
            summary: None,
            analysis: None,
        };
        assert!(
            video_prompt_workflow_run_input("k", 0, 1, "m", "s", &source, &sample_options())
                .is_err()
        );
        let (input, fp) =
            video_prompt_workflow_run_input("k", 1, 2, "m", "s", &source, &sample_options())
                .expect("ok");
        assert_eq!(input.status, WORKFLOW_STATUS_RUNNING);
        assert_eq!(fp["chapter_id"], 2);
    }
}
