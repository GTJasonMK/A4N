use std::{
    collections::HashMap,
    sync::{
        Mutex,
        atomic::{AtomicI32, Ordering},
    },
    time::{Duration, Instant},
};

use a4n_domain::{
    manga::{
        MANGA_PREPROCESSING_V3, MangaPipelineArtifactDraft, MangaPipelineArtifactStage,
        MangaPreprocessingFacts, MangaPreprocessingSpeech, MangaPreprocessingStep,
        MangaPreprocessingSynthesis, MangaPreprocessingV3, MangaPreprocessingVisuals,
        manga_preprocessing_input_fingerprint,
    },
    source_spans::source_spans_for_content,
    source_text_hash,
};
use a4n_error::AppError;
use a4n_progress::ProgressPhaseKind;
use a4n_prompt::rendering::PromptRenderer;
use a4n_service_protocol::ProgressEvent;
use serde_json::Value;
use tokio_util::sync::CancellationToken;

use crate::{OrchestratorContext, source_markers::SourceMarkerMap};

use super::super::prompt::{MangaPreprocessingMessages, MangaRenderTraceInput};
use super::contracts::manga_stage_output_exists_error;

const PREPROCESSING_TOTAL_STEPS: i32 = MangaPreprocessingStep::ALL.len() as i32 + 1;

pub(super) struct MangaPreprocessingInput<'a> {
    pub project_id: i64,
    pub chapter_id: i64,
    pub chapter_number: i32,
    pub chapter_content: &'a str,
    pub prompt_renderer: &'a dyn PromptRenderer,
    pub reference_assets: Value,
    pub task_id: &'a str,
    pub workflow_run_id: i64,
    pub resume_from_checkpoint: bool,
    pub cancel_token: Option<&'a CancellationToken>,
    pub progress_reporter: Option<&'a (dyn Fn(ProgressEvent) + Send + Sync)>,
    pub stream_reporter: Option<&'a (dyn Fn(&str, &str) + Send + Sync)>,
}

pub(super) async fn run_manga_preprocessing(
    context: &OrchestratorContext,
    input: MangaPreprocessingInput<'_>,
) -> Result<MangaPreprocessingV3, AppError> {
    let source_spans = source_spans_for_content(
        input.chapter_id,
        input.chapter_number,
        input.chapter_content,
    );
    let source_markers = SourceMarkerMap::from_source_spans(&source_spans)?;
    let input_fingerprint = manga_preprocessing_input_fingerprint(
        &source_text_hash(input.chapter_content),
        input.chapter_number,
        &input.reference_assets,
    );
    if let Some(artifact) = context
        .manga_pipeline_artifact(
            input.project_id,
            input.chapter_id,
            MangaPipelineArtifactStage::Preprocessing,
            None,
        )
        .await?
    {
        if !input.resume_from_checkpoint
            || artifact.stale
            || artifact.input_fingerprint != input_fingerprint
        {
            return Err(manga_stage_output_exists_error(
                MangaPipelineArtifactStage::Preprocessing,
                None,
            ));
        }
        let preprocessing: MangaPreprocessingV3 = serde_json::from_value(artifact.payload)
            .map_err(|error| AppError::InvalidArgument(format!("漫画预处理产物无效: {error}")))?;
        preprocessing
            .validate()
            .map_err(AppError::InvalidArgument)?;
        let progress = PreprocessingProgress::new(context, &input);
        progress
            .report(
                None,
                "已复用完整预处理产物",
                PREPROCESSING_TOTAL_STEPS,
                ProgressPhaseKind::Save,
            )
            .await?;
        return Ok(preprocessing);
    }

    let progress = PreprocessingProgress::new(context, &input);
    let messages = MangaPreprocessingMessages {
        chapter_content: input.chapter_content,
        source_spans: &source_spans,
        source_markers: &source_markers,
        reference_assets: &input.reference_assets,
    };
    let facts: MangaPreprocessingFacts = run_step(
        &progress,
        MangaPreprocessingStep::Facts,
        messages.build(MangaPreprocessingStep::Facts, None, None, None)?,
        &source_markers,
        |value: &MangaPreprocessingFacts| value.validate(),
    )
    .await?;
    // Join both branches so a sibling's validated checkpoint survives a failure.
    let (speech, visuals) = futures::join!(
        run_step(
            &progress,
            MangaPreprocessingStep::Speech,
            messages.build(MangaPreprocessingStep::Speech, Some(&facts), None, None)?,
            &source_markers,
            |value: &MangaPreprocessingSpeech| value.validate(&facts)
        ),
        run_step(
            &progress,
            MangaPreprocessingStep::Visuals,
            messages.build(MangaPreprocessingStep::Visuals, Some(&facts), None, None)?,
            &source_markers,
            |value: &MangaPreprocessingVisuals| value.validate(&facts)
        ),
    );
    let (speech, visuals) = (speech?, visuals?);
    let synthesis: MangaPreprocessingSynthesis = run_step(
        &progress,
        MangaPreprocessingStep::Synthesis,
        messages.build(
            MangaPreprocessingStep::Synthesis,
            Some(&facts),
            Some(&speech),
            Some(&visuals),
        )?,
        &source_markers,
        |value: &MangaPreprocessingSynthesis| value.validate(),
    )
    .await?;
    let preprocessing = MangaPreprocessingV3 {
        schema_version: MANGA_PREPROCESSING_V3.into(),
        source_events: facts.source_events,
        characters: facts.characters,
        speech: speech.speech,
        visual_anchors: visuals.visual_anchors,
        items: visuals.items,
        comic_synthesis: synthesis.comic_synthesis,
    };
    progress
        .report(
            None,
            "正在校验并保存完整预处理资料",
            0,
            ProgressPhaseKind::Validate,
        )
        .await?;
    preprocessing
        .validate()
        .map_err(AppError::InvalidArgument)?;
    if input
        .cancel_token
        .is_some_and(CancellationToken::is_cancelled)
    {
        return Err(AppError::Cancelled);
    }

    context
        .create_manga_pipeline_artifact(MangaPipelineArtifactDraft {
            project_id: input.project_id,
            chapter_id: input.chapter_id,
            stage: MangaPipelineArtifactStage::Preprocessing,
            page_number: None,
            input_fingerprint,
            payload: serde_json::to_value(&preprocessing)?,
            stale: false,
        })
        .await?;
    progress
        .report(None, "漫画预处理完成", 1, ProgressPhaseKind::Save)
        .await?;
    Ok(preprocessing)
}

