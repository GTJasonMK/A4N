use super::*;

pub(crate) async fn complete_run(
    state: &AgentRuntimeState,
    claimed: &AgentWorkerRun,
    content: String,
    source: &AgentExecutionSnapshot,
    messages: &mut Vec<ChatMessage>,
) -> Result<RunProgress, AppError> {
    let refs = state.result_refs_for_run(claimed).await?;
    let receipt = state
        .complete_agent_worker_run(
            claimed,
            content.clone(),
            content,
            Some(refs),
            execution_input_message_id(source)?,
            source.id,
        )
        .await?;
    if receipt.blockers.is_empty() {
        return Ok(RunProgress::Released);
    }
    if let Some(client_message_id) = stream_client_message_id(claimed.run.id, source) {
        state
            .worker_message_application(claimed)
            .interrupt_assistant_streaming_message(
                claimed,
                client_message_id,
                Some("仍有未完成事项，继续执行".into()),
            )
            .await?;
    }
    messages.push(ChatMessage::system(serde_json::to_string(&receipt)?));
    Ok(RunProgress::Continue)
}

pub(crate) async fn consume_model_stream(
    stream: std::pin::Pin<
        Box<
            dyn futures::Stream<Item = Result<a4n_llm_provider_contract::ChatChunk, AppError>>
                + Send,
        >,
    >,
    message_application: &dyn a4n_agent_runtime_contract::AgentWorkerMessageApplicationPort,
    claimed: &AgentWorkerRun,
    stream_client_message_id: &str,
    cancel: &CancellationToken,
) -> Result<ChatResponse, AppError> {
    let mut stream = stream;
    let mut accumulator = ChatStreamAccumulator::default();
    let mut flushed_chars = 0usize;
    let mut last_flush = Instant::now();
    let mut projection_created = false;

    loop {
        let item = tokio::select! {
            _ = cancel.cancelled() => {
                message_application
                    .interrupt_assistant_streaming_message(
                        claimed,
                        stream_client_message_id.to_string(),
                        Some("Agent run 已取消".into()),
                    )
                    .await?;
                return Err(AppError::Cancelled);
            }
            item = stream.next() => item,
        };
        let Some(item) = item else {
            break;
        };
        let chunk = match item {
            Ok(chunk) => chunk,
            Err(error) => {
                message_application
                    .interrupt_assistant_streaming_message(
                        claimed,
                        stream_client_message_id.to_string(),
                        Some(error.to_string()),
                    )
                    .await?;
                return Err(error);
            }
        };
        let has_stream_data = chunk
            .content
            .as_deref()
            .is_some_and(|content| !content.is_empty())
            || chunk
                .reasoning_content
                .as_deref()
                .is_some_and(|content| !content.is_empty())
            || chunk
                .tool_calls
                .as_ref()
                .is_some_and(|calls| !calls.is_empty());
        accumulator.push(chunk)?;
        let current_chars = accumulator.content().chars().count()
            + accumulator
                .reasoning_content()
                .map_or(0, |content| content.chars().count());
        if should_flush_stream_projection(
            has_stream_data,
            projection_created,
            current_chars,
            flushed_chars,
            last_flush.elapsed(),
        ) {
            message_application
                .upsert_assistant_streaming_message(
                    claimed,
                    stream_client_message_id.to_string(),
                    accumulator.content().to_string(),
                    accumulator.reasoning_content().map(str::to_string),
                )
                .await?;
            projection_created = true;
            flushed_chars = current_chars;
            last_flush = Instant::now();
        }
    }

    let response = match accumulator.finish() {
        Ok(response) => response,
        Err(error) => {
            message_application
                .interrupt_assistant_streaming_message(
                    claimed,
                    stream_client_message_id.to_string(),
                    Some(error.to_string()),
                )
                .await?;
            return Err(error);
        }
    };
    let current_chars = response.message.content.chars().count()
        + response
            .reasoning_content
            .as_deref()
            .map_or(0, |content| content.chars().count());
    if current_chars > flushed_chars {
        message_application
            .upsert_assistant_streaming_message(
                claimed,
                stream_client_message_id.to_string(),
                response.message.content.clone(),
                response.reasoning_content.clone(),
            )
            .await?;
    }
    Ok(response)
}

pub(crate) fn should_flush_stream_projection(
    has_stream_data: bool,
    projection_created: bool,
    current_chars: usize,
    flushed_chars: usize,
    elapsed: Duration,
) -> bool {
    has_stream_data
        && (!projection_created
            || current_chars.saturating_sub(flushed_chars) >= AGENT_STREAM_PROJECTION_MAX_CHARS
            || elapsed >= AGENT_STREAM_PROJECTION_INTERVAL)
}
