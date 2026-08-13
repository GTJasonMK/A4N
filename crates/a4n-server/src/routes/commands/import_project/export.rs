use super::super::{
    ApiError, command_input, export_files_actions, finish_web_task, register_web_task,
    reject_unknown_command_args, sanitize_task_id, string_arg,
    web_fire_and_forget_progress_reporter, web_progress_reporter,
};
use crate::state::ServerState;
use a4n_api_contract::export::{BatchExportInput, ExportProjectInput, ExportPublicationInput};
use a4n_core::event::channel::TaskKind;
use serde_json::{Value, json};

macro_rules! export_web_read_command {
    (
        $name:ident($input_ty:ty),
        $total:expr,
        $start_message:expr,
        $progress_message:expr,
        $success_message:expr,
        |$state:ident, $user_id:ident, $input:ident| $run:expr $(,)?
    ) => {
        pub(in crate::routes::commands) async fn $name(
            $state: &ServerState,
            $user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            reject_unknown_command_args(args, stringify!($name), &["input", "taskId"])?;
            let $input: $input_ty = command_input(args, stringify!($name))?;
            let project_id = $input.project_id;
            let total = $total;
            let task_id = sanitize_task_id(string_arg(args, "taskId")?)?;
            let task = register_web_task(
                $state,
                $user_id,
                Some(task_id.clone()),
                TaskKind::ExportGeneration,
                project_id,
                total,
                $start_message,
            )
            .expect("required export task ID must register a task");
            let progress = web_progress_reporter(Some(task));
            let result = async {
                progress(0, total, $progress_message)?;
                let value = $run.await?;
                progress(total, total, $success_message)?;
                Ok(value)
            }
            .await;
            let value = finish_web_task($state, Some(task_id), result)?;
            Ok(json!(value))
        }
    };
}

fn batch_progress_total(input: &BatchExportInput) -> Result<u32, ApiError> {
    u32::try_from(input.formats.len())
        .ok()
        .and_then(|count| count.checked_add(1))
        .ok_or_else(|| ApiError::bad_request("批量导出格式数量超出 u32 范围"))
}

macro_rules! export_web_progress_command {
    (
        $name:ident($input_ty:ty),
        $total:expr,
        $start_message:expr,
        |$state:ident, $user_id:ident, $input:ident, $progress:ident| $run:expr $(,)?
    ) => {
        pub(in crate::routes::commands) async fn $name(
            $state: &ServerState,
            $user_id: i64,
            args: &Value,
        ) -> Result<Value, ApiError> {
            reject_unknown_command_args(args, stringify!($name), &["input", "taskId"])?;
            let $input: $input_ty = command_input(args, stringify!($name))?;
            let project_id = $input.project_id;
            let total = $total;
            let task_id = sanitize_task_id(string_arg(args, "taskId")?)?;
            let task = register_web_task(
                $state,
                $user_id,
                Some(task_id.clone()),
                TaskKind::ExportGeneration,
                project_id,
                total,
                $start_message,
            )
            .expect("required export task ID must register a task");
            let cancel_token = task.cancel_token.clone();
            let reporter = web_fire_and_forget_progress_reporter(Some(task.clone()));
            let $progress =
                export_files_actions::ExportProgress::new(Some(&reporter), Some(&cancel_token));
            let result = $run.await;
            let value = finish_web_task($state, Some(task_id), result)?;
            Ok(json!(value))
        }
    };
}

export_web_read_command!(
    export_txt_data(ExportProjectInput),
    2,
    "正在准备 TXT 导出",
    "正在读取可导出章节",
    "TXT 导出完成",
    |state, user_id, input| export_files_actions::export_txt_data(
        state.runtime(),
        user_id,
        input.project_id,
    ),
);

export_web_read_command!(
    export_markdown_data(ExportPublicationInput),
    2,
    "正在准备 Markdown 导出",
    "正在读取可导出章节",
    "Markdown 导出完成",
    |state, user_id, input| export_files_actions::export_markdown_data(
        state.runtime(),
        user_id,
        input.project_id,
        &input.metadata.title,
        &input.metadata.author,
    ),
);

export_web_progress_command!(
    export_epub_data(ExportPublicationInput),
    4,
    "正在准备 EPUB 导出",
    |state, user_id, input, progress| export_files_actions::export_epub_data(
        state.runtime(),
        user_id,
        input.project_id,
        &input.metadata.title,
        &input.metadata.author,
        &progress,
    ),
);

export_web_progress_command!(
    export_pdf_data(ExportProjectInput),
    5,
    "正在准备 PDF 导出",
    |state, user_id, input, progress| export_files_actions::export_pdf_data(
        state.runtime(),
        user_id,
        input.project_id,
        &progress,
    ),
);

export_web_progress_command!(
    export_batch_data(BatchExportInput),
    batch_progress_total(&input)?,
    "正在准备批量导出",
    |state, user_id, input, progress| export_files_actions::export_batch_data(
        state.runtime(),
        user_id,
        input,
        &progress,
    ),
);

export_web_read_command!(
    export_outline_json_data(ExportProjectInput),
    2,
    "正在准备导出大纲 JSON",
    "正在读取大纲数据",
    "大纲 JSON 导出完成",
    |state, user_id, input| export_files_actions::export_outline_json_data(
        state.runtime(),
        user_id,
        input.project_id,
    ),
);

export_web_read_command!(
    export_characters_json_data(ExportProjectInput),
    2,
    "正在准备导出角色 JSON",
    "正在读取角色数据",
    "角色 JSON 导出完成",
    |state, user_id, input| export_files_actions::export_characters_json_data(
        state.runtime(),
        user_id,
        input.project_id,
    ),
);
