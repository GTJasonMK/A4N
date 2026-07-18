use super::prelude::*;
use super::*;

pub(super) fn ensure_chapter_not_generating(
    ch: &chapter::Model,
    operation: &str,
) -> Result<(), String> {
    if ch.is_generating() {
        return Err(format!(
            "第 {} 章正在生成中，请等待当前生成完成后再{operation}",
            ch.chapter_number
        ));
    }
    Ok(())
}

pub(super) fn blocked_chapter_analysis_step(
    key: impl Into<String>,
    title: impl Into<String>,
    prompt_name: Option<&str>,
    note: impl Into<String>,
) -> ChatStatusPreviewStep {
    ChatStatusPreviewStep::llm(
        key,
        title,
        prompt_name.map(str::to_string),
        "blocked",
        Some(note.into()),
        None,
    )
}

pub(super) fn character_discovery_chapter_analysis_steps(
    steps: Vec<crate::actions::character_ai::CharacterAiPreviewStep>,
) -> Vec<ChatStatusPreviewStep> {
    steps
        .into_iter()
        .map(|step| {
            ChatStatusPreviewStep::llm(
                step.key,
                step.title,
                step.prompt_name,
                step.state,
                step.note,
                step.request,
            )
        })
        .collect()
}

pub(super) fn object_discovery_chapter_analysis_steps(
    steps: Vec<crate::actions::object_discovery::ObjectDiscoveryPreviewStep>,
) -> Vec<ChatStatusPreviewStep> {
    steps
        .into_iter()
        .map(|step| {
            ChatStatusPreviewStep::llm(
                step.key,
                step.title,
                step.prompt_name,
                step.state,
                step.note,
                Some(step.request),
            )
        })
        .collect()
}

pub(super) fn scene_profile_sync_chapter_analysis_steps(
    steps: Vec<crate::actions::scene_profile_sync::SceneProfileSyncPreviewStep>,
) -> Vec<ChatStatusPreviewStep> {
    steps
        .into_iter()
        .map(|step| {
            ChatStatusPreviewStep::llm(
                step.key,
                step.title,
                step.prompt_name,
                step.state,
                step.note,
                step.request,
            )
        })
        .collect()
}

pub(super) fn blocked_scene_profile_sync_chapter_analysis_steps(
    chapter_number: i32,
    note: &str,
) -> Vec<ChatStatusPreviewStep> {
    vec![
        blocked_chapter_analysis_step(
            format!("scene_profile_sync_ch{chapter_number}_scenes"),
            "分析后场景档案候选识别请求",
            Some("scene_profile_candidates"),
            note,
        ),
        blocked_chapter_analysis_step(
            format!("scene_profile_sync_ch{chapter_number}_occurrences"),
            "分析后场景出场抽取请求",
            Some("scene_occurrence_extraction"),
            "该请求依赖场景档案候选识别返回的 scenes，发送前无法提前构造真实请求",
        ),
        blocked_chapter_analysis_step(
            format!("scene_profile_sync_ch{chapter_number}_transitions"),
            "分析后场景转场抽取请求",
            Some("scene_transition_extraction"),
            "该请求依赖场景档案候选和场景出场模型返回结果，发送前无法提前构造真实请求",
        ),
        blocked_chapter_analysis_step(
            format!("scene_profile_sync_ch{chapter_number}_entity_states"),
            "分析后场景实体状态抽取请求",
            Some("scene_entity_state_extraction"),
            "该请求依赖场景档案候选和场景出场模型返回结果，发送前无法提前构造真实请求",
        ),
    ]
}

pub(super) fn report_chapter_analysis_progress(
    progress: Option<&ChapterAnalysisProgress<'_>>,
    current: u32,
    total: u32,
    message: &str,
    cancelled_message: &str,
) -> Result<(), String> {
    if let Some(progress) = progress {
        progress.report(current, total, message, cancelled_message)?;
    }
    Ok(())
}

pub(super) fn report_chapter_subprogress(
    progress: Option<&ChapterAnalysisProgress<'_>>,
    current: u32,
    total: u32,
    span_start: u32,
    span_end: u32,
    message: &str,
) {
    let Some(progress) = progress else {
        return;
    };
    let Some(reporter) = progress.reporter else {
        return;
    };
    let mapped = map_subprogress(current, total, span_start, span_end);
    reporter(mapped, CHAPTER_ANALYSIS_PROGRESS_TOTAL, message);
}

pub(super) fn map_subprogress(current: u32, total: u32, span_start: u32, span_end: u32) -> u32 {
    let total = total.max(1);
    let span_width = span_end
        .checked_sub(span_start)
        .expect("chapter analysis progress spans must be ordered");
    let offset = (u64::from(span_width) * u64::from(current.min(total))) / u64::from(total);
    span_start + u32::try_from(offset).expect("subprogress offset is bounded by the u32 span width")
}