struct PreprocessingProgress<'a> {
    context: &'a OrchestratorContext,
    input: &'a MangaPreprocessingInput<'a>,
    lock: tokio::sync::Mutex<()>,
    completed: AtomicI32,
}

impl<'a> PreprocessingProgress<'a> {
    fn new(context: &'a OrchestratorContext, input: &'a MangaPreprocessingInput<'a>) -> Self {
        Self {
            context,
            input,
            lock: tokio::sync::Mutex::new(()),
            completed: AtomicI32::new(0),
        }
    }

    fn notify(
        &self,
        step: Option<MangaPreprocessingStep>,
        message: &str,
        phase_kind: ProgressPhaseKind,
    ) {
        if let Some(reporter) = self.input.progress_reporter {
            reporter(ProgressEvent {
                stage_code: "manga_preprocessing".into(),
                phase_kind,
                current: Some(self.completed.load(Ordering::SeqCst) as u64),
                total: Some(PREPROCESSING_TOTAL_STEPS as u64),
                message: message.into(),
                unit_label: Some("阶段".into()),
                item_key: step.map(|step| step.prompt_name().into()),
                item_index: None,
                item_total: None,
                severity: a4n_progress::ProgressSeverity::Info,
                estimated: false,
            });
        }
    }

    async fn report(
        &self,
        step: Option<MangaPreprocessingStep>,
        message: &str,
        completed: i32,
        phase_kind: ProgressPhaseKind,
    ) -> Result<(), AppError> {
        let _guard = self.lock.lock().await;
        let count = self.completed.load(Ordering::SeqCst) + completed;
        self.context
            .record_manga_pipeline_progress_with_total(
                self.input.workflow_run_id,
                self.input.task_id,
                "preprocessing",
                count,
                PREPROCESSING_TOTAL_STEPS,
                message,
            )
            .await?;
        self.completed.store(count, Ordering::SeqCst);
        self.notify(step, message, phase_kind);
        Ok(())
    }
}

