use a4n_api_contract::media_authoring::*;
use a4n_domain::collaboration::ProjectAccessRole;
use a4n_error::AppError;
use serde_json::{Value, json};

use super::{GenerateAdaptation, authoring_error, authoring_source, records};
use crate::{OrchestratorContext, prompt_records::PromptRendererAdapter};

#[async_trait::async_trait]
impl MediaAuthoringApplicationPort for OrchestratorContext {
    async fn workspace(
        &self,
        user_id: i64,
        input: MediaAuthoringInput,
    ) -> Result<MediaAuthoringWorkspace, AppError> {
        let (target, snapshot_id, sources) = self
            .authoring_sources(
                user_id,
                input.project_id,
                input.chapter_id,
                ProjectAccessRole::Viewer,
            )
            .await?;
        let records = records::list_authoring_documents(
            self.database.connection(),
            input.project_id,
            input.chapter_id,
        )
        .await
        .map_err(authoring_error)?;
        let mut documents = Vec::new();
        for record in &records {
            let stale = record.source_snapshot_id != snapshot_id
                || record.inputs.sources.iter().any(|source| {
                    !sources.iter().any(|current| {
                        current.source_chapter_id == source.source_chapter_id
                            && current.source_snapshot_chapter_id
                                == Some(source.source_snapshot_chapter_id)
                            && current.content_sha256 == source.content_sha256
                    })
                })
                || record.inputs.dependencies.iter().any(|dependency| {
                    !records.iter().any(|parent| {
                        parent.reference() == *dependency
                            && parent.status == AuthoringDocumentStatus::Adopted
                    })
                });
            let video_reference_status = self
                .video_reference_status(user_id, record.inputs.video_reference.as_ref())
                .await?;
            documents.push(AuthoringDocumentView {
                record: record.clone(),
                stale,
                video_reference_status,
            });
        }
        let chapters = sources
            .iter()
            .map(|source| {
                Ok(AuthoringChapter {
                    source: authoring_source(source)?,
                    chapter_number: source.chapter_number,
                    title: source.title.clone(),
                    source_spans: a4n_domain::source_spans::source_spans_for_content(
                        source.source_chapter_id,
                        source.chapter_number,
                        &source.content,
                    )
                    .into_iter()
                    .map(|span| AuthoringSourceSpan {
                        id: span.id,
                        text: span.text,
                    })
                    .collect(),
                })
            })
            .collect::<Result<_, AppError>>()?;
        let artifacts = self
            .authoring_artifact_views(input.project_id, target, snapshot_id, &documents)
            .await?;
        Ok(MediaAuthoringWorkspace {
            target,
            source_snapshot_id: snapshot_id,
            chapters,
            documents,
            artifacts,
        })
    }

    async fn save(
        &self,
        user_id: i64,
        input: SaveAuthoringDocumentInput,
    ) -> Result<AuthoringDocumentRecord, AppError> {
        let (target, snapshot_id, sources) = self
            .authoring_sources(
                user_id,
                input.project_id,
                input.chapter_id,
                ProjectAccessRole::Editor,
            )
            .await?;
        let existing = self
            .authoring_edit_base(input.project_id, input.record_id, input.expected_revision)
            .await?;
        let inputs = match &existing {
            Some(record)
                if record.kind == input.kind
                    && record.source_chapter_id == input.chapter_id
                    && record.source_snapshot_id == snapshot_id =>
            {
                record.inputs.clone()
            }
            Some(_) => {
                return Err(AppError::InvalidArgument(
                    "文档种类、章节或快照与编辑请求不匹配".into(),
                ));
            }
            None => {
                self.authoring_inputs_for_sources(&sources, input.kind)
                    .await?
            }
        };
        self.validate_authoring_payload(
            input.project_id,
            target,
            input.kind,
            &sources,
            &inputs,
            &input.document,
        )
        .await?;
        let draft = existing.filter(|record| record.status == AuthoringDocumentStatus::Draft);
        records::save_authoring_document(
            self.database.connection(),
            records::SaveAuthoringDocument {
                project_id: input.project_id,
                source_snapshot_id: snapshot_id,
                source_chapter_id: input.chapter_id,
                kind: input.kind,
                record_id: draft.as_ref().map(|record| record.id),
                expected_revision: draft.map(|record| record.revision),
                inputs,
                document: input.document,
            },
        )
        .await
        .map_err(authoring_error)
    }

