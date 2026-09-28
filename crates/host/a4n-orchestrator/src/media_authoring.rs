use std::collections::BTreeSet;

use a4n_db::media_authoring::{self as records, AuthoringDocumentError, SaveAuthoringDocument};
use a4n_domain::{
    chapter::ResolvedChapterSource,
    media_authoring::{
        AdaptationPlanV1, AdaptationTarget, AuthoringDocumentInputs, AuthoringDocumentKind,
        AuthoringDocumentRecord, AuthoringDocumentStatus, AuthoringSourceRef,
    },
    source_spans::{source_span_prompt_values, source_spans_for_content},
};
use a4n_error::AppError;
use a4n_prompt::rendering::PromptRenderer;
use serde_json::json;

use crate::{OrchestratorContext, workflow::WorkflowLlmOnceInput};

const DEFAULT_INSTRUCTION: &str = "按目标媒介独立改编，保留原作关键因果和人物动机";
mod application;
mod artifacts;
mod references;

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
}

impl OrchestratorContext {
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
                self.database.connection(),
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

    pub(crate) async fn preview_initial_adaptation(
        &self,
        source: &ResolvedChapterSource,
        target: AdaptationTarget,
        renderer: &dyn PromptRenderer,
    ) -> Result<a4n_llm_provider_contract::provider::ChatRequest, AppError> {
        let inputs = AuthoringDocumentInputs {
            sources: vec![authoring_source(source)?],
            video_reference: None,
            dependencies: Vec::new(),
        };
        let user = self
            .adaptation_user_prompt(source, target, &inputs, DEFAULT_INSTRUCTION, None)
            .await?;
        let system = renderer
            .render_prompt(
                "chapter_media_adaptation_plan",
                &Default::default(),
                Some(source.requested_project_id),
            )
            .await?;
        Ok(crate::workflow::workflow_json_request(system, user))
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
    ) -> Result<PreparedAdaptation, AppError> {
        let db = self.database.connection();
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
                },
                record_id: None,
                expected_revision: None,
                previous_plan_id: None,
                instruction: DEFAULT_INSTRUCTION,
                workflow_run_id,
                workflow,
                renderer,
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
        let user_prompt = self
            .adaptation_user_prompt(
                source,
                input.target,
                &input.inputs,
                input.instruction,
                input.previous_plan_id,
            )
            .await?;
        let (_, plan) = self
            .run_workflow_llm_once(
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
                    user_prompt,
                },
                |value| {
                    let plan: AdaptationPlanV1 = serde_json::from_value(value)?;
                    if plan.target != input.target {
                        return Err(AppError::InvalidArgument("改编计划的目标媒介不一致".into()));
                    }
                    plan.validate(&span_ids)
                        .map_err(AppError::InvalidArgument)?;
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
            self.database.connection(),
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
    ) -> Result<String, AppError> {
        let previous = match previous_plan_id {
            Some(id) => Some(
                records::authoring_document(
                    self.database.connection(),
                    source.requested_project_id,
                    id,
                )
                .await
                .map_err(authoring_error)?,
            ),
            None => records::current_authoring_document(
                self.database.connection(),
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
        Ok(serde_json::to_string(&json!({
            "target": target,
            "instruction": instruction,
            "source": inputs.sources,
            "chapter": {"number": source.chapter_number, "title": source.title, "content": source.content},
            "source_spans": source_span_prompt_values(&spans),
            "video_reference": inputs.video_reference,
            "previous_plan": previous.map(|record| record.document),
        }))?)
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
    match error {
        AuthoringDocumentError::Database(error) => AppError::Database(error),
        AuthoringDocumentError::InvalidArgument(message) => AppError::InvalidArgument(message),
        AuthoringDocumentError::NotFound(id) => AppError::NotFound(format!("创作文档不存在: {id}")),
        error => AppError::InvalidState {
            current: error.to_string(),
            operation: "更新媒体创作文档".into(),
        },
    }
}
