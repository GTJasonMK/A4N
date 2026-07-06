use super::*;

pub(super) async fn import_uploaded_file(
    _state: &ServerState,
    _user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: a4n_runtime::actions::import_file::ImportUploadedFileInput =
        command_input(args, "import_uploaded_file")?;
    let result = a4n_runtime::actions::import_file::import_uploaded_file(input)
        .map_err(ApiError::bad_request)?;
    Ok(json!(result))
}

macro_rules! import_web_project_command {
    ($name:ident, $action:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let project_id = i64_arg(args, "projectId")?;
            to_value($action(state, user_id, project_id).await?)
        }
    };
}

macro_rules! import_web_string_project_command {
    ($name:ident, $action:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let project_id = i64_arg(args, "projectId")?;
            to_value(
                $action(state, user_id, project_id)
                    .await
                    .map_err(ApiError::bad_request)?,
            )
        }
    };
}

macro_rules! import_web_review_command {
    ($name:ident, $action:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let review_id = i64_arg(args, "reviewId")?;
            to_value($action(state, user_id, review_id).await?)
        }
    };
}

macro_rules! import_web_review_unit_command {
    ($name:ident, $action:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let review_id = i64_arg(args, "reviewId")?;
            $action(state, user_id, review_id).await?;
            Ok(Value::Null)
        }
    };
}

macro_rules! import_web_review_input_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let review_id = i64_arg(args, "reviewId")?;
            let input: $input = command_input(args, stringify!($name))?;
            to_value($action(state, user_id, review_id, input).await?)
        }
    };
}

macro_rules! import_web_input_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $input = command_input(args, stringify!($name))?;
            to_value($action(state, user_id, input).await?)
        }
    };
}

import_web_project_command!(list_import_queue, import_review_actions::list_import_queue);
import_web_review_input_command!(
    update_import_status,
    import_review_actions::UpdateImportStatusInput,
    import_review_actions::update_import_status,
);
import_web_review_unit_command!(
    delete_import_review,
    import_review_actions::delete_import_review,
);
import_web_review_command!(get_import_review, import_review_actions::get_import_review);
import_web_review_command!(
    accept_import_review,
    import_review_actions::accept_import_review,
);
import_web_review_input_command!(
    modify_import_review,
    a4n_services::import::ModifyImportReviewInput,
    import_review_actions::modify_import_review,
);
import_web_input_command!(
    batch_accept_reviews,
    import_review_actions::BatchAcceptImportReviewsInput,
    import_review_actions::batch_accept_import_reviews,
);
import_web_review_command!(
    reject_import_review,
    import_review_actions::reject_import_review,
);

