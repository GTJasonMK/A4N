use std::{
    sync::atomic::{AtomicI32, Ordering},
    time::Instant,
};

use a4n_error::AppError;
use a4n_llm_context::LlmContextManifest;
use a4n_progress::ProgressPhaseKind;
use serde_json::Value;
use tokio::sync::Mutex;

use super::super::super::{prompt::MangaRenderTraceInput, state::MangaPagePlanningStep};
use super::super::contracts::MangaRunContext;

pub(super) struct PagePlanningProgress<'a, 'context> {
    context: &'a MangaRunContext<'context>,
    total: i32,
    completed: AtomicI32,
    lock: Mutex<()>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::sync::{Arc, Mutex as StdMutex};

    #[tokio::test]
    async fn progress_events_validate_and_failed_persistence_never_advances_count() {
        let database = Arc::new(a4n_migration::test_support::connect_test_database().await);
        let service = crate::test_support::TestOrchestratorBuilder::new(database.clone())
            .build()
            .await;
        let workflow = a4n_db::workflow::upsert_workflow_run(database.as_ref(), serde_json::from_value(json!({
            "business_key":"manga-page-planning-progress-test", "workflow_kind":"manga_pipeline", "status":"running", "can_resume":true
        })).unwrap()).await.unwrap();
        let (adaptation, spans, _) = crate::manga::pipeline::json::tests::page_planning_fixture();
        let events = StdMutex::new(Vec::new());
        let reporter = |event| events.lock().unwrap().push(event);
        let context = MangaRunContext {
            service:&service,chapter_id:9,chapter_number:1,page_number:None,project_id:2,user_id:1,
            adaptation, prompt_renderer:&service,task_id:"page-progress-test",workflow_run_id:workflow.id,
            resume_from_checkpoint:false,stop_after:None,run_stage:None,image_size:None,image_generator:None,cancel_token:None,
            progress_reporter:Some(&reporter),stream_reporter:None,chapter_content:"正文".into(),
            source_markers:crate::source_markers::SourceMarkerMap::from_source_spans(&spans).unwrap(), source_spans:spans,
            variables:Default::default(),reference_assets:json!({}),reference_assets_metadata:json!({}), available_reference_images:vec![],prompt_fingerprint:json!({}),
            source_metadata:serde_json::from_value(json!({"root_project_id":1,"source_chapter_id":9,"source_snapshot_id":null,"source_snapshot_chapter_id":null,"source_revision_key":"source","content_sha256":"sha","adaptation_plan_id":1,"adaptation_plan_revision":1,"adaptation_plan_sha256":"plan"})).unwrap(),
        };
        let progress = PagePlanningProgress::new(&context, 4);
        progress
            .report(
                Some(MangaPagePlanningStep::Outline),
                "页纲完成",
                1,
                ProgressPhaseKind::Save,
            )
            .await
            .unwrap();
        let (first, second) = futures::join!(
            progress.report(
                Some(MangaPagePlanningStep::Content(0)),
                "第1页完成",
                1,
                ProgressPhaseKind::Save
            ),
            progress.report(
                Some(MangaPagePlanningStep::Content(1)),
                "第2页完成",
                1,
                ProgressPhaseKind::Save
            ),
        );
        first.unwrap();
        second.unwrap();
        service
            .complete_workflow_run_validated(workflow.id)
            .await
            .unwrap();
        assert!(
            progress
                .report(None, "保存进度失败", 1, ProgressPhaseKind::Save)
                .await
                .is_err()
        );
        progress.notify(None, "保留真实计数", ProgressPhaseKind::Validate);
        let events = events.lock().unwrap();
        assert_eq!(
            events.iter().map(|event| event.current).collect::<Vec<_>>(),
            [Some(1), Some(2), Some(3), Some(3)]
        );
        for event in events.iter() {
            let envelope = a4n_service_protocol::GenerationTaskEvent {
                schema_version: a4n_service_protocol::GENERATION_EVENT_SCHEMA_VERSION,
                task_id: "550e8400-e29b-41d4-a716-446655440000".into(),
                attempt: 1,
                sequence: 2,
                emitted_at_ms: 0,
                event: a4n_service_protocol::GenerationEvent::Progress(event.clone()),
            };
            envelope.validate().unwrap();
        }
    }
}