    async fn adopt(
        &self,
        user_id: i64,
        input: AdoptAuthoringDocumentInput,
    ) -> Result<AuthoringDocumentRecord, AppError> {
        self.verify_project_permission(input.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        let record = records::authoring_document(
            self.database.connection(),
            input.project_id,
            input.record_id,
        )
        .await
        .map_err(authoring_error)?;
        let (target, _, sources) = self
            .authoring_sources(
                user_id,
                input.project_id,
                record.source_chapter_id,
                ProjectAccessRole::Editor,
            )
            .await?;
        self.validate_authoring_payload(
            input.project_id,
            target,
            record.kind,
            &sources,
            &record.inputs,
            &record.document,
        )
        .await?;
        records::adopt_authoring_document(
            self.database.connection(),
            input.project_id,
            record.id,
            input.expected_revision,
        )
        .await
        .map_err(authoring_error)
    }

    async fn video_references(
        &self,
        user_id: i64,
        input: MediaAuthoringInput,
    ) -> Result<Vec<VideoScriptReferenceCandidate>, AppError> {
        self.list_video_references(user_id, input).await
    }

    async fn generate(
        &self,
        user_id: i64,
        input: GenerateAdaptationPlanInput,
        renderer: &dyn a4n_api_contract::prompt::PromptRenderPort,
    ) -> Result<AuthoringDocumentRecord, AppError> {
        let (target, source, inputs, existing) =
            self.prepare_adaptation_request(user_id, &input).await?;
        let workflow = crate::workflow::UpsertWorkflowRunInput {
            business_key: format!(
                "media_adaptation_plan:{}:{}",
                input.project_id, input.chapter_id
            ),
            workflow_kind: "media_adaptation_plan".into(),
            project_id: Some(input.project_id),
            root_project_id: Some(source.root_project_id),
            source_snapshot_id: source.source_snapshot_id,
            scope_table: Some("chapter".into()),
            scope_id: Some(input.chapter_id),
            chapter_id: Some(input.chapter_id),
            mode: None,
            input_fingerprint_json: Some(
                json!({"inputs": inputs, "instruction": input.instruction, "base": existing.as_ref().map(AuthoringDocumentRecord::reference)}),
            ),
            status: crate::workflow::WORKFLOW_STATUS_RUNNING.into(),
            current_step_key: Some("adaptation_plan".into()),
            active_task_id: Some(input.task_id.clone()),
            can_resume: true,
            resume_block_reason: None,
            error_message: None,
        };
        let run = if input.resume_from_checkpoint {
            self.begin_or_resume_workflow_run_validated(workflow)
                .await?
        } else {
            self.begin_workflow_run_validated(workflow).await?
        };
        let draft = existing.as_ref().filter(|record| {
            record.status == AuthoringDocumentStatus::Draft
                && Some(record.source_snapshot_id) == source.source_snapshot_id
        });
        let result = self
            .generate_adaptation_plan(GenerateAdaptation {
                source: &source,
                target,
                inputs,
                record_id: draft.map(|record| record.id),
                expected_revision: draft.map(|record| record.revision),
                previous_plan_id: existing.as_ref().map(|record| record.id),
                instruction: &input.instruction,
                workflow_run_id: run.id,
                workflow: "media_adaptation_plan",
                renderer: &PromptRendererAdapter(renderer),
            })
            .await
            .map(|prepared| prepared.record);
        self.finalize_workflow_result(run.id, result, "生成改编计划")
            .await
    }

    async fn preview(
        &self,
        user_id: i64,
        input: GenerateAdaptationPlanInput,
        renderer: &dyn a4n_api_contract::prompt::PromptRenderPort,
    ) -> Result<Value, AppError> {
        let (target, source, inputs, existing) =
            self.prepare_adaptation_request(user_id, &input).await?;
        let user = self
            .adaptation_user_prompt(
                &source,
                target,
                &inputs,
                &input.instruction,
                existing.map(|record| record.id),
            )
            .await?;
        let system = renderer
            .render_prompt(
                "chapter_media_adaptation_plan",
                &Default::default(),
                Some(input.project_id),
            )
            .await?;
        Ok(serde_json::to_value(
            crate::workflow::workflow_json_request(system, user),
        )?)
    }
}
