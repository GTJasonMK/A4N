use crate::routes::commands::{
    ApiError, chapter_generation_actions, command_input, i64_arg, optional_i32_field,
    optional_string_field, reject_unknown_command_args, required_json_i32, to_value,
};
use crate::state::ServerState;
use serde_json::{Value, json};

pub(in crate::routes::commands) async fn prepare_chapter_generation_rag_embedding(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    reject_unknown_command_args(
        args,
        "prepare_chapter_generation_rag_embedding",
        &[
            "projectId",
            "chapterNumber",
            "writingNotes",
            "minWordCount",
            "maxWordCount",
            "runtime_config_snapshot",
        ],
    )?;
    let project_id = i64_arg(args, "projectId")?;
    let chapter_number = required_json_i32(
        args,
        "chapterNumber",
        "prepare_chapter_generation_rag_embedding",
    )?;
    if chapter_number <= 0 {
        return Err(ApiError::bad_request(
            "prepare_chapter_generation_rag_embedding.chapterNumber 必须是正整数",
        ));
    }
    let input = chapter_generation_actions::PreviewChapterPromptInput {
        project_id,
        chapter_number,
        writing_notes: optional_string_field(args, "writingNotes")?,
        min_word_count: optional_i32_field(args, "minWordCount")?,
        max_word_count: optional_i32_field(args, "maxWordCount")?,
    };
    Ok(json!(
        chapter_generation_actions::prepare_chapter_generation_rag_embedding(
            state.runtime(),
            user_id,
            input
        )
        .await
        .map_err(ApiError::bad_request)?
    ))
}

pub(in crate::routes::commands) async fn preview_chapter_prompt(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: chapter_generation_actions::PreviewChapterPromptInput =
        command_input(args, "preview_chapter_prompt")?;
    let preview =
        chapter_generation_actions::preview_chapter_prompt(state.runtime(), user_id, input)
            .await
            .map_err(ApiError::bad_request)?;
    to_value(preview)
}
