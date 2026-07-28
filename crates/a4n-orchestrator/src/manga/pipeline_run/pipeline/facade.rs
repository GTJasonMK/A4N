use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_manga_prompt;

use super::super::super::pipeline_status::MangaPipelineStatusInput;
use super::super::super::{MangaGeneratePanelsInput, OrchestratorContext};

impl OrchestratorContext {
    /// 生成整页漫画提示词：完整 8 步流水线。
    ///
    /// 每步把上一步的 JSON 输出拼到下一步的 user message 中传给 LLM；prompt 模板
    /// （system message）只描述抽取/规划/编排的策略，不依赖 Tera 变量插值。
    pub(crate) async fn generate_panels(
        &self,
        input: MangaGeneratePanelsInput<'_>,
    ) -> Result<Vec<chapter_manga_prompt::Model>, AppError> {
        let chapter_id = input.chapter_id;
        let project_id = input.project_id;
        let task_id = input.task_id;
        let result = self.generate_panels_inner(input).await;

        match result {
            Ok(prompts) => Ok(prompts),
            Err(err) => {
                let is_cancelled = matches!(&err, AppError::Cancelled);
                let original_error = err.to_string();
                let (pipeline_status, stage, message) = if is_cancelled {
                    ("cancelled", "cancelled", "漫画流水线已取消".to_string())
                } else {
                    ("failed", "failed", original_error.clone())
                };
                let (current, total) = self
                    .pipeline_progress(project_id, chapter_id)
                    .await
                    .map_err(|progress_err| {
                        AppError::InvalidArgument(format!(
                            "{original_error}; 读取漫画流水线进度失败: {progress_err}"
                        ))
                    })?;
                self.set_pipeline_status(MangaPipelineStatusInput {
                    chapter_id,
                    project_id,
                    pipeline_status,
                    stage,
                    current,
                    total,
                    message: &message,
                    task_id,
                    allow_cancel_override: true,
                })
                .await
                .map_err(|status_err| {
                    AppError::InvalidArgument(format!(
                        "{original_error}; 记录漫画流水线终止状态失败: {status_err}"
                    ))
                })?;
                Err(err)
            }
        }
    }
}
