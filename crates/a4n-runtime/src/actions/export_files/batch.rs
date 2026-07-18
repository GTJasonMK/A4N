use super::{
    contracts::{BatchExportInput, ExportFileData, ExportProgress},
    epub::export_epub_data,
    pdf::export_pdf_data,
    rules::{batch_export_metadata, normalize_batch_formats},
    scopes::export_batch_task_scope,
    text::{export_markdown_data, export_outline_json_data, export_txt_data},
};
use crate::AppState;

const BATCH_EXPORT_CANCELLED: &str = "批量导出已停止";

pub async fn export_batch_data(
    state: &AppState,
    user_id: i64,
    input: BatchExportInput,
    progress: &ExportProgress<'_>,
) -> Result<Vec<ExportFileData>, String> {
    let project_id = input.project_id;
    export_batch_task_scope(state, user_id, &input).await?;
    let formats = normalize_batch_formats(&input.formats)?;
    let needs_metadata = formats.contains(&"markdown") || formats.contains(&"epub");
    let (title, author) = batch_export_metadata(&input, needs_metadata)?;
    let format_count =
        u32::try_from(formats.len()).map_err(|_| "批量导出格式数量超出 u32 范围".to_string())?;
    let total = format_count
        .checked_add(1)
        .ok_or_else(|| "批量导出进度总量超出 u32 范围".to_string())?;
    progress.progress(0, total, "正在准备批量导出");
    let mut results = Vec::with_capacity(formats.len());
    for (index, format) in formats.iter().enumerate() {
        progress.check_cancelled(BATCH_EXPORT_CANCELLED)?;
        let current = u32::try_from(index)
            .map_err(|_| "批量导出进度索引超出 u32 范围".to_string())?
            .checked_add(1)
            .ok_or_else(|| "批量导出进度索引超出 u32 范围".to_string())?;
        progress.progress(current, total, &format!("正在导出 {format}"));
        let item = match *format {
            "txt" => export_txt_data(state, user_id, project_id).await?,
            "markdown" => {
                export_markdown_data(
                    state,
                    user_id,
                    project_id,
                    title.ok_or_else(|| "批量导出标题不能为空".to_string())?,
                    author.ok_or_else(|| "批量导出作者不能为空".to_string())?,
                )
                .await?
            }
            "epub" => {
                export_epub_data(
                    state,
                    user_id,
                    project_id,
                    title
                        .ok_or_else(|| "批量导出标题不能为空".to_string())?
                        .to_string(),
                    author
                        .ok_or_else(|| "批量导出作者不能为空".to_string())?
                        .to_string(),
                    progress,
                )
                .await?
            }
            "json" => export_outline_json_data(state, user_id, project_id).await?,
            "pdf" => export_pdf_data(state, user_id, project_id, progress).await?,
            _ => unreachable!("normalize_batch_formats must reject unsupported formats"),
        };
        results.push(item);
    }
    progress.progress(total, total, "批量导出完成");
    Ok(results)
}