pub(super) async fn start_import_analysis(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let input: import_analysis_actions::StartImportAnalysisInput =
        command_input(args, "start_import_analysis")?;
    let reviews = import_analysis_actions::start_import_analysis(state, user_id, project_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    let summaries = reviews
        .iter()
        .map(import_review_actions::import_review_summary)
        .collect::<Result<Vec<_>, _>>()?;
    to_value(summaries)
}

import_web_project_command!(
    get_import_analysis_status,
    import_review_actions::get_import_analysis_status,
);
import_web_string_project_command!(
    get_project_import_analysis_progress,
    import_analysis_actions::get_project_import_analysis_progress,
);
import_web_string_project_command!(
    run_import_project_analysis,
    import_analysis_actions::run_import_project_analysis,
);

pub(super) async fn preview_import_project_analysis_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let preview = import_analysis_actions::preview_import_project_analysis_request(
        state, user_id, project_id,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(preview)
}

import_web_string_project_command!(
    cancel_import_analysis,
    import_analysis_actions::cancel_import_analysis,
);
import_web_input_command!(
    check_import_dedup,
    import_review_actions::CheckImportDedupInput,
    import_review_actions::check_import_dedup,
);

#[derive(Debug, Clone, Copy)]
pub(super) enum ReverseEngineerWebCommand {
    Run {
        target: import_reverse_actions::ReverseEngineeringTarget,
        apply: bool,
    },
    Preview {
        target: import_reverse_actions::ReverseEngineeringTarget,
    },
}

pub(super) fn reverse_engineer_web_command(command: &str) -> Option<ReverseEngineerWebCommand> {
    use ReverseEngineerWebCommand::{Preview, Run};
    use import_reverse_actions::ReverseEngineeringTarget::{Blueprint, Outline, PartOutline};

    Some(match command {
        "reverse_engineer_blueprint" => Run {
            target: Blueprint,
            apply: false,
        },
        "reverse_engineer_blueprint_and_apply" => Run {
            target: Blueprint,
            apply: true,
        },
        "preview_reverse_engineer_blueprint_request" => Preview { target: Blueprint },
        "reverse_engineer_outline" => Run {
            target: Outline,
            apply: false,
        },
        "reverse_engineer_outline_and_apply" => Run {
            target: Outline,
            apply: true,
        },
        "preview_reverse_engineer_outline_request" => Preview { target: Outline },
        "reverse_engineer_part_outline" => Run {
            target: PartOutline,
            apply: false,
        },
        "reverse_engineer_part_outline_and_apply" => Run {
            target: PartOutline,
            apply: true,
        },
        "preview_reverse_engineer_part_outline_request" => Preview {
            target: PartOutline,
        },
        _ => return None,
    })
}

pub(super) async fn run_reverse_engineer_web_command(
    state: &ServerState,
    user_id: i64,
    args: &Value,
    command: ReverseEngineerWebCommand,
) -> Result<Value, ApiError> {
    match command {
        ReverseEngineerWebCommand::Run { target, apply } => {
            reverse_engineer_command(state, user_id, args, target, apply).await
        }
        ReverseEngineerWebCommand::Preview { target } => {
            preview_reverse_engineer_request_command(state, user_id, args, target).await
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReverseEngineerCommandArgs {
    #[serde(default, rename = "userId")]
    _user_id: Option<i64>,
    #[serde(default, rename = "runtime_config_snapshot")]
    _runtime_config_snapshot: Option<Value>,
    input: import_reverse_actions::ReverseEngineerInput,
    #[serde(default, rename = "taskId")]
    task_id: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ReverseEngineerPreviewArgs {
    #[serde(default, rename = "userId")]
    _user_id: Option<i64>,
    #[serde(default, rename = "runtime_config_snapshot")]
    _runtime_config_snapshot: Option<Value>,
    input: import_reverse_actions::ReverseEngineerInput,
}

#[cfg(test)]
mod reverse_engineer_command_args_tests {
    use super::*;

    pub(super) fn reverse_input_json() -> Value {
        json!({
            "project_id": 42,
            "content_sample": "样本文本",
            "auto_context": true,
            "context_limit": 8,
            "variables": null,
        })
    }

    #[test]
    pub(super) fn accepts_web_envelope_fields_for_run_command() {
        let args: ReverseEngineerCommandArgs = serde_json::from_value(json!({
            "userId": 7,
            "runtime_config_snapshot": { "schema_version": 1 },
            "input": reverse_input_json(),
            "taskId": "reverse-task-1",
        }))
        .expect("web envelope fields should be accepted");

        assert_eq!(args.input.project_id, 42);
        assert_eq!(args.task_id.as_deref(), Some("reverse-task-1"));
    }

    #[test]
    pub(super) fn accepts_web_envelope_fields_for_preview_command() {
        let args: ReverseEngineerPreviewArgs = serde_json::from_value(json!({
            "userId": 7,
            "runtime_config_snapshot": { "schema_version": 1 },
            "input": reverse_input_json(),
        }))
        .expect("web envelope fields should be accepted");

        assert_eq!(args.input.project_id, 42);
    }

    #[test]
    pub(super) fn rejects_unknown_reverse_command_fields() {
        let err = serde_json::from_value::<ReverseEngineerPreviewArgs>(json!({
            "userId": 7,
            "input": reverse_input_json(),
            "unexpected": true,
        }))
        .expect_err("unknown fields should remain explicit command errors");

        assert!(err.to_string().contains("unexpected"));
    }
}

pub(super) async fn reverse_engineer_command(
    state: &ServerState,
    user_id: i64,
    args: &Value,
    target: import_reverse_actions::ReverseEngineeringTarget,
    apply: bool,
) -> Result<Value, ApiError> {
    let command: ReverseEngineerCommandArgs = serde_json::from_value(args.clone())
        .map_err(|e| ApiError::bad_request(format!("{} 参数无效: {e}", target.prompt_name())))?;
    let scope =
        import_reverse_actions::reverse_engineer_task_scope(state, user_id, &command.input, apply)
            .await?;

    let task_id = sanitize_optional_task_id(command.task_id)?;
    let task = register_web_task(
        state,
        task_id.clone(),
        TaskType::LlmCall,
        scope.project_id,
        scope.total,
        target.initial_message(apply),
    )
    .await;
    let progress = web_progress_reporter(state, task, task_id.clone());
    let result = if apply {
        import_reverse_actions::reverse_engineer_and_apply(
            state,
            user_id,
            &command.input,
            target,
            &progress,
        )
        .await
    } else {
        import_reverse_actions::reverse_engineer(state, user_id, &command.input, target, &progress)
            .await
    };
    finish_web_task(state, task_id, result, target.success_message(apply)).await
}

pub(super) async fn preview_reverse_engineer_request_command(
    state: &ServerState,
    user_id: i64,
    args: &Value,
    target: import_reverse_actions::ReverseEngineeringTarget,
) -> Result<Value, ApiError> {
    let command: ReverseEngineerPreviewArgs =
        serde_json::from_value(args.clone()).map_err(|e| {
            ApiError::bad_request(format!(
                "preview_{}_request 参数无效: {e}",
                target.prompt_name()
            ))
        })?;
    let preview = import_reverse_actions::preview_reverse_engineer_request(
        state,
        user_id,
        &command.input,
        target,
    )
    .await?;
    to_value(preview)
}

macro_rules! export_web_project_read_command {
    (
        $name:ident,
        $scope:path,
        $start_message:expr,
        $progress_message:expr,
        $success_message:expr,
        |$state:ident, $user_id:ident, $args:ident, $project_id:ident|,
        {$($prepare:tt)*},
        $run:expr $(,)?
    ) => {
        pub(super) async fn $name(
            $state: &ServerState,
            $user_id: i64,
            $args: &Value,
        ) -> Result<Value, ApiError> {
            let $project_id = i64_arg($args, "projectId")?;
            $($prepare)*
            let scope = $scope($state, $user_id, $project_id)
                .await
                .map_err(ApiError::bad_request)?;
            let task_id = sanitize_optional_task_id(optional_string_field($args, "taskId")?)?;
            let task = register_web_task(
                $state,
                task_id.clone(),
                TaskType::ExportGeneration,
                scope.project_id,
                scope.total,
                $start_message,
            )
            .await;
            let progress = web_progress_reporter($state, task, task_id.clone());
            let result = async {
                progress(0, scope.total, $progress_message)?;
                let value = $run
                    .await
                    .map_err(a4n_core::error::AppError::InvalidArgument)?;
                progress(scope.total, scope.total, $success_message)?;
                Ok(value)
            }
            .await;
            let value = finish_web_task($state, task_id, result, $success_message).await?;
            Ok(json!(value))
        }
    };
}

macro_rules! export_web_progress_command {
    (
        $name:ident,
        $scope:expr,
        $start_message:expr,
        $success_message:expr,
        |$state:ident, $user_id:ident, $args:ident, $progress:ident|,
        {$($prepare:tt)*},
        $run:expr $(,)?
    ) => {
        pub(super) async fn $name(
            $state: &ServerState,
            $user_id: i64,
            $args: &Value,
        ) -> Result<Value, ApiError> {
            $($prepare)*
            let scope = $scope.await.map_err(ApiError::bad_request)?;
            let task_id = sanitize_optional_task_id(optional_string_field($args, "taskId")?)?;
            let task = register_web_task(
                $state,
                task_id.clone(),
                TaskType::ExportGeneration,
                scope.project_id,
                scope.total,
                $start_message,
            )
            .await;
            let cancel_token = task.as_ref().map(|handle| handle.cancel_token.clone());
            let reporter = web_fire_and_forget_progress_reporter($state, task, task_id.clone());
            let $progress =
                export_files_actions::ExportProgress::new(Some(&reporter), cancel_token.as_ref());
            let result = $run.await;
            let value = finish_web_task(
                $state,
                task_id,
                string_action_result(result),
                $success_message,
            )
            .await?;
            Ok(json!(value))
        }
    };
}

export_web_project_read_command!(
    export_txt_data,
    export_files_actions::export_txt_task_scope,
    "正在准备 TXT 导出",
    "正在读取可导出章节",
    "TXT 导出完成",
    |state, user_id, args, project_id|,
    {},
    export_files_actions::export_txt_data(state, user_id, project_id),
);

export_web_project_read_command!(
    export_markdown_data,
    export_files_actions::export_markdown_task_scope,
    "正在准备 Markdown 导出",
    "正在读取可导出章节",
    "Markdown 导出完成",
    |state, user_id, args, project_id|,
    {
        let input: export_files_actions::ExportMarkdownInput =
            command_input(args, "export_markdown_data")?;
    },
    export_files_actions::export_markdown_data(
        state,
        user_id,
        project_id,
        &input.title,
        &input.author,
    ),
);

export_web_progress_command!(
    export_epub_data,
    export_files_actions::export_epub_task_scope(state, user_id, project_id),
    "正在准备 EPUB 导出",
    "EPUB 导出完成",
    |state, user_id, args, progress|,
    {
        let project_id = i64_arg(args, "projectId")?;
        let title = string_arg(args, "title")?;
        let author = string_arg(args, "author")?;
    },
    export_files_actions::export_epub_data(
        state, user_id, project_id, title, author, &progress,
    ),
);

export_web_progress_command!(
    export_pdf_data,
    export_files_actions::export_pdf_task_scope(state, user_id, project_id),
    "正在准备 PDF 导出",
    "PDF 导出完成",
    |state, user_id, args, progress|,
    {
        let project_id = i64_arg(args, "projectId")?;
    },
    export_files_actions::export_pdf_data(state, user_id, project_id, &progress),
);

export_web_progress_command!(
    export_batch_data,
    export_files_actions::export_batch_task_scope(state, user_id, &input),
    "正在准备批量导出",
    "批量导出完成",
    |state, user_id, args, progress|,
    {
        let input: export_files_actions::BatchExportInput =
            command_input(args, "export_batch_data")?;
    },
    export_files_actions::export_batch_data(state, user_id, input, &progress,),
);

export_web_project_read_command!(
    export_outline_json_data,
    export_files_actions::export_outline_json_task_scope,
    "正在准备导出大纲 JSON",
    "正在读取大纲数据",
    "大纲 JSON 导出完成",
    |state, user_id, args, project_id|,
    {},
    export_files_actions::export_outline_json_data(state, user_id, project_id),
);

export_web_project_read_command!(
    export_characters_json_data,
    export_files_actions::export_characters_json_task_scope,
    "正在准备导出角色 JSON",
    "正在读取角色数据",
    "角色 JSON 导出完成",
    |state, user_id, args, project_id|,
    {},
    export_files_actions::export_characters_json_data(state, user_id, project_id),
);

pub(super) async fn list_portraits_by_project(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let portraits = portrait_records_actions::list_portraits_by_project(state, user_id, project_id)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(portraits)
}

pub(super) async fn get_primary_portrait(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let profile_id = i64_arg(args, "profileId")?;
    let portrait = portrait_records_actions::get_primary_portrait(state, user_id, profile_id)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(portrait)
}

pub(super) async fn set_primary_portrait(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let portrait_id = i64_arg(args, "portraitId")?;
    portrait_records_actions::set_primary_portrait(state, user_id, portrait_id)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}

pub(super) async fn upload_portrait(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: portrait_generation_actions::UploadPortraitInput =
        command_input(args, "upload_portrait")?;
    let portrait = portrait_generation_actions::upload_portrait(state, user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(portrait)
}

pub(super) async fn generate_portrait_prompt(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: portrait_generation_actions::GeneratePortraitPromptInput =
        command_input(args, "generate_portrait_prompt")?;
    let prompt = portrait_generation_actions::generate_portrait_prompt(state, user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    Ok(json!(prompt))
}

pub(super) async fn preview_generate_portrait_prompt_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: portrait_generation_actions::GeneratePortraitPromptInput =
        command_input(args, "preview_generate_portrait_prompt_request")?;
    let preview = portrait_generation_actions::preview_generate_portrait_prompt_request(
        state, user_id, input,
    )
    .await
    .map_err(ApiError::bad_request)?;
    to_value(preview)
}

macro_rules! portrait_web_generation_command {
    (
        $name:ident,
        |$args:ident| $input:expr,
        $scope:path,
        $start_message:expr,
        $success_message:expr,
        $action:path $(,)?
    ) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            $args: &Value,
        ) -> Result<Value, ApiError> {
            let input = $input;
            let scope = $scope(state, user_id, &input)
                .await
                .map_err(ApiError::bad_request)?;
            let task_id = sanitize_optional_task_id(optional_string_field($args, "taskId")?)?;
            let task = register_web_task(
                state,
                task_id.clone(),
                TaskType::ImageGeneration,
                scope.project_id,
                scope.total,
                $start_message,
            )
            .await;
            let cancel_token = task
                .as_ref()
                .map(|handle| handle.cancel_token.clone())
                .unwrap_or_else(tokio_util::sync::CancellationToken::new);
            let mut progress = web_progress_reporter(state, task, task_id.clone());
            let result = $action(state, user_id, input, Some(&mut progress), cancel_token).await;
            let value = finish_web_task(
                state,
                task_id,
                string_action_result(result),
                $success_message,
            )
            .await?;
            Ok(json!(value))
        }
    };
}

macro_rules! portrait_web_preview_command {
    ($name:ident, |$args:ident| $input:expr, $action:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            $args: &Value,
        ) -> Result<Value, ApiError> {
            let preview = $action(state, user_id, $input)
                .await
                .map_err(ApiError::bad_request)?;
            to_value(preview)
        }
    };
}

portrait_web_generation_command!(
    generate_portrait,
    |args| {
        let project_id = i64_arg(args, "projectId")?;
        portrait_generation_actions::GeneratePortraitInput {
            project_id,
            character_name: string_arg(args, "characterName")?,
            style: optional_string_field(args, "style")?,
            expressions: optional_string_vec_field(args, "expressions")?,
            pose: optional_string_field(args, "pose")?,
            reference_image_ids: optional_json_i64_vec(
                args,
                "referenceImageIds",
                "generate_portrait",
            )?,
        }
    },
    portrait_generation_actions::generate_portrait_task_scope,
    "正在准备人物三视图拆解生成",
    "人物三视图拆解生成完成",
    portrait_generation_actions::generate_portrait,
);

portrait_web_preview_command!(
    preview_generate_portrait_request,
    |args| {
        let project_id = i64_arg(args, "projectId")?;
        portrait_generation_actions::GeneratePortraitInput {
            project_id,
            character_name: string_arg(args, "characterName")?,
            style: optional_string_field(args, "style")?,
            expressions: optional_string_vec_field(args, "expressions")?,
            pose: optional_string_field(args, "pose")?,
            reference_image_ids: optional_json_i64_vec(
                args,
                "referenceImageIds",
                "preview_generate_portrait_request",
            )?,
        }
    },
    portrait_generation_actions::preview_generate_portrait_request,
);

portrait_web_generation_command!(
    regenerate_portrait,
    |args| {
        portrait_generation_actions::RegeneratePortraitInput {
            portrait_id: i64_arg(args, "portraitId")?,
            reference_image_ids: optional_json_i64_vec(
                args,
                "referenceImageIds",
                "regenerate_portrait",
            )?,
        }
    },
    portrait_generation_actions::regenerate_portrait_task_scope,
    "正在准备重绘人物三视图拆解",
    "人物三视图拆解重绘完成",
    portrait_generation_actions::regenerate_portrait,
);

portrait_web_preview_command!(
    preview_regenerate_portrait_request,
    |args| {
        portrait_generation_actions::RegeneratePortraitInput {
            portrait_id: i64_arg(args, "portraitId")?,
            reference_image_ids: optional_json_i64_vec(
                args,
                "referenceImageIds",
                "preview_regenerate_portrait_request",
            )?,
        }
    },
    portrait_generation_actions::preview_regenerate_portrait_request,
);

portrait_web_generation_command!(
    generate_age_variant,
    |args| {
        portrait_generation_actions::GenerateAgeVariantInput {
            portrait_id: i64_arg(args, "portraitId")?,
            age: string_arg(args, "age")?,
            reference_image_ids: optional_json_i64_vec(
                args,
                "referenceImageIds",
                "generate_age_variant",
            )?,
        }
    },
    portrait_generation_actions::generate_age_variant_task_scope,
    "正在准备年龄变体",
    "年龄变体生成完成",
    portrait_generation_actions::generate_age_variant,
);

portrait_web_preview_command!(
    preview_generate_age_variant_request,
    |args| {
        portrait_generation_actions::GenerateAgeVariantInput {
            portrait_id: i64_arg(args, "portraitId")?,
            age: string_arg(args, "age")?,
            reference_image_ids: optional_json_i64_vec(
                args,
                "referenceImageIds",
                "preview_generate_age_variant_request",
            )?,
        }
    },
    portrait_generation_actions::preview_generate_age_variant_request,
);

portrait_web_generation_command!(
    generate_missing_portraits,
    |args| {
        portrait_generation_actions::GenerateMissingPortraitsInput {
            project_id: i64_arg(args, "projectId")?,
        }
    },
    portrait_generation_actions::generate_missing_portraits_task_scope,
    "准备补齐人物三视图拆解",
    "人物三视图拆解补齐完成",
    portrait_generation_actions::generate_missing_portraits,
);

portrait_web_preview_command!(
    preview_generate_missing_portraits_request,
    |args| {
        portrait_generation_actions::GenerateMissingPortraitsInput {
            project_id: i64_arg(args, "projectId")?,
        }
    },
    portrait_generation_actions::preview_generate_missing_portraits_request,
);

pub(super) async fn get_storage_paths(
    state: &ServerState,
    _user_id: i64,
    _args: &Value,
) -> Result<Value, ApiError> {
    to_value(storage_paths_actions::get_storage_paths(state))
}

pub(super) async fn list_orphan_project_data_dirs(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    super::require_admin_caller(state, user_id, args).await?;
    let caller_user_id = i64_arg(args, "callerUserId")?;
    let result = storage_paths_actions::list_orphan_project_data_dirs(state, caller_user_id)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(result)
}

pub(super) async fn cleanup_orphan_project_data_dirs(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: storage_paths_actions::CleanupOrphanProjectDataDirsInput =
        command_input(args, "cleanup_orphan_project_data_dirs")?;
    if input.caller_user_id != user_id {
        return Err(ApiError::forbidden("请求用户与登录用户不一致"));
    }
    let result = storage_paths_actions::cleanup_orphan_project_data_dirs(state, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(result)
}

pub(super) async fn get_log_dir(
    state: &ServerState,
    user_id: i64,
    _args: &Value,
) -> Result<Value, ApiError> {
    let path =
        logging_actions::log_dir_path_for_admin(state, user_id, &state.config.log_dir).await?;
    Ok(json!(path))
}

pub(super) async fn read_recent_logs(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: logging_actions::ReadLogsInput = command_input(args, "read_recent_logs")?;
    let result =
        logging_actions::read_recent_logs_for_admin(state, user_id, &state.config.log_dir, input)
            .await?;
    to_value(result)
}

pub(super) async fn set_log_level(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: a4n_runtime::actions::logging::SetLogLevelInput =
        command_input(args, "set_log_level")?;
    logging_actions::ensure_log_diagnostics_admin(state, user_id).await?;
    crate::logging::set_log_level(input).map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}

pub(super) async fn get_generation_call_analytics(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: generation_analytics_actions::GenerationCallAnalyticsInput =
        command_input(args, "get_generation_call_analytics")?;
    let result = generation_analytics_actions::get_generation_call_analytics_for_admin(
        state, user_id, input,
    )
    .await?;
    to_value(result)
}

pub(super) async fn list_chapters(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    Ok(json!(
        chapter_content_actions::list_chapters(state, user_id, project_id).await?
    ))
}

pub(super) fn chapter_summary_value(ch: &a4n_core::models::chapter::chapter::Model) -> Value {
    json!({
        "id": ch.id,
        "project_id": ch.project_id,
        "outline_id": ch.outline_id,
        "chapter_number": ch.chapter_number,
        "title": ch.title,
        "content": ch.content,
        "word_count": ch.word_count,
        "status": ch.status,
        "summary": ch.summary,
        "style_tags": ch.style_tags,
        "analysis_data": ch.analysis_data,
        "created_at": ch.created_at.to_string(),
        "updated_at": ch.updated_at.to_string(),
    })
}

pub(super) async fn import_chapter(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: chapter_content_actions::ImportChapterInput = command_input(args, "import_chapter")?;
    Ok(json!(
        chapter_content_actions::import_chapter(state, user_id, input).await?
    ))
}

pub(super) async fn update_chapter_content(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: chapter_content_actions::UpdateChapterContentInput =
        command_input(args, "update_chapter_content")?;
    Ok(json!(
        chapter_content_actions::update_chapter_content(state, user_id, input).await?
    ))
}

pub(super) async fn generate_chapter_stream(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_number = required_json_i32(args, "chapterNumber", "generate_chapter_stream")?;
    let input = chapter_generation_actions::GenerateChapterStreamInput {
        project_id,
        chapter_number,
        task_id: optional_string_field(args, "taskId")?,
        writing_notes: optional_string_field(args, "writingNotes")?,
        min_word_count: optional_i32_field(args, "minWordCount")?,
        max_word_count: optional_i32_field(args, "maxWordCount")?,
        client_rag_context: optional_string_field(args, "clientRagContext")?,
    };
    let scope =
        chapter_generation_actions::generate_chapter_stream_task_scope(state, user_id, &input)
            .await
            .map_err(ApiError::bad_request)?;

    let task_id = chapter_generation_actions::chapter_stream_task_id(
        project_id,
        chapter_number,
        input.task_id.clone(),
    )
    .map_err(ApiError::bad_request)?;
    let task = register_web_task(
        state,
        Some(task_id.clone()),
        TaskType::LlmCall,
        scope.project_id,
        scope.total,
        "正在启动章节流式生成",
    )
    .await;
    let cancel_token = task
        .as_ref()
        .map(|handle| handle.cancel_token.clone())
        .unwrap_or_else(tokio_util::sync::CancellationToken::new);
    let mut report_progress = web_progress_reporter(state, task.clone(), Some(task_id.clone()));
    let (chunk_tx, mut chunk_rx) = tokio::sync::mpsc::unbounded_channel::<(String, u32)>();
    let chunk_state = (*state).clone();
    let chunk_task_id = task_id.clone();
    let chunk_worker = tokio::spawn(async move {
        while let Some((content, index)) = chunk_rx.recv().await {
            emit_workflow_chunk(&chunk_state, &chunk_task_id, &content, index).await;
        }
    });
    let chunk_task = task.clone();
    let mut report_stream = move |delta: &str, index: u32| {
        if chunk_task
            .as_ref()
            .is_some_and(|handle| handle.is_cancelled())
        {
            return Err(a4n_core::error::AppError::Cancelled);
        }
        chunk_tx.send((delta.to_string(), index)).map_err(|_| {
            a4n_core::error::AppError::InvalidArgument("章节流式事件发送通道已关闭".to_string())
        })?;
        Ok(())
    };
    let result = chapter_generation_actions::generate_chapter_stream(
        state,
        user_id,
        chapter_generation_actions::GenerateChapterStreamInput {
            task_id: Some(task_id.clone()),
            ..input
        },
        Some(&mut report_progress),
        Some(&mut report_stream),
        cancel_token,
    )
    .await;
    drop(report_stream);
    let _ = chunk_worker.await;
    let done_task_id = finish_web_task(
        state,
        Some(task_id),
        string_action_result(result),
        "章节正文生成完成",
    )
    .await?;
    Ok(json!(done_task_id))
}

pub(super) async fn prepare_chapter_generation_rag_embedding(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let chapter_number = required_json_i32(
        args,
        "chapterNumber",
        "prepare_chapter_generation_rag_embedding",
    )?;
    let input = chapter_generation_actions::PreviewChapterPromptInput {
        project_id,
        chapter_number,
        writing_notes: optional_string_field(args, "writingNotes")?,
        min_word_count: optional_i32_field(args, "minWordCount")?,
        max_word_count: optional_i32_field(args, "maxWordCount")?,
    };
    Ok(json!(
        chapter_generation_actions::prepare_chapter_generation_rag_embedding(state, user_id, input)
            .await
            .map_err(ApiError::bad_request)?
    ))
}

pub(super) async fn finish_chapter_generation_rag_context(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: chapter_generation_actions::FinishChapterGenerationRagContextInput =
        command_input(args, "finish_chapter_generation_rag_context")?;
    Ok(json!(
        chapter_generation_actions::finish_chapter_generation_rag_context(state, user_id, input)
            .await
            .map_err(ApiError::bad_request)?
    ))
}

pub(super) async fn preview_chapter_prompt(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: chapter_generation_actions::PreviewChapterPromptInput =
        command_input(args, "preview_chapter_prompt")?;
    let preview = chapter_generation_actions::preview_chapter_prompt(state, user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    to_value(preview)
}

macro_rules! chapter_analysis_web_task_command {
    (
        $name:ident,
        $command_name:expr,
        $preflight:path,
        $run:path,
        $start_message:expr,
        $success_message:expr $(,)?
    ) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: a4n_runtime::workflow::GenerateChapterAnalysisInput =
                command_input(args, $command_name)?;
            let ch = $preflight(state, user_id, input.chapter_id)
                .await
                .map_err(ApiError::bad_request)?;
            let task_id = sanitize_optional_task_id(optional_string_field(args, "taskId")?)?;
            let task = register_web_task(
                state,
                task_id.clone(),
                TaskType::LlmCall,
                ch.project_id,
                a4n_runtime::workflow::CHAPTER_ANALYSIS_PROGRESS_TOTAL,
                $start_message,
            )
            .await;
            let cancel_token = task.as_ref().map(|handle| handle.cancel_token.clone());
            let reporter = web_fire_and_forget_progress_reporter(state, task, task_id.clone());
            let progress = a4n_runtime::workflow::ChapterAnalysisProgress::new(
                Some(&reporter),
                cancel_token.as_ref(),
            );
            let result = $run(state, user_id, input.chapter_id, Some(&progress)).await;
            let chapter = finish_web_task(
                state,
                task_id,
                string_action_result(result),
                $success_message,
            )
            .await?;
            Ok(chapter_summary_value(&chapter))
        }
    };
}

macro_rules! chapter_analysis_web_preview_command {
    ($name:ident, $command_name:expr, $preview:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: a4n_runtime::workflow::GenerateChapterAnalysisInput =
                command_input(args, $command_name)?;
            let preview = $preview(state, user_id, input.chapter_id)
                .await
                .map_err(ApiError::bad_request)?;
            to_value(preview)
        }
    };
}

chapter_analysis_web_task_command!(
    generate_chapter_analysis,
    "generate_chapter_analysis",
    a4n_runtime::workflow::preflight_generate_chapter_analysis,
    a4n_runtime::workflow::generate_chapter_analysis_with_progress,
    "正在准备章节结构化分析",
    "章节结构化分析完成",
);

chapter_analysis_web_preview_command!(
    preview_chapter_analysis_request,
    "preview_chapter_analysis_request",
    a4n_runtime::workflow::preview_chapter_analysis_request,
);

chapter_analysis_web_task_command!(
    resume_chapter_analysis_sync,
    "resume_chapter_analysis_sync",
    a4n_runtime::workflow::preflight_resume_chapter_analysis_sync,
    a4n_runtime::workflow::resume_chapter_analysis_sync_with_progress,
    "正在准备继续同步章节分析",
    "章节分析派生数据已继续同步",
);

chapter_analysis_web_preview_command!(
    preview_resume_chapter_analysis_sync_request,
    "preview_resume_chapter_analysis_sync_request",
    a4n_runtime::workflow::preview_resume_chapter_analysis_sync_request,
);

pub(super) async fn list_chapter_scenes(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let chapter_id = i64_arg(args, "chapterId")?;
    Ok(json!(
        chapter_scene_actions::list_chapter_scenes(state, user_id, chapter_id).await?
    ))
}

pub(super) async fn update_chapter_scene(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: chapter_scene_actions::UpdateChapterSceneInput =
        command_input(args, "update_chapter_scene")?;
    Ok(json!(
        chapter_scene_actions::update_chapter_scene(state, user_id, input).await?
    ))
}

pub(super) async fn delete_chapter_scene(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let scene_id = i64_arg(args, "sceneId")?;
    chapter_scene_actions::delete_chapter_scene(state, user_id, scene_id).await?;
    Ok(Value::Null)
}

pub(super) async fn reorder_chapter_scenes(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: chapter_scene_actions::ReorderScenesInput =
        command_input(args, "reorder_chapter_scenes")?;
    Ok(json!(
        chapter_scene_actions::reorder_chapter_scenes(state, user_id, input).await?
    ))
}

pub(super) async fn generate_chapter_scene(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: chapter_scene_actions::GenerateChapterSceneInput =
        command_input(args, "generate_chapter_scene")?;
    let scope = chapter_scene_actions::generate_chapter_scene_task_scope(state, user_id, &input)
        .await
        .map_err(|err| ApiError::bad_request(err.to_string()))?;
    let task_id = sanitize_optional_task_id(optional_string_field(args, "taskId")?)?;
    let task = register_web_task(
        state,
        task_id.clone(),
        TaskType::LlmCall,
        scope.project_id,
        scope.total,
        "正在准备场景生成",
    )
    .await;
    let progress = web_progress_reporter(state, task, task_id.clone());
    let result =
        chapter_scene_actions::generate_chapter_scene(state, user_id, input, &progress).await;
    let scene = finish_web_task(state, task_id, result, "场景已生成").await?;
    Ok(json!(scene))
}

pub(super) async fn preview_chapter_scene_request(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: chapter_scene_actions::GenerateChapterSceneInput =
        command_input(args, "preview_chapter_scene_request")?;
    let preview =
        chapter_scene_actions::preview_chapter_scene_request(state, user_id, &input).await?;
    to_value(preview)
}

pub(super) async fn get_blueprint(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    to_value(blueprint_records_actions::get_blueprint(state, user_id, project_id).await?)
}

macro_rules! blueprint_web_input_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $input = command_input(args, stringify!($name))?;
            to_value($action(state, user_id, input).await?)
        }
    };
}

