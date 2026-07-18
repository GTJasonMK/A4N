use a4n_llm::provider::ChatRequest;

use super::super::contracts::MangaPipelineRequestPreviewStep;

impl MangaPipelineRequestPreviewStep {
    pub(in crate::manga::pipeline_preview) fn new(
        key: &str,
        title: &str,
        prompt_name: &str,
        state: &str,
        note: Option<String>,
        request: Option<ChatRequest>,
    ) -> Self {
        Self {
            key: key.to_string(),
            title: title.to_string(),
            prompt_name: Some(prompt_name.to_string()),
            state: state.to_string(),
            note,
            request,
        }
    }

    pub(in crate::manga::pipeline_preview) fn ready(
        key: &str,
        title: &str,
        prompt_name: &str,
        note: Option<String>,
        request: ChatRequest,
    ) -> Self {
        Self::new(key, title, prompt_name, "ready", note, Some(request))
    }

    pub(in crate::manga::pipeline_preview) fn reused(
        key: &str,
        title: &str,
        prompt_name: &str,
        note: String,
    ) -> Self {
        Self::new(key, title, prompt_name, "reused", Some(note), None)
    }

    pub(in crate::manga::pipeline_preview) fn reused_checkpoint(
        key: &str,
        title: &str,
        prompt_name: &str,
    ) -> Self {
        Self::reused(
            key,
            title,
            prompt_name,
            "已有匹配的调用级 checkpoint，本次不会重新发送该请求".to_string(),
        )
    }

    pub(in crate::manga::pipeline_preview) fn blocked(
        key: &str,
        title: &str,
        prompt_name: &str,
        note: &str,
    ) -> Self {
        Self::new(
            key,
            title,
            prompt_name,
            "blocked",
            Some(note.to_string()),
            None,
        )
    }
}

pub(in crate::manga::pipeline_preview) fn push_blocked_after(
    steps: &mut Vec<MangaPipelineRequestPreviewStep>,
    next_step_key: &str,
) {
    for (key, title, prompt_name) in manga_pipeline_steps_after(next_step_key) {
        steps.push(MangaPipelineRequestPreviewStep::blocked(
            key,
            title,
            prompt_name,
            "该请求依赖前序模型输出，发送前无法提前构造真实请求",
        ));
    }
}

fn manga_pipeline_steps_after(
    next_step_key: &str,
) -> &'static [(&'static str, &'static str, &'static str)] {
    const STEPS: &[(&str, &str, &str)] = &[
        (
            "manga_character_binding",
            "漫画 Step 1：角色绑定",
            "manga_character_binding",
        ),
        (
            "manga_speech_selection",
            "漫画 Step 2：对白旁白选择",
            "manga_speech_selection",
        ),
        (
            "manga_visual_anchor_extraction",
            "漫画 Step 3：视觉锚点抽取",
            "manga_visual_anchor_extraction",
        ),
        (
            "manga_item_extraction",
            "漫画 Step 4：关键物品抽取",
            "manga_item_extraction",
        ),
        (
            "manga_chapter_comic_synthesis",
            "漫画 Step 5：章节节奏综合",
            "manga_chapter_comic_synthesis",
        ),
        (
            "manga_page_planning",
            "漫画 Step 6：分页规划",
            "manga_page_planning",
        ),
        (
            "manga_page_drawing_prompt",
            "漫画 Step 7：整页绘图提示词",
            "manga_page_drawing_prompt",
        ),
    ];
    let index = STEPS
        .iter()
        .position(|(key, _, _)| *key == next_step_key)
        .unwrap_or(STEPS.len());
    &STEPS[index..]
}
