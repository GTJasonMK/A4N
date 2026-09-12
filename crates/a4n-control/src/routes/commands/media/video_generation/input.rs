use serde_json::Value;

use super::super::super::{
    ApiError, command_input, i64_arg, nullable_i64_arg, video_generation_actions,
};

pub(super) fn chapter_video_generation_input(
    user_id: i64,
    args: &Value,
) -> Result<video_generation_actions::GenerateChapterVideoInput, ApiError> {
    Ok(video_generation_actions::GenerateChapterVideoInput {
        user_id,
        project_id: i64_arg(args, "projectId")?,
        chapter_id: i64_arg(args, "chapterId")?,
        prompt_id: i64_arg(args, "promptId")?,
        config_id: nullable_i64_arg(args, "configId")?,
    })
}

pub(super) fn reference_media_video_generation_input(
    user_id: i64,
    args: &Value,
    command: &str,
) -> Result<video_generation_actions::GenerateReferenceMediaVideoInput, ApiError> {
    let input = command_input(args, command)?;
    Ok(video_generation_actions::GenerateReferenceMediaVideoInput {
        user_id,
        project_id: i64_arg(args, "projectId")?,
        chapter_id: i64_arg(args, "chapterId")?,
        input,
    })
}

pub(super) fn optional_prompt_override(
    args: &Value,
    command: &str,
) -> Result<Option<video_generation_actions::PreviewChapterVideoPromptOverrideInput>, ApiError> {
    match args.get("promptOverride") {
        None => Ok(None),
        Some(value) => {
            let input = serde_json::from_value::<
                video_generation_actions::PreviewChapterVideoPromptOverrideInput,
            >(value.clone())
            .map_err(|error| {
                ApiError::bad_request(format!("{command}.promptOverride 无效: {error}"))
            })?;
            if input.is_empty() {
                return Err(ApiError::bad_request(format!(
                    "{command}.promptOverride 至少需要一个覆盖字段"
                )));
            }
            Ok(Some(input))
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::optional_prompt_override;

    #[test]
    fn prompt_override_rejects_null_empty_and_unknown_inputs() {
        assert!(
            optional_prompt_override(&json!({}), "preview_chapter_video_request")
                .unwrap()
                .is_none()
        );

        for value in [
            json!(null),
            json!({}),
            json!({"visual_prompt": null}),
            json!({"unknown": true}),
        ] {
            assert!(
                optional_prompt_override(
                    &json!({"promptOverride": value}),
                    "preview_chapter_video_request"
                )
                .is_err()
            );
        }

        let input = optional_prompt_override(
            &json!({"promptOverride": {"motion_prompt": null}}),
            "preview_chapter_video_request",
        )
        .unwrap()
        .unwrap();
        assert_eq!(input.motion_prompt, Some(None));
    }
}
