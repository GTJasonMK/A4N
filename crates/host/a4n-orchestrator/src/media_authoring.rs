use std::collections::BTreeSet;

use a4n_api_contract::media_authoring::{
    GenerateAdaptationPlanInput, MediaAuthoringStreamReporter,
};
use a4n_db::media_authoring::{self as records, AuthoringDocumentError, SaveAuthoringDocument};
use a4n_domain::{
    chapter::ResolvedChapterSource,
    media_authoring::{
        AdaptationPlanV1, AdaptationTarget, AuthoringDocumentInputs, AuthoringDocumentKind,
        AuthoringDocumentRecord, AuthoringDocumentStatus, AuthoringSourceRef,
    },
    source_spans::source_spans_for_content,
};
use a4n_error::AppError;
use a4n_llm_provider_contract::provider::ChatRequest;
use a4n_prompt::rendering::PromptRenderer;
use serde_json::json;
use tokio_util::sync::CancellationToken;

use crate::{OrchestratorContext, source_markers::SourceMarkerMap, workflow::WorkflowLlmOnceInput};

const DEFAULT_INSTRUCTION: &str = "按目标媒介独立改编，保留原作关键因果和人物动机";
mod application;
mod artifacts;
mod ports;
mod references;

#[derive(Clone)]
pub(crate) struct PreparedAdaptation {
    pub record: AuthoringDocumentRecord,
    pub plan: AdaptationPlanV1,
}

pub(crate) struct GenerateAdaptation<'a> {
    pub source: &'a ResolvedChapterSource,
    pub target: AdaptationTarget,
    pub inputs: AuthoringDocumentInputs,
    pub record_id: Option<i64>,
    pub expected_revision: Option<i32>,
    pub previous_plan_id: Option<i64>,
    pub instruction: &'a str,
    pub workflow_run_id: i64,
    pub workflow: &'a str,
    pub renderer: &'a dyn PromptRenderer,
    pub stream_reporter: Option<&'a MediaAuthoringStreamReporter<'a>>,
    pub cancel_token: Option<&'a CancellationToken>,
}

impl OrchestratorContext {
    pub(crate) async fn preview_adaptation_plan_request(
        &self,
        user_id: i64,
        input: GenerateAdaptationPlanInput,
        renderer: &dyn PromptRenderer,
    ) -> Result<ChatRequest, AppError> {
        let (target, source, inputs, existing) =
            self.prepare_adaptation_request(user_id, &input).await?;
        let (user, _) = self
            .adaptation_user_prompt(
                &source,
                target,
                &inputs,
                &input.instruction,
                existing.map(|record| record.id),
            )
            .await?;
        let system = renderer
            .render_prompt_for_call(
                "OrchestratorContext.generate_adaptation_plan",
                "chapter_media_adaptation_plan",
                &Default::default(),
                Some(input.project_id),
            )
            .await?;
        Ok(crate::workflow::workflow_json_request(system, user)
            .with_prompt_assembly_key("OrchestratorContext.generate_adaptation_plan"))
    }

    pub(crate) async fn preview_missing_galgame_adaptation_plan_request(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        renderer: &dyn PromptRenderer,
    ) -> Result<ChatRequest, AppError> {
        self.preview_adaptation_plan_request(
            user_id,
            GenerateAdaptationPlanInput {
                project_id,
                chapter_id,
                task_id: "request-preview".into(),
                instruction: DEFAULT_INSTRUCTION.into(),
                video_reference: None,
                clear_video_reference: false,
                record_id: None,
                expected_revision: None,
                resume_from_checkpoint: false,
            },
            renderer,
        )
        .await
    }

    pub(crate) async fn load_current_adaptation(
        &self,
        source: &ResolvedChapterSource,
        target: AdaptationTarget,
    ) -> Result<Option<PreparedAdaptation>, AppError> {
        for status in [
            AuthoringDocumentStatus::Draft,
            AuthoringDocumentStatus::Adopted,
        ] {
            if let Some(record) = records::current_authoring_document(
                self.database.executor(),
                source.requested_project_id,
                authoring_source(source)?.source_snapshot_id,
                Some(source.source_chapter_id),
                AuthoringDocumentKind::AdaptationPlan,
                status,
            )
            .await
            .map_err(authoring_error)?
            {
                return parse_adaptation(record, source, target).map(Some);
            }
        }
        Ok(None)
    }