macro_rules! blueprint_web_preview_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $input = command_input(args, stringify!($name))?;
            to_value($action(state, user_id, input).await?)
        }
    };
}

pub(super) async fn generate_blueprint(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: blueprint_records_actions::GenerateBlueprintInput =
        command_input(args, "generate_blueprint")?;
    let scope = blueprint_records_actions::generate_blueprint_task_scope(state, user_id, &input)
        .await
        .map_err(|err| ApiError::bad_request(err.to_string()))?;
    let task_id = sanitize_optional_task_id(input.task_id.clone())?;
    let task = register_web_task(
        state,
        task_id.clone(),
        TaskType::LlmCall,
        scope.project_id,
        scope.total,
        "启动蓝图生成",
    )
    .await;
    let cancel_token = task.as_ref().map(|handle| handle.cancel_token.clone());
    let progress = web_progress_reporter(state, task, task_id.clone());
    let result = blueprint_records_actions::generate_blueprint(
        state,
        user_id,
        input,
        cancel_token.as_ref(),
        Some(&progress),
    )
    .await;
    let blueprint = finish_web_task(state, task_id, result, "首版蓝图已生成").await?;
    to_value(blueprint)
}

blueprint_web_preview_command!(
    preview_generate_blueprint_request,
    blueprint_records_actions::GenerateBlueprintInput,
    blueprint_records_actions::preview_generate_blueprint_request,
);

