use super::*;

pub(crate) async fn resolve_project_chapter_video_source(
    state: &AppState,
    project_id: i64,
    chapter_id: i64,
    operation: &str,
    writable: bool,
) -> Result<ProjectChapterVideoSource, String> {
    state
        .orchestrator
        .project_chapter_video_source(project_id, chapter_id, operation, writable)
        .await
        .map_err(crate::actions::app_error_message)
}

pub(super) fn source_context(source: &ChapterVideoSource) -> ChapterVideoSourceContext<'_> {
    ChapterVideoSourceContext {
        content: &source.content,
        summary: source.summary.as_deref(),
        analysis: source.analysis.as_ref(),
    }
}

pub(super) fn build_video_prompt_vars(
    project_id: i64,
    chapter_id: i64,
    chapter: &chapter::Model,
    source: &ChapterVideoSource,
    prompt_options: &ChapterVideoPromptOptions,
) -> Result<HashMap<String, Value>, String> {
    let mut vars = HashMap::new();
    vars.insert("project_id".into(), json!(project_id));
    vars.insert("chapter_id".into(), json!(chapter_id));
    vars.insert("chapter_number".into(), json!(chapter.chapter_number));
    vars.insert(
        "chapter_title".into(),
        json!(required_chapter_title(
            chapter.title.as_deref(),
            chapter.chapter_number,
            "生成视频提示词",
        )?),
    );
    vars.insert("chapter_status".into(), json!(&chapter.status));
    vars.insert(
        "source_content_sha256".into(),
        json!(source_content_sha256(&source.content)),
    );
    if let Some(summary) = source.summary.as_deref().filter(|s| !s.trim().is_empty()) {
        vars.insert("chapter_summary".into(), json!(summary));
    }
    if let Some(analysis) = source.analysis.as_ref() {
        vars.insert("chapter_analysis".into(), analysis.clone());
        if let Some(compressed) = analysis
            .get("summaries")
            .and_then(|v| v.get("compressed"))
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            vars.insert("chapter_analysis_summary".into(), json!(compressed));
        }
    }
    vars.insert("style".into(), json!(&prompt_options.style));
    vars.insert(
        "prompt_output_language".into(),
        json!(prompt_options.prompt_output_language.as_str()),
    );
    vars.insert("duration_secs".into(), json!(prompt_options.duration_secs));
    vars.insert(
        "duration_secs_options".into(),
        json!(&prompt_options.duration_secs_options),
    );
    if let Some(capability) = prompt_options.duration_capability.as_ref() {
        vars.insert(
            "video_duration_capability".into(),
            capability.to_prompt_json(),
        );
    }
    vars.insert("aspect_ratio".into(), json!(&prompt_options.aspect_ratio));
    vars.insert(
        "reference_image_ids".into(),
        json!(&prompt_options.reference_image_ids),
    );
    vars.insert(
        "video_library_image_ids".into(),
        json!(&prompt_options.video_library_image_ids),
    );
    Ok(vars)
}

pub(super) fn required_chapter_title<'a>(
    title: Option<&'a str>,
    chapter_number: i32,
    operation: &str,
) -> Result<&'a str, String> {
    title
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("第 {chapter_number} 章缺少标题，无法{operation}"))
}

pub(super) fn validate_generate_prompt_options(
    input: GeneratePromptOptionsInput,
    workflow_mode: &str,
) -> Result<ChapterVideoPromptOptions, String> {
    validate_generate_prompt_options_inner(input, workflow_mode, None)
}

pub(super) fn validate_generate_prompt_options_for_preview(
    input: GeneratePromptOptionsInput,
    workflow_mode: &str,
) -> Result<ChapterVideoPromptOptions, String> {
    validate_generate_prompt_options_inner(input, workflow_mode, Some(PREVIEW_STYLE_PLACEHOLDER))
}

pub(super) fn video_prompt_workflow_fingerprint(
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
