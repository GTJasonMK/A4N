use a4n_core::chapter_analysis::CHAPTER_ANALYSIS_SERVICE_PROGRESS_TOTAL;
use a4n_core::error::AppError;
use a4n_llm::provider::ChatRequest;

use crate::structured_llm::{ChatJsonStreamInput, stream_error_or_cancelled};

use crate::OrchestratorContext;

use super::super::super::ChapterAnalysisProgress;
use super::super::super::owner::chapter_analysis_root_field_progress;
use super::super::contracts::ChapterAnalysisStageSpec;

pub(super) async fn chat_chapter_analysis_stage(
    context: &OrchestratorContext,
    request: ChatRequest,
    spec: ChapterAnalysisStageSpec,
    progress: &mut ChapterAnalysisProgress<'_>,
) -> Result<String, AppError> {
    let truncated_message = format!(
        "{} 输出达到当前 max_tokens 上限，返回 JSON 被截断；请在设置页调大“章节分析”输出档位，或减少章节正文长度后重试",
        spec.stage
    );
    // 活体信号与字段级里程碑取自同一个 token 流，二者互补而非替代。
    // 先取出 reporter 引用，避免与下面 &mut progress 的进度回调争用借用。
    let stream_reporter = progress.stream_reporter();
    let mut streamed = String::new();
    let stream_sink = stream_reporter.map(|reporter| {
        move |delta: &str| {
            streamed.push_str(delta);
            reporter(&streamed);
        }
    });
    context
        .chat_json_stream_result_with_sink(
            ChatJsonStreamInput {
                request,
                prompt_name: spec.stage,
                truncated_message: &truncated_message,
                structured_progress: Some(chapter_analysis_root_field_progress(
                    spec.context,
                    spec.fields,
                    spec.progress_span,
                )),
                cancel_token: progress.cancel_token,
                cancelled_message: "章节结构化分析已停止",
            },
            |current, message| {
                progress
                    .report(current, CHAPTER_ANALYSIS_SERVICE_PROGRESS_TOTAL, message)
                    .map_err(|error| error.to_string())
            },
            stream_sink,
        )
        .await
        .map(|result| result.response_content)
        .map_err(|error| stream_error_or_cancelled(error, progress.cancel_token))
}