    pub(crate) async fn require_adopted_manga_adaptation(
        &self,
        source: &ResolvedChapterSource,
        preprocessing_fingerprint: &str,
    ) -> Result<PreparedAdaptation, AppError> {
        let adaptation = self
            .load_current_adaptation(source, AdaptationTarget::Manga)
            .await?
            .filter(|prepared| prepared.record.status == AuthoringDocumentStatus::Adopted)
            .ok_or_else(|| AppError::InvalidState {
                current: "manga_adaptation_plan_not_adopted".into(),
                operation: "请先保存并采用漫画改编计划".into(),
            })?;
        if adaptation
            .record
            .inputs
            .preprocessing_fingerprint
            .as_deref()
            != Some(preprocessing_fingerprint)
        {
            return Err(AppError::InvalidState {
                current: "manga_adaptation_preprocessing_changed".into(),
                operation: "请先重新生成并采用漫画改编计划".into(),
            });
        }
        adaptation
            .plan
            .validate_manga_page_count()
            .map_err(|error| AppError::InvalidState {
                current: "manga_adaptation_page_count_missing".into(),
                operation: format!("请先在漫画改编计划中设置页数：{error}"),
            })?;
        Ok(adaptation)
    }

    /// One-click execution adopts the saved draft, or creates a plan if none
    /// exists. A mismatched source is an explicit update decision, not a retry.
    pub(crate) async fn ensure_adaptation_plan(
        &self,
        source: &ResolvedChapterSource,
        target: AdaptationTarget,
        workflow_run_id: i64,
        workflow: &str,
        renderer: &dyn PromptRenderer,
        stream_reporter: Option<&MediaAuthoringStreamReporter<'_>>,
        cancel_token: Option<&CancellationToken>,
    ) -> Result<PreparedAdaptation, AppError> {
        let db = self.database.executor();
        if let Some(prepared) = self.load_current_adaptation(source, target).await? {
            if prepared.record.status == AuthoringDocumentStatus::Adopted {
                return Ok(prepared);
            }
            let adopted = records::adopt_authoring_document(
                db,
                source.requested_project_id,
                prepared.record.id,
                prepared.record.revision,
            )
            .await
            .map_err(authoring_error)?;
            return Ok(PreparedAdaptation {
                record: adopted,
                plan: prepared.plan,
            });
        }
        let prepared = self
            .generate_adaptation_plan(GenerateAdaptation {
                source,
                target,
                inputs: AuthoringDocumentInputs {
                    sources: vec![authoring_source(source)?],
                    video_reference: None,
                    dependencies: Vec::new(),
                    preprocessing_fingerprint: if target == AdaptationTarget::Manga {
                        Some(
                            self.current_manga_preprocessing_fingerprint(
                                source.requested_project_id,
                                source.source_chapter_id,
                                &source.content_sha256,
                                source.chapter_number,
                            )
                            .await?,
                        )
                    } else {
                        None
                    },
                },
                record_id: None,
                expected_revision: None,
                previous_plan_id: None,
                instruction: DEFAULT_INSTRUCTION,
                workflow_run_id,
                workflow,
                renderer,
                stream_reporter,
                cancel_token,
            })
            .await?;
        let record = records::adopt_authoring_document(
            db,
            source.requested_project_id,
            prepared.record.id,
            prepared.record.revision,
        )
        .await
        .map_err(authoring_error)?;
        Ok(PreparedAdaptation {
            record,
            plan: prepared.plan,
        })
    }