blueprint_web_input_command!(
    refine_blueprint,
    blueprint_records_actions::RefineBlueprintInput,
    blueprint_records_actions::refine_blueprint,
);

blueprint_web_preview_command!(
    preview_refine_blueprint_request,
    blueprint_records_actions::RefineBlueprintInput,
    blueprint_records_actions::preview_refine_blueprint_request,
);

pub(super) async fn list_blueprint_versions(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    to_value(blueprint_records_actions::list_blueprint_versions(state, user_id, project_id).await?)
}

blueprint_web_input_command!(
    diff_blueprint_versions,
    blueprint_records_actions::DiffBlueprintVersionsInput,
    blueprint_records_actions::diff_blueprint_versions,
);

blueprint_web_input_command!(
    restore_blueprint_version,
    blueprint_records_actions::RestoreBlueprintVersionInput,
    blueprint_records_actions::restore_blueprint_version,
);

blueprint_web_input_command!(
    update_world_setting,
    blueprint_records_actions::UpdateWorldSettingInput,
    blueprint_records_actions::update_world_setting,
);

pub(super) async fn list_parts(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    to_value(outline_records_actions::list_parts(state, user_id, project_id).await?)
}

macro_rules! outline_web_generation_command {
    (
        $name:ident,
        $input:ty,
        $scope:path,
        $start_message:expr,
        $success_message:expr,
        $action:path $(,)?
    ) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $input = command_input(args, stringify!($name))?;
            let scope = $scope(state, user_id, &input).await?;
            let task_id = sanitize_optional_task_id(input.task_id.clone())?;
            let task = register_web_task(
                state,
                task_id.clone(),
                TaskType::LlmCall,
                scope.project_id,
                scope.total,
                $start_message,
            )
            .await;
            let progress = web_progress_reporter(state, task.clone(), task_id.clone());
            let result = $action(state, user_id, input, Some(&progress)).await;
            let value = finish_web_task(state, task_id, result, $success_message).await?;
            to_value(value)
        }
    };
}

