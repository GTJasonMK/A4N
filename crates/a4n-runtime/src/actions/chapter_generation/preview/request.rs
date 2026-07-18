use super::super::{contracts::PreviewChapterPromptInput, preparation::prepare_chapter_generation};
use crate::{
    AppState,
    actions::{
        access::verify_project_editor_string,
        prompt_records::render_prompt_template,
        request_preview::{
            ChatRequestStepInput, ModelRequestPreview, chat_request_step_for_state,
            model_request_preview,
        },
    },
};

pub async fn preview_chapter_prompt(
    state: &AppState,
    user_id: i64,
    input: PreviewChapterPromptInput,
) -> Result<ModelRequestPreview, String> {
    verify_project_editor_string(state, user_id, input.project_id).await?;
    state
        .orchestrator
        .ensure_text_source_project(input.project_id, "预览章节正文生成请求")
        .await?;
    crate::workflow::validate_chapter_generation_ready(
        state,
        input.project_id,
        input.chapter_number,
    )
    .await?;
    let prepared = prepare_chapter_generation(
        state,
        input.project_id,
        input.chapter_number,
        input.writing_notes.as_deref(),
        input.min_word_count,
        input.max_word_count,
        None,
        &mut None,
        1,
        None,
    )
    .await?;

    let system_prompt = render_prompt_template(
        state,
        "writing",
        prepared.variables.clone(),
        Some(input.project_id),
    )
    .await
    .map_err(|error| format!("渲染 writing 提示词失败: {error}"))?;
    let request = a4n_prompt::chapter_generation::build_chapter_generation_request(
        input.project_id,
        input.chapter_number,
        system_prompt,
        &prepared.outline_text,
        prepared.context_before.as_deref(),
    )
    .map_err(|error| format!("构建章节生成请求失败: {error}"))?;
    let note = format!(
        "章节：{}；字数：{}-{}；检索参考：{}",
        prepared.chapter_title,
        prepared.word_count_range.min,
        prepared.word_count_range.max,
        if prepared.use_rag {
            "启用"
        } else {
            "未启用"
        }
    );
    Ok(model_request_preview(
        "chapter_generation",
        format!("第 {} 章生成请求预览", input.chapter_number),
        vec![
            chat_request_step_for_state(
                state,
                ChatRequestStepInput {
                    key: "chapter_generation".to_string(),
                    title: "章节正文生成 LLM 请求".to_string(),
                    prompt_name: Some("writing".to_string()),
                    state: "ready".to_string(),
                    note: Some(note),
                    request,
                },
            )
            .await?,
        ],
    ))
}