    pub(crate) async fn generate_adaptation_plan(
        &self,
        input: GenerateAdaptation<'_>,
    ) -> Result<PreparedAdaptation, AppError> {
        let source = input.source;
        if input.target == AdaptationTarget::Manga {
            let current_fingerprint = self
                .current_manga_preprocessing_fingerprint(
                    source.requested_project_id,
                    source.source_chapter_id,
                    &source.content_sha256,
                    source.chapter_number,
                )
                .await?;
            if input.inputs.preprocessing_fingerprint.as_deref()
                != Some(current_fingerprint.as_str())
            {
                return Err(AppError::InvalidState {
                    current: "manga_preprocessing_changed".into(),
                    operation: "请先重新生成漫画改编计划".into(),
                });
            }
        }
        if input.inputs.sources != vec![authoring_source(source)?] {
            return Err(AppError::InvalidArgument(
                "改编请求与目标章节来源不一致".into(),
            ));
        }
        let spans = source_spans_for_content(
            source.source_chapter_id,
            source.chapter_number,
            &source.content,
        );
        let span_ids = spans
            .iter()
            .map(|span| span.id.clone())
            .collect::<BTreeSet<_>>();
        let (user_prompt, source_markers) = self
            .adaptation_user_prompt(
                source,
                input.target,
                &input.inputs,
                input.instruction,
                input.previous_plan_id,
            )
            .await?;
        let (_, plan) = self
            .run_workflow_llm_once_with_stream_reporter(
                WorkflowLlmOnceInput {
                    stage_key: "adaptation_plan",
                    prompt_renderer: input.renderer,
                    project_id: source.requested_project_id,
                    root_project_id: source.root_project_id,
                    workflow_run_id: input.workflow_run_id,
                    workflow: input.workflow,
                    step_key: format!("adaptation_plan:{}", source.source_chapter_id),
                    call_order: 1,
                    prompt_name: "chapter_media_adaptation_plan",
                    prompt_assembly_call_key: "OrchestratorContext.generate_adaptation_plan",
                    user_prompt,
                    cancel_token: input.cancel_token,
                },
                input.stream_reporter,
                |value| {
                    let value = source_markers.decode_output(&value)?;
                    let mut plan: AdaptationPlanV1 = serde_json::from_value(value)?;
                    if plan.target != input.target {
                        return Err(AppError::InvalidArgument("改编计划的目标媒介不一致".into()));
                    }
                    plan.complete_unit_source_span_ids_from_beats();
                    plan.validate(&span_ids)
                        .map_err(AppError::InvalidArgument)?;
                    if input.target == AdaptationTarget::Manga {
                        plan.validate_manga_page_count()
                            .map_err(AppError::InvalidArgument)?;
                    }
                    Ok(plan)
                },
            )
            .await?;
        let run = self
            .workflow_run_status_record_by_id(input.workflow_run_id)
            .await?;
        if run.status == crate::workflow::WORKFLOW_STATUS_CANCELLED {
            return Err(AppError::Cancelled);
        }
        let record = records::save_authoring_document(
            self.database.executor(),
            SaveAuthoringDocument {
                project_id: source.requested_project_id,
                source_snapshot_id: authoring_source(source)?.source_snapshot_id,
                source_chapter_id: Some(source.source_chapter_id),
                kind: AuthoringDocumentKind::AdaptationPlan,
                record_id: input.record_id,
                expected_revision: input.expected_revision,
                inputs: input.inputs,
                document: serde_json::to_value(&plan)?,
            },
        )
        .await
        .map_err(authoring_error)?;
        Ok(PreparedAdaptation { record, plan })
    }