macro_rules! outline_web_preview_command {
    ($name:ident, $input:ty, $action:path $(,)?) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $input = command_input(args, stringify!($name))?;
            to_value($action(state, user_id, input).await?)
        }
    };
}

outline_web_generation_command!(
    generate_parts,
    outline_records_actions::GeneratePartsInput,
    outline_records_actions::generate_parts_task_scope,
    "启动分部大纲生成",
    "分部大纲已生成",
    outline_records_actions::generate_parts,
);

outline_web_preview_command!(
    preview_generate_parts_request,
    outline_records_actions::GeneratePartsInput,
    outline_records_actions::preview_generate_parts_request,
);

pub(super) async fn delete_latest_parts(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let count = u64_arg(args, "count")?;
    outline_records_actions::delete_latest_parts(state, user_id, project_id, count).await?;
    Ok(Value::Null)
}

outline_web_generation_command!(
    regenerate_parts,
    outline_records_actions::RegeneratePartsInput,
    outline_records_actions::regenerate_parts_task_scope,
    "启动分部大纲重生成",
    "已重新生成所有部",
    outline_records_actions::regenerate_parts,
);

outline_web_preview_command!(
    preview_regenerate_parts_request,
    outline_records_actions::RegeneratePartsInput,
    outline_records_actions::preview_regenerate_parts_request,
);

