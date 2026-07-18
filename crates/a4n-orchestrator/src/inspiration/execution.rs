use a4n_core::error::AppError;
use a4n_llm::provider::{ChatRequest, UsageInfo, is_truncated_finish_reason};
use futures::StreamExt;

use super::validation::validate_inspiration_response_json;
use super::{InspirationStreamInput, OrchestratorContext};

impl OrchestratorContext {
    pub async fn converse_inspiration_with_request(
        &self,
        project_id: i64,
        user_input: &str,
        request: ChatRequest,
        workflow_run_id: Option<i64>,
    ) -> Result<String, AppError> {
        let output = self
            .complete_inspiration_call(project_id, request, workflow_run_id)
            .await?;
        self.save_inspiration_exchange(project_id, user_input, &output)
            .await?;
        Ok(output)
    }

    pub async fn converse_inspiration_stream_with_request<F>(
        &self,
        input: InspirationStreamInput<'_, F>,
    ) -> Result<String, AppError>
    where
        F: FnMut(&str, u32),
    {
        let InspirationStreamInput {
            project_id,
            user_input,
            request,
            workflow_run_id,
            mut on_chunk,
        } = input;
        let request = self
            .prepare_inspiration_checkpoint_request(project_id, request, workflow_run_id)
            .await?;
        if let Some(restored) = self
            .load_inspiration_checkpoint(
                project_id,
                request.workflow_run_id,
                &request.fingerprint_json,
            )
            .await?
        {
            on_chunk(&restored, 0);
            self.save_inspiration_exchange(project_id, user_input, &restored)
                .await?;
            return Ok(restored);
        }

        let mut stream = self.llm.chat_stream(request.request.clone()).await?;
        let mut accumulated = String::new();
        let mut index = 0_u32;
        let mut finish_reason = None::<String>;
        let mut usage = None::<UsageInfo>;
        while let Some(item) = stream.next().await {
            let chunk = item?;
            if let Some(delta) = chunk.content.as_deref().filter(|delta| !delta.is_empty()) {
                accumulated.push_str(delta);
                on_chunk(delta, index);
                index += 1;
            }
            if chunk.usage.is_some() {
                usage = chunk.usage;
            }
            if chunk.finish_reason.is_some() {
                finish_reason = chunk.finish_reason;
                break;
            }
        }
        ensure_inspiration_not_truncated(finish_reason.as_deref())?;
        validate_inspiration_response_json(&accumulated)?;
        self.save_inspiration_checkpoint(
            request.workflow_run_id,
            request.call_key,
            request.fingerprint_json,
            &accumulated,
            finish_reason,
            usage,
        )
        .await?;
        self.save_inspiration_exchange(project_id, user_input, &accumulated)
            .await?;
        Ok(accumulated)
    }

    async fn complete_inspiration_call(
        &self,
        project_id: i64,
        request: ChatRequest,
        workflow_run_id: Option<i64>,
    ) -> Result<String, AppError> {
        let request = self
            .prepare_inspiration_checkpoint_request(project_id, request, workflow_run_id)
            .await?;
        if let Some(restored) = self
            .load_inspiration_checkpoint(
                project_id,
                request.workflow_run_id,
                &request.fingerprint_json,
            )
            .await?
        {
            return Ok(restored);
        }

        let response = self.llm.chat(request.request).await?;
        ensure_inspiration_not_truncated(response.finish_reason.as_deref())?;
        validate_inspiration_response_json(&response.message.content)?;
        self.save_inspiration_checkpoint(
            request.workflow_run_id,
            request.call_key,
            request.fingerprint_json,
            &response.message.content,
            response.finish_reason,
            response.usage,
        )
        .await?;
        Ok(response.message.content)
    }
}

fn ensure_inspiration_not_truncated(finish_reason: Option<&str>) -> Result<(), AppError> {
    if is_truncated_finish_reason(finish_reason) {
        return Err(AppError::InvalidArgument(
            "灵感对话输出达到当前 max_tokens 上限，JSON 可能被截断；请调大标准输出档位或缩短本轮上下文后重试".to_string(),
        ));
    }
    Ok(())
}
