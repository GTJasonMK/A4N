use std::sync::Mutex;

use a4n_api_contract::{
    generation_tasks::GenerationTaskHandle,
    shooting_script::{ShootingScriptGenerationMode, StartChapterShootingScriptGenerationInput},
};
use a4n_error::AppError;
use a4n_progress::ProgressPhaseKind;
use a4n_service_protocol::{GenerationError, GenerationResultRef};
use tokio::sync::mpsc;

use crate::{
    OrchestratorContext,
    generation::{
        GenerationProgressUpdate, checkpoint::GenerationCheckpointMetadata,
        events::GenerationEventPublisher, generation_public_error, publish_step_progress,
    },
};

const OPERATION: &str = "chapter_shooting_script";

impl OrchestratorContext {
    pub(super) async fn run_shooting_script_generation_task(
        &self,
        user_id: i64,
        input: StartChapterShootingScriptGenerationInput,
        handle: GenerationTaskHandle,
    ) -> Result<(), String> {
        let Some(execution) = self.acquire_generation_task_execution(&handle).await
            .map_err(|error| error.to_string())? else { return Ok(()); };
        let cancellation = execution.permit.cancellation_token();
        let (progress_sender, progress_receiver) = mpsc::unbounded_channel();
        let progress_worker = tokio::spawn(publish_step_progress(
            progress_receiver, execution.publisher.clone(), execution.checkpoint_writer.clone(),
            execution.session.clone(), OPERATION, OPERATION, ProgressPhaseKind::ModelStream, 100,
        ));
        let reporter_cancel = cancellation.clone();
        let reporter = move |current, total, message: &str| {
            if reporter_cancel.is_cancelled() { return Err(AppError::Cancelled); }
            progress_sender.send(GenerationProgressUpdate { current, total, message: message.into() })
                .map_err(|_| AppError::InvalidState {
                    current: "shooting_progress_publisher_closed".into(),
                    operation: "发布拍摄剧本进度".into(),
                })
        };

        let (stream_sender, mut stream_receiver) = mpsc::unbounded_channel::<String>();
        let publisher = execution.publisher.clone();
        let writer = execution.checkpoint_writer.clone();
        let session = execution.session.clone();
        let stream_worker = tokio::spawn(async move {
            while let Some(delta) = stream_receiver.recv().await {
                publisher.text_delta("raw_json", &delta).await
                    .map_err(|error| generation_public_error(error.code(), OPERATION, error))?;
                writer.persist_if_due(&session, GenerationCheckpointMetadata {
                    step_key: Some(OPERATION.into()), ..Default::default()
                }, false).await
                    .map_err(|error| generation_public_error(error.code(), OPERATION, error))?;
            }
            Ok::<_, GenerationError>(())
        });
        let previous = Mutex::new(String::new());
        let stream_cancel = cancellation.clone();
        let stream_reporter = move |text: &str| {
            let delta = stream_delta(&mut previous.lock().expect("shooting stream mutex poisoned"), text);
            if !delta.is_empty() && stream_sender.send(delta).is_err() {
                // The publisher's error is reported below; stop the provider instead of losing output.
                stream_cancel.cancel();
            }
        };

        let result = match input.mode {
            ShootingScriptGenerationMode::Full => self.generate_chapter_shooting_script_for_user_validated(
                user_id, input.generation_input(), &reporter, Some(&stream_reporter), Some(&cancellation), Some(&handle.task_id),
            ).await.map(|result| GenerationResultRef {
                kind: "chapter_shooting_script".into(),
                id: result.shooting_script.record.id.to_string(),
                revision: result.shooting_script.record.revision.to_string(),
            }),
            ShootingScriptGenerationMode::NextScene => self.generate_next_chapter_shooting_script_unit_for_user_validated(
                user_id, input.generation_input(), &reporter, Some(&stream_reporter), Some(&cancellation), Some(&handle.task_id),
            ).await.and_then(|result| {
                if let Some(resolved) = result.resolved {
                    return Ok(GenerationResultRef {
                        kind: "chapter_shooting_script".into(),
                        id: resolved.shooting_script.record.id.to_string(),
                        revision: resolved.shooting_script.record.revision.to_string(),
                    });
                }
                Ok(GenerationResultRef {
                    kind: "chapter_shooting_script_progress".into(),
                    id: result.workflow_run_id.to_string(),
                    revision: a4n_db::generation::canonical_json_sha256(&serde_json::to_value(result.script)?)?,
                })
            }),
        };
        drop(reporter);
        drop(stream_reporter);
        let (progress_result, stream_result) = tokio::join!(progress_worker, stream_worker);
        for published in [progress_result, stream_result] {
            let error = match published {
                Ok(Ok(())) => continue,
                Ok(Err(error)) => error,
                Err(error) => generation_public_error("generation_event_invalid", OPERATION, error),
            };
            return Err(execution.fail(error).await);
        }
        let result = match result {
            Ok(result) => result,
            Err(error) => {
                let code = match error {
                    AppError::Cancelled => "generation_interrupted",
                    AppError::Forbidden(_) | AppError::Unauthorized(_) => "generation_task_forbidden",
                    AppError::InvalidArgument(_) | AppError::InvalidState { .. } | AppError::NotFound(_) => "generation_request_invalid",
                    AppError::Database(_) | AppError::Serialization(_) => "generation_persist_failed",
                    _ => "provider_job_failed",
                };
                return Err(execution.fail(generation_public_error(code, OPERATION, error)).await);
            }
        };
        execution.complete_with_result(OPERATION, result, "拍摄剧本已保存".into(), vec![], None).await
    }
}

// The existing workflow callback carries each call's cumulative text; generation streams append deltas.
fn stream_delta(previous: &mut String, text: &str) -> String {
    let delta = match text.strip_prefix(previous.as_str()) {
        Some(delta) => delta.to_string(),
        None => format!("\n{text}"),
    };
    previous.clear();
    previous.push_str(text);
    delta
}

#[cfg(test)]
mod tests {
    use super::stream_delta;

    #[test]
    fn cumulative_streams_preserve_unicode_and_separate_model_calls() {
        let mut previous = String::new();
        assert_eq!(stream_delta(&mut previous, "{\"场景\":"), "{\"场景\":");
        assert_eq!(stream_delta(&mut previous, "{\"场景\":1}"), "1}");
        assert_eq!(stream_delta(&mut previous, "{\"场景\":1}"), "");
        assert_eq!(stream_delta(&mut previous, "{"), "\n{");
        assert_eq!(stream_delta(&mut previous, "{\"场景\":2}"), "\"场景\":2}");
    }
}