outline_web_generation_command!(
    regenerate_last_part,
    outline_records_actions::RegenerateLastPartInput,
    outline_records_actions::regenerate_last_part_task_scope,
    "正在重新生成最后一部",
    "最后一部已重新生成",
    outline_records_actions::regenerate_last_part,
);

outline_web_preview_command!(
    preview_regenerate_last_part_request,
    outline_records_actions::RegenerateLastPartInput,
    outline_records_actions::preview_regenerate_last_part_request,
);

outline_web_generation_command!(
    regenerate_part_outline,
    outline_records_actions::RegeneratePartOutlineInput,
    outline_records_actions::regenerate_part_outline_task_scope,
    "正在重新生成指定分部",
    "指定分部已重新生成",
    outline_records_actions::regenerate_part_outline,
);

outline_web_preview_command!(
    preview_regenerate_part_outline_request,
    outline_records_actions::RegeneratePartOutlineInput,
    outline_records_actions::preview_regenerate_part_outline_request,
);

outline_web_generation_command!(
    continue_part_outlines,
    outline_records_actions::ContinuePartOutlinesInput,
    outline_records_actions::continue_part_outlines_task_scope,
    "正在续作分部大纲",
    "分部大纲续作完成",
    outline_records_actions::continue_part_outlines,
);