    async fn adaptation_user_prompt(
        &self,
        source: &ResolvedChapterSource,
        target: AdaptationTarget,
        inputs: &AuthoringDocumentInputs,
        instruction: &str,
        previous_plan_id: Option<i64>,
    ) -> Result<(String, SourceMarkerMap), AppError> {
        let previous = match previous_plan_id {
            Some(id) => Some(
                records::authoring_document(
                    self.database.executor(),
                    source.requested_project_id,
                    id,
                )
                .await
                .map_err(authoring_error)?,
            ),
            None => records::current_authoring_document(
                self.database.executor(),
                source.requested_project_id,
                authoring_source(source)?.source_snapshot_id,
                Some(source.source_chapter_id),
                AuthoringDocumentKind::AdaptationPlan,
                AuthoringDocumentStatus::Adopted,
            )
            .await
            .map_err(authoring_error)?,
        };
        if previous.as_ref().is_some_and(|record| {
            record.kind != AuthoringDocumentKind::AdaptationPlan
                || record.source_chapter_id != Some(source.source_chapter_id)
        }) {
            return Err(AppError::InvalidArgument(
                "更新改编计划的原版本与目标章节不匹配".into(),
            ));
        }
        let spans = source_spans_for_content(
            source.source_chapter_id,
            source.chapter_number,
            &source.content,
        );
        let manga_preprocessing = if target == AdaptationTarget::Manga {
            Some(
                self.manga_pipeline_artifact(
                    source.requested_project_id,
                    source.source_chapter_id,
                    a4n_domain::manga::MangaPipelineArtifactStage::Preprocessing,
                    None,
                )
                .await?
                .ok_or_else(|| AppError::InvalidState {
                    current: "manga_preprocessing_missing".into(),
                    operation: "请先完成漫画预处理".into(),
                })?
                .payload,
            )
        } else {
            None
        };
        let mut payload = json!({
            "target": target,
            "instruction": instruction,
            "source": inputs.sources,
            "chapter": {"number": source.chapter_number, "title": source.title, "content": source.content},
            "video_reference": inputs.video_reference,
            "manga_preprocessing": manga_preprocessing,
            "previous_plan": previous.map(|record| record.document),
        });
        let source_markers =
            SourceMarkerMap::from_spans_and_values(&spans, std::iter::once(&payload))?;
        payload["chapter"]["content"] =
            serde_json::Value::String(source_markers.mark_text(&source.content, &spans)?);
        let payload = source_markers.encode_input(&payload)?;
        Ok((serde_json::to_string(&payload)?, source_markers))
    }
}

pub(crate) fn authoring_source(
    source: &ResolvedChapterSource,
) -> Result<AuthoringSourceRef, AppError> {
    let (Some(snapshot_id), Some(chapter_id)) =
        (source.source_snapshot_id, source.source_snapshot_chapter_id)
    else {
        return Err(AppError::InvalidArgument(
            "媒体改编必须使用派生项目的原文快照".into(),
        ));
    };
    Ok(AuthoringSourceRef {
        root_project_id: source.root_project_id,
        source_chapter_id: source.source_chapter_id,
        source_snapshot_id: snapshot_id,
        source_snapshot_chapter_id: chapter_id,
        content_sha256: source.content_sha256.clone(),
    })
}

pub(crate) fn parse_adaptation(
    record: AuthoringDocumentRecord,
    source: &ResolvedChapterSource,
    target: AdaptationTarget,
) -> Result<PreparedAdaptation, AppError> {
    if record.project_id != source.requested_project_id
        || Some(record.source_snapshot_id) != source.source_snapshot_id
        || record.source_chapter_id != Some(source.source_chapter_id)
        || record.kind != AuthoringDocumentKind::AdaptationPlan
        || record.inputs.sources != vec![authoring_source(source)?]
    {
        return Err(AppError::InvalidState {
            current: "adaptation_source_changed".into(),
            operation: "来源已变化，请先更新并采用本项目的改编计划".into(),
        });
    }
    let plan: AdaptationPlanV1 = serde_json::from_value(record.document.clone())?;
    if plan.target != target {
        return Err(AppError::InvalidArgument("改编计划目标媒介不一致".into()));
    }
    let spans = source_spans_for_content(
        source.source_chapter_id,
        source.chapter_number,
        &source.content,
    );
    plan.validate(&spans.into_iter().map(|span| span.id).collect())
        .map_err(AppError::InvalidArgument)?;
    Ok(PreparedAdaptation { record, plan })
}

pub(crate) fn authoring_error(error: AuthoringDocumentError) -> AppError {
    authoring_error_with_operation(error, "更新媒体创作文档")
}

pub(crate) fn authoring_delete_error(error: AuthoringDocumentError) -> AppError {
    authoring_error_with_operation(error, "删除媒体创作文档")
}

fn authoring_error_with_operation(error: AuthoringDocumentError, operation: &str) -> AppError {
    match error {
        AuthoringDocumentError::Database(error) => AppError::database(error),
        AuthoringDocumentError::InvalidArgument(message) => AppError::InvalidArgument(message),
        AuthoringDocumentError::NotFound(id) => AppError::NotFound(format!("创作文档不存在: {id}")),
        error => AppError::InvalidState {
            current: error.to_string(),
            operation: operation.into(),
        },
    }
}