impl<'a, 'context> PagePlanningProgress<'a, 'context> {
    pub(super) fn new(context: &'a MangaRunContext<'context>, total: i32) -> Self {
        Self {
            context,
            total,
            completed: AtomicI32::new(0),
            lock: Mutex::new(()),
        }
    }

    fn notify(&self, step: Option<MangaPagePlanningStep>, message: &str, phase: ProgressPhaseKind) {
        self.context.publish_progress(
            "page_slots",
            phase,
            self.completed.load(Ordering::Acquire),
            self.total,
            message,
            "步骤",
            step.map(|step| step.key()),
            step.and_then(|step| match step {
                MangaPagePlanningStep::Content(index) => Some(index as u64),
                _ => None,
            }),
            step.and_then(|step| match step {
                MangaPagePlanningStep::Content(_) => {
                    self.context.adaptation.plan.page_count.map(u64::from)
                }
                _ => None,
            }),
        );
    }

    pub(super) async fn report(
        &self,
        step: Option<MangaPagePlanningStep>,
        message: &str,
        increment: i32,
        phase: ProgressPhaseKind,
    ) -> Result<(), AppError> {
        let _guard = self.lock.lock().await;
        let current = self.completed.load(Ordering::Acquire) + increment;
        let message = if matches!(step, Some(MangaPagePlanningStep::Content(_))) && increment == 1 {
            format!(
                "页面内容已完成 {}/{} 页；{message}",
                current - 1,
                self.context
                    .adaptation
                    .plan
                    .validate_manga_page_count()
                    .map_err(AppError::InvalidArgument)?
            )
        } else {
            message.into()
        };
        self.context
            .service
            .record_manga_pipeline_progress_with_total(
                self.context.workflow_run_id,
                self.context.task_id,
                "page_slots",
                current,
                self.total,
                &message,
            )
            .await?;
        self.completed.store(current, Ordering::Release);
        self.notify(step, &message, phase);
        Ok(())
    }

    pub(super) async fn run_call<T: serde::Serialize>(
        &self,
        step: MangaPagePlanningStep,
        user_content: LlmContextManifest,
        mut validate: impl FnMut(&Value) -> Result<T, AppError>,
    ) -> Result<T, AppError> {
        self.context.check_cancelled()?;
        self.report(
            Some(step),
            &format!("{}：生成中或读取匹配检查点", step.title()),
            0,
            ProgressPhaseKind::StructuredOutput,
        )
        .await?;
        let started = Instant::now();
        let correction = || {
            self.notify(
                Some(step),
                &format!("{}：输出未通过校验，正在进行唯一一次纠正", step.title()),
                ProgressPhaseKind::Validate,
            )
        };
        let result = self
            .context
            .service
            .render_and_validate_manga_call_observed(
                MangaRenderTraceInput {
                    prompt_renderer: self.context.prompt_renderer,
                    prompt_name: step.prompt_name(),
                    project_id: self.context.project_id,
                    workflow_run_id: self.context.workflow_run_id,
                    call_key: step.call_key(self.context.chapter_id),
                    call_order: step.call_order()?,
                    restore_from_checkpoint: self.context.resume_from_checkpoint,
                    variables: &self.context.variables,
                    user_content,
                    temperature: 0.3,
                    cancel_token: self.context.cancel_token,
                    stream_id: step.key(),
                    stream_reporter: self.context.stream_reporter,
                },
                |value| validate(&self.context.source_markers.decode_output(value)?),
                Some(&correction),
            )
            .await;
        let (trace, output) = match result {
            Ok(value) => value,
            Err(AppError::Cancelled) => return Err(AppError::Cancelled),
            Err(error) => {
                let message = format!("{}失败：{error}", step.title());
                self.notify(Some(step), &message, ProgressPhaseKind::Validate);
                return Err(match error {
                    AppError::LlmError(reason) => {
                        AppError::LlmError(format!("{}：{reason}", step.title()))
                    }
                    other => other,
                });
            }
        };
        self.context.check_cancelled()?;
        self.report(
            Some(step),
            &format!(
                "{}{}，用时 {:.1} 秒，输出 {} 字符；{}",
                step.title(),
                if trace.restored_from_checkpoint {
                    "已复用检查点"
                } else {
                    "生成并校验完成"
                },
                started.elapsed().as_secs_f64(),
                trace.response_text.chars().count(),
                a4n_llm_trace::llm_usage_summary_text(trace.usage.as_ref())
            ),
            1,
            ProgressPhaseKind::Save,
        )
        .await?;
        Ok(output)
    }
}