outline_web_preview_command!(
    preview_continue_part_outlines_request,
    outline_records_actions::ContinuePartOutlinesInput,
    outline_records_actions::preview_continue_part_outlines_request,
);

pub(super) async fn get_generation_progress(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    to_value(outline_records_actions::get_generation_progress(state, user_id, project_id).await?)
}

pub(super) async fn list_chapter_outlines(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    to_value(outline_records_actions::list_chapter_outlines(state, user_id, project_id).await?)
}

pub(super) async fn delete_latest_chapter_outlines(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let count = u64_arg(args, "count")?;
    outline_records_actions::delete_latest_chapter_outlines(state, user_id, project_id, count)
        .await?;
    Ok(Value::Null)
}

outline_web_generation_command!(
    generate_part_chapter_outlines,
    outline_records_actions::GeneratePartChapterOutlinesInput,
    outline_records_actions::generate_part_chapter_outlines_task_scope,
    "启动本部章节大纲生成",
    "本部章节大纲已生成",
    outline_records_actions::generate_part_chapter_outlines,
);

outline_web_preview_command!(
    preview_generate_part_chapter_outlines_request,
    outline_records_actions::GeneratePartChapterOutlinesInput,
    outline_records_actions::preview_generate_part_chapter_outlines_request,
);

outline_web_generation_command!(
    batch_generate_part_outlines,
    outline_records_actions::BatchGeneratePartOutlinesInput,
    outline_records_actions::batch_generate_part_outlines_task_scope,
    "启动批量章节大纲生成",
    "批量章节大纲已生成",
    outline_records_actions::batch_generate_part_outlines,
);

outline_web_preview_command!(
    preview_batch_generate_part_outlines_request,
    outline_records_actions::BatchGeneratePartOutlinesInput,
    outline_records_actions::preview_batch_generate_part_outlines_request,
);

outline_web_generation_command!(
    apply_beat_mapping,
    outline_records_actions::ApplyBeatMappingInput,
    outline_records_actions::apply_beat_mapping_task_scope,
    "正在映射节拍表",
    "节拍表映射完成",
    outline_records_actions::apply_beat_mapping,
);

outline_web_preview_command!(
    preview_apply_beat_mapping_request,
    outline_records_actions::ApplyBeatMappingInput,
    outline_records_actions::preview_apply_beat_mapping_request,
);

outline_web_generation_command!(
    generate_chapter_outlines_by_count,
    outline_records_actions::GenerateChapterOutlinesByCountInput,
    outline_records_actions::generate_chapter_outlines_by_count_task_scope,
    "启动章节大纲生成",
    "章节大纲已生成",
    outline_records_actions::generate_chapter_outlines_by_count,
);

outline_web_preview_command!(
    preview_generate_chapter_outlines_by_count_request,
    outline_records_actions::GenerateChapterOutlinesByCountInput,
    outline_records_actions::preview_generate_chapter_outlines_by_count_request,
);

outline_web_generation_command!(
    regenerate_chapter_outline,
    outline_records_actions::RegenerateChapterOutlineInput,
    outline_records_actions::regenerate_chapter_outline_task_scope,
    "正在重新生成章节大纲",
    "章节大纲已重新生成",
    outline_records_actions::regenerate_chapter_outline,
);

outline_web_preview_command!(
    preview_regenerate_chapter_outline_request,
    outline_records_actions::RegenerateChapterOutlineInput,
    outline_records_actions::preview_regenerate_chapter_outline_request,
);

pub(super) async fn update_chapter_outline(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: outline_records_actions::UpdateChapterOutlineInput =
        command_input(args, "update_chapter_outline")?;
    to_value(outline_records_actions::update_chapter_outline(state, user_id, input).await?)
}

pub(super) async fn get_chapter_outlines_pacing(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    Ok(outline_records_actions::get_chapter_outlines_pacing(state, user_id, project_id).await?)
}

pub(super) async fn project_summary_value(
    state: &ServerState,
    user_id: i64,
    project: &novel_project::Model,
) -> Result<Value, ApiError> {
    to_value(project_summary_with_family_for_user(state, user_id, project).await?)
}

pub(super) async fn project_summaries_value(
    state: &ServerState,
    user_id: i64,
    projects: Vec<novel_project::Model>,
) -> Result<Value, ApiError> {
    to_value(
        project_records_actions::project_summaries_with_family_for_user(state, user_id, &projects)
            .await
            .map_err(ApiError::bad_request)?,
    )
}

macro_rules! project_web_input_summary_command {
    ($name:ident, $input:ty, $action:path) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $input = command_input(args, stringify!($name))?;
            let project = $action(state, user_id, input)
                .await
                .map_err(ApiError::bad_request)?;
            project_summary_value(state, user_id, &project).await
        }
    };
}

macro_rules! project_web_input_value_command {
    ($name:ident, $input:ty, $action:path) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let input: $input = command_input(args, stringify!($name))?;
            to_value(
                $action(state, user_id, input)
                    .await
                    .map_err(ApiError::bad_request)?,
            )
        }
    };
}

macro_rules! project_web_project_value_command {
    ($name:ident, $action:path) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let project_id = i64_arg(args, "projectId")?;
            to_value(
                $action(state, user_id, project_id)
                    .await
                    .map_err(ApiError::bad_request)?,
            )
        }
    };
}

macro_rules! project_web_project_node_value_command {
    ($name:ident, $action:path) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let project_id = i64_arg(args, "projectId")?;
            let node_id = string_arg(args, "nodeId")?;
            to_value(
                $action(state, user_id, project_id, node_id)
                    .await
                    .map_err(ApiError::bad_request)?,
            )
        }
    };
}

