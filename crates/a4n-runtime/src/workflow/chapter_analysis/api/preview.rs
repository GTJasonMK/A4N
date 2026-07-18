use super::super::{
    blocked_chapter_analysis_step, blocked_scene_profile_sync_chapter_analysis_steps,
    character_discovery_chapter_analysis_steps, object_discovery_chapter_analysis_steps,
    preflight_chapter_analysis, scene_profile_sync_chapter_analysis_steps,
};
use super::resume::preflight_resume_chapter_analysis_sync;
use crate::actions::request_preview::{
    ChatStatusPreviewStep, ModelRequestPreview, chat_status_preview_for_state,
};
use crate::prompt_renderer::RuntimePromptRenderer;
use crate::state::AppState;

pub async fn preview_chapter_analysis_request(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
) -> Result<ModelRequestPreview, String> {
    let ch = preflight_chapter_analysis(
        state,
        user_id,
        chapter_id,
        "预览结构化分析提示词",
        "章节正文为空，不能预览结构化分析提示词",
        false,
    )
    .await?;

    let prompt_renderer = RuntimePromptRenderer::new(state);
    let analysis_steps = state
        .orchestrator
        .build_chapter_analysis_requests(chapter_id, &prompt_renderer)
        .await
        .map_err(|err| err.to_string())?;
    let mut steps = analysis_steps
        .into_iter()
        .map(|step| {
            ChatStatusPreviewStep::llm(
                step.key,
                step.title,
                Some(step.prompt_name),
                "ready",
                None,
                Some(step.request),
            )
        })
        .collect::<Vec<_>>();
    steps.push(blocked_chapter_analysis_step(
        "object_discovery",
        "分析后物体档案发现请求",
        Some("object_discovery"),
        "该请求依赖结构化分析保存后的章节数据，发送前无法提前构造真实请求",
    ));
    steps.extend(character_discovery_chapter_analysis_steps(
        crate::actions::character_ai::character_discovery_preview_steps(
            state,
            user_id,
            ch.project_id,
            Some(ch.chapter_number),
            "ready",
            Some("结构化分析保存后会继续执行该请求".to_string()),
        )
        .await?,
    ));
    steps.extend(blocked_scene_profile_sync_chapter_analysis_steps(
        ch.chapter_number,
        "该请求依赖结构化分析保存后的章节数据，发送前无法提前构造真实请求",
    ));
    steps.push(blocked_chapter_analysis_step(
        "analysis_character_sync",
        "分析后角色档案同步请求",
        Some("character_sync"),
        "该请求依赖结构化分析输出和角色发现模型返回结果，发送前无法提前构造真实请求",
    ));
    chat_status_preview_for_state(
        state,
        "chapter_analysis",
        format!("第 {} 章结构化分析请求预览", ch.chapter_number),
        steps,
        "llm_chat",
    )
    .await
}

pub async fn preview_resume_chapter_analysis_sync_request(
    state: &AppState,
    user_id: i64,
    chapter_id: i64,
) -> Result<ModelRequestPreview, String> {
    let ch = preflight_resume_chapter_analysis_sync(state, user_id, chapter_id).await?;

    let mut steps = object_discovery_chapter_analysis_steps(
        crate::actions::object_discovery::object_discovery_preview_steps(
            state,
            ch.project_id,
            Some(ch.chapter_number),
            "ready",
            Some("继续同步会先重新执行物体档案发现".to_string()),
        )
        .await?,
    );
    steps.extend(character_discovery_chapter_analysis_steps(
        crate::actions::character_ai::character_discovery_preview_steps(
            state,
            user_id,
            ch.project_id,
            Some(ch.chapter_number),
            "ready",
            Some("继续同步会先重新执行角色发现".to_string()),
        )
        .await?,
    ));
    steps.extend(scene_profile_sync_chapter_analysis_steps(
        crate::actions::scene_profile_sync::scene_profile_sync_preview_steps(
            state,
            user_id,
            ch.project_id,
            Some(ch.chapter_number),
            "ready",
            Some(
                "继续同步会先执行场景档案候选识别；后续场景同步请求依赖该阶段返回结果".to_string(),
            ),
        )
        .await?,
    ));
    steps.push(blocked_chapter_analysis_step(
        "analysis_character_sync",
        "分析后角色档案同步请求",
        Some("character_sync"),
        "该请求依赖角色发现模型返回结果；当前只能确认会在发现结果返回后构造",
    ));
    chat_status_preview_for_state(
        state,
        "chapter_analysis_resume_sync",
        format!("第 {} 章分析继续同步请求预览", ch.chapter_number),
        steps,
        "llm_chat",
    )
    .await
}