async fn run_step<T: serde::Serialize + serde::de::DeserializeOwned>(
    progress: &PreprocessingProgress<'_>,
    step: MangaPreprocessingStep,
    user_content: a4n_llm_context::LlmContextManifest,
    source_markers: &SourceMarkerMap,
    validate: impl Fn(&T) -> Result<(), String>,
) -> Result<T, AppError> {
    let input = progress.input;
    if input
        .cancel_token
        .is_some_and(CancellationToken::is_cancelled)
    {
        return Err(AppError::Cancelled);
    }
    let started = Instant::now();
    progress
        .report(
            Some(step),
            &format!("{}：正在生成或读取匹配检查点", step.title()),
            0,
            ProgressPhaseKind::StructuredOutput,
        )
        .await?;
    let stream_progress = Mutex::new((Instant::now(), 0usize));
    let correction = || {
        *stream_progress
            .lock()
            .expect("preprocessing stream progress") = (Instant::now(), 0);
        progress.notify(
            Some(step),
            &format!("{}：输出未通过校验，正在进行唯一一次纠正", step.title()),
            ProgressPhaseKind::Validate,
        )
    };
    let stream = |stream_id: &str, delta: &str| {
        if input
            .cancel_token
            .is_some_and(CancellationToken::is_cancelled)
        {
            return;
        }
        if let Some(reporter) = input.stream_reporter {
            reporter(stream_id, delta);
        }
        let mut state = stream_progress
            .lock()
            .expect("preprocessing stream progress");
        state.1 += delta.chars().count();
        if state.0.elapsed() >= Duration::from_secs(1) {
            progress.notify(
                Some(step),
                &format!(
                    "{}：{}，用时 {:.1} 秒，已输出 {} 字符",
                    step.title(),
                    if stream_id.ends_with(".correction") {
                        "正在纠正"
                    } else {
                        "正在生成"
                    },
                    started.elapsed().as_secs_f64(),
                    state.1,
                ),
                ProgressPhaseKind::ModelStream,
            );
            state.0 = Instant::now();
        }
    };
    let result = progress
        .context
        .render_and_validate_manga_call_observed(
            MangaRenderTraceInput {
                prompt_renderer: input.prompt_renderer,
                prompt_name: step.prompt_name(),
                project_id: input.project_id,
                workflow_run_id: input.workflow_run_id,
                call_key: step.call_key(input.chapter_id),
                call_order: step.order(),
                restore_from_checkpoint: input.resume_from_checkpoint,
                variables: &HashMap::new(),
                user_content,
                temperature: 0.3,
                cancel_token: input.cancel_token,
                stream_id: step.prompt_name().into(),
                stream_reporter: Some(&stream),
            },
            |value| {
                let value = source_markers.decode_output(value)?;
                let output: T = serde_json::from_value(value)?;
                validate(&output).map_err(AppError::InvalidArgument)?;
                Ok(output)
            },
            Some(&correction),
        )
        .await;
    let (trace, output) = match result {
        Ok(value) => value,
        Err(AppError::Cancelled) => return Err(AppError::Cancelled),
        Err(error) => {
            progress.notify(
                Some(step),
                &format!("{}失败：{error}", step.title()),
                ProgressPhaseKind::Validate,
            );
            return Err(error);
        }
    };
    if input
        .cancel_token
        .is_some_and(CancellationToken::is_cancelled)
    {
        return Err(AppError::Cancelled);
    }
    let usage = trace
        .usage
        .as_ref()
        .map(|usage| {
            format!(
                "；Token {}（输入 {} / 输出 {}）",
                usage.total_tokens, usage.prompt_tokens, usage.completion_tokens
            )
        })
        .unwrap_or_default();
    progress
        .report(
            Some(step),
            &format!(
                "{}{}，用时 {:.1} 秒，输出 {} 字符{usage}",
                step.title(),
                if trace.restored_from_checkpoint {
                    "已复用检查点"
                } else {
                    "生成并校验完成"
                },
                started.elapsed().as_secs_f64(),
                trace.response_text.chars().count(),
            ),
            1,
            ProgressPhaseKind::Save,
        )
        .await?;
    Ok(output)
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;

    #[tokio::test]
    async fn progress_only_advances_after_a_successful_workflow_write() {
        let database = Arc::new(a4n_migration::test_support::connect_test_database().await);
        let context = crate::test_support::TestOrchestratorBuilder::new(database.clone())
            .build()
            .await;
        let workflow = a4n_db::workflow::upsert_workflow_run(
            database.as_ref(),
            serde_json::from_value(serde_json::json!({
                "business_key": "manga-preprocessing-progress-test",
                "workflow_kind": "manga_pipeline", "status": "running", "can_resume": true
            }))
            .unwrap(),
        )
        .await
        .unwrap();
        let events = Mutex::new(Vec::new());
        let reporter = |event| events.lock().unwrap().push(event);
        let input = MangaPreprocessingInput {
            project_id: 1,
            chapter_id: 1,
            chapter_number: 1,
            chapter_content: "原文",
            prompt_renderer: &context,
            reference_assets: serde_json::json!({}),
            task_id: "manga-progress-test",
            workflow_run_id: workflow.id,
            resume_from_checkpoint: false,
            cancel_token: None,
            progress_reporter: Some(&reporter),
            stream_reporter: None,
        };
        let progress = PreprocessingProgress::new(&context, &input);
        let (facts, visuals) = futures::join!(
            progress.report(
                Some(MangaPreprocessingStep::Facts),
                "事实完成",
                1,
                ProgressPhaseKind::Save
            ),
            progress.report(
                Some(MangaPreprocessingStep::Visuals),
                "视觉完成",
                1,
                ProgressPhaseKind::Save
            ),
        );
        facts.unwrap();
        visuals.unwrap();
        assert_eq!(progress.completed.load(Ordering::SeqCst), 2);
        context
            .complete_workflow_run_validated(workflow.id)
            .await
            .unwrap();
        assert!(
            progress
                .report(None, "写入失败", 1, ProgressPhaseKind::Save)
                .await
                .is_err()
        );
        progress.notify(None, "保留已保存的进度", ProgressPhaseKind::Validate);
        let events = events.lock().unwrap();
        assert_eq!(
            events.iter().map(|event| event.current).collect::<Vec<_>>(),
            [Some(1), Some(2), Some(2)]
        );
        assert!(
            events
                .iter()
                .all(|event| event.total == Some(PREPROCESSING_TOTAL_STEPS as u64))
        );
    }
}