macro_rules! project_web_project_unit_command {
    ($name:ident, $action:path) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let project_id = i64_arg(args, "projectId")?;
            $action(state, user_id, project_id)
                .await
                .map_err(ApiError::bad_request)?;
            Ok(Value::Null)
        }
    };
}

macro_rules! project_web_project_input_value_command {
    ($name:ident, $input:ty, $action:path) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let project_id = i64_arg(args, "projectId")?;
            let input: $input = command_input(args, stringify!($name))?;
            to_value(
                $action(state, user_id, project_id, input)
                    .await
                    .map_err(ApiError::bad_request)?,
            )
        }
    };
}

macro_rules! project_web_project_string_value_command {
    ($name:ident, $arg:literal, $action:path) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let project_id = i64_arg(args, "projectId")?;
            let value = string_arg(args, $arg)?;
            to_value(
                $action(state, user_id, project_id, &value)
                    .await
                    .map_err(ApiError::bad_request)?,
            )
        }
    };
}

macro_rules! project_web_status_command {
    ($name:ident, $action:path) => {
        pub(super) async fn $name(
            state: &ServerState,
            user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            let project_id = i64_arg(args, "projectId")?;
            let target = string_arg(args, "target")?;
            let project = $action(state, user_id, project_id, &target)
                .await
                .map_err(ApiError::bad_request)?;
            project_summary_value(state, user_id, &project).await
        }
    };
}

project_web_project_input_value_command!(
    update_project,
    project_records_actions::UpdateProjectInput,
    project_records_actions::update_project
);
project_web_project_value_command!(
    get_project_asset_dependency_graph,
    project_records_actions::get_project_asset_dependency_graph
);
project_web_project_node_value_command!(
    get_project_asset_dependency_graph_node_detail,
    project_records_actions::get_project_asset_dependency_graph_node_detail
);
project_web_project_unit_command!(delete_project, project_records_actions::delete_project);
project_web_project_value_command!(
    reset_project_generated_data,
    project_records_actions::reset_project_generated_data
);

project_web_project_value_command!(
    list_project_members,
    project_collaboration_actions::list_project_members
);

project_web_project_input_value_command!(
    invite_project_member,
    project_collaboration_actions::InviteProjectMemberInput,
    project_collaboration_actions::invite_project_member
);

project_web_project_value_command!(
    list_project_invites,
    project_collaboration_actions::list_project_invites
);

pub(super) async fn list_my_project_invites(
    state: &ServerState,
    user_id: i64,
    _args: &Value,
) -> Result<Value, ApiError> {
    to_value(
        project_collaboration_actions::list_my_project_invites(state, user_id)
            .await
            .map_err(ApiError::bad_request)?,
    )
}

pub(super) async fn accept_project_invite(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let invite_id = i64_arg(args, "inviteId")?;
    to_value(
        project_collaboration_actions::accept_project_invite(state, user_id, invite_id)
            .await
            .map_err(ApiError::bad_request)?,
    )
}

pub(super) async fn decline_project_invite(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let invite_id = i64_arg(args, "inviteId")?;
    to_value(
        project_collaboration_actions::decline_project_invite(state, user_id, invite_id)
            .await
            .map_err(ApiError::bad_request)?,
    )
}

pub(super) async fn revoke_project_invite(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let invite_id = i64_arg(args, "inviteId")?;
    to_value(
        project_collaboration_actions::revoke_project_invite(state, user_id, project_id, invite_id)
            .await
            .map_err(ApiError::bad_request)?,
    )
}

pub(super) async fn update_project_member_role(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let member_user_id = i64_arg(args, "memberUserId")?;
    let input: project_collaboration_actions::UpdateProjectMemberRoleInput =
        command_input(args, "update_project_member_role")?;
    to_value(
        project_collaboration_actions::update_project_member_role(
            state,
            user_id,
            project_id,
            member_user_id,
            input,
        )
        .await
        .map_err(ApiError::bad_request)?,
    )
}

pub(super) async fn remove_project_member(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let member_user_id = i64_arg(args, "memberUserId")?;
    project_collaboration_actions::remove_project_member(
        state,
        user_id,
        project_id,
        member_user_id,
    )
    .await
    .map_err(ApiError::bad_request)?;
    Ok(Value::Null)
}

project_web_project_unit_command!(leave_project, project_collaboration_actions::leave_project);

project_web_input_summary_command!(
    import_project_full_upload,
    project_transfer_actions::ImportProjectFullUploadInput,
    project_transfer_actions::import_project_full_upload
);
project_web_input_value_command!(
    export_project_family_data,
    project_transfer_actions::ExportProjectFamilyDataInput,
    project_transfer_actions::export_project_family_data
);
project_web_input_value_command!(
    export_derived_project_data,
    project_transfer_actions::ExportDerivedProjectDataInput,
    project_transfer_actions::export_derived_project_data
);
project_web_input_value_command!(
    export_video_studio_project_data,
    project_transfer_actions::ExportVideoStudioProjectDataInput,
    project_transfer_actions::export_video_studio_project_data
);
project_web_status_command!(
    forward_project_status,
    project_status_actions::forward_project_status
);
project_web_input_summary_command!(
    duplicate_project,
    a4n_runtime::actions::project_duplication::DuplicateProjectInput,
    a4n_runtime::actions::project_duplication::duplicate_project
);

pub(super) async fn branch_project(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let project_id = i64_arg(args, "projectId")?;
    let branch_title = string_arg(args, "branchTitle")?;
    let at_status = string_arg(args, "atStatus")?;
    let project = a4n_runtime::actions::project_duplication::branch_project(
        state,
        user_id,
        project_id,
        branch_title,
        at_status,
    )
    .await
    .map_err(ApiError::bad_request)?;
    project_summary_value(state, user_id, &project).await
}

pub(super) async fn import_project_family_upload(
    state: &ServerState,
    user_id: i64,
    args: &Value,
) -> Result<Value, ApiError> {
    let input: project_transfer_actions::ImportProjectFamilyUploadInput =
        command_input(args, "import_project_family_upload")?;
    let projects = project_transfer_actions::import_project_family_upload(state, user_id, input)
        .await
        .map_err(ApiError::bad_request)?;
    project_summaries_value(state, user_id, projects).await
}

project_web_input_summary_command!(
    import_video_studio_project_upload,
    project_transfer_actions::ImportVideoStudioProjectUploadInput,
    project_transfer_actions::import_video_studio_project_upload
);

project_web_status_command!(
    rollback_project_status,
    project_status_actions::rollback_project_status
);
project_web_project_string_value_command!(
    preview_rollback,
    "targetStatus",
    project_status_actions::preview_rollback
);
project_web_project_value_command!(
    get_project_status_history,
    project_status_actions::get_project_status_history
);
