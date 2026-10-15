use a4n_api_contract::media_authoring::*;
use a4n_domain::collaboration::ProjectAccessRole;
use a4n_error::AppError;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

use super::{
    GenerateAdaptation, authoring_delete_error, authoring_error, authoring_source, records,
    references::video_reference_status_from_batch,
};
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
            self.database.executor(),
            input.project_id,
            input.chapter_id,
        )
        .await
        .map_err(authoring_error)?;
        let video_reference_keys = records
            .iter()
            .filter_map(|record| record.inputs.video_reference.as_ref())
            .filter_map(|reference| {
                reference
                    .video_project_id
                    .filter(|project_id| *project_id > 0 && reference.script_id > 0)
                    .map(|project_id| (project_id, reference.script_id))
            })
            .collect::<Vec<_>>();
        let mut video_reference_project_ids = video_reference_keys
            .iter()
            .map(|(project_id, _)| *project_id)
            .collect::<Vec<_>>();
        video_reference_project_ids.sort_unstable();
        video_reference_project_ids.dedup();
        let accessible_video_project_ids = self
            .project_ids_with_permission(
                &video_reference_project_ids,
                user_id,
                ProjectAccessRole::Viewer,
            )
            .await?
            .into_iter()
            .collect::<HashSet<_>>();
        let accessible_video_reference_keys = video_reference_keys
            .iter()
            .copied()
            .filter(|(project_id, _)| accessible_video_project_ids.contains(project_id))
            .collect::<Vec<_>>();
        let video_reference_scripts = records::video_reference_scripts_by_ids(
            self.database.executor(),
            &accessible_video_reference_keys,
        )
        .await
        .map_err(AppError::database)?;
        let sources_by_chapter = sources
            .iter()
            .map(|source| (source.source_chapter_id, source))
            .collect::<HashMap<_, _>>();
        let needs_manga_fingerprints = target == AdaptationTarget::Manga
            && records
                .iter()
                .any(|record| record.kind == AuthoringDocumentKind::AdaptationPlan);
        let preprocessing_artifacts = if needs_manga_fingerprints {
            self.manga_pipeline_artifacts_by_chapters(
                input.project_id,
                &sources_by_chapter.keys().copied().collect::<Vec<_>>(),
                a4n_domain::manga::MangaPipelineArtifactStage::Preprocessing,
            )
            .await?
        } else {
            HashMap::new()
        };
        let reference_assets = if needs_manga_fingerprints {
            Some(a4n_manga_core::manga_reference_assets_json(
                &self.manga_character_portrait_refs(input.project_id).await?,
            ))
        } else {
            None
        };
        let mut documents = Vec::new();
        for record in &records {
            let preprocessing_stale = if target == AdaptationTarget::Manga
                && record.kind == AuthoringDocumentKind::AdaptationPlan
            {
                match record.source_chapter_id {
                    Some(chapter_id) => match sources_by_chapter.get(&chapter_id) {
                        Some(source) => match preprocessing_artifacts.get(&chapter_id) {
                            Some(artifact) => {
                                let reference_assets =
                                    reference_assets.as_ref().ok_or_else(|| {
                                        AppError::InvalidState {
                                            current: "manga_reference_assets_missing".into(),
                                            operation: "计算漫画预处理指纹".into(),
                                        }
                                    })?;
                                let input_fingerprint =
                                    a4n_domain::manga::manga_preprocessing_input_fingerprint(
                                        &source.content_sha256,
                                        source.chapter_number,
                                        reference_assets,
                                    );
                                artifact.stale
                                    || artifact.input_fingerprint != input_fingerprint
                                    || record.inputs.preprocessing_fingerprint.as_deref()
                                        != Some(artifact.output_fingerprint.as_str())
                            }
                            None => true,
                        },
                        None => true,
                    },
                    None => true,
                }
            } else {
                false
            };
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
                })
                || preprocessing_stale;
            let video_reference_status = video_reference_status_from_batch(
                record.inputs.video_reference.as_ref(),
                &accessible_video_project_ids,
                &video_reference_scripts,
            );
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
            .authoring_artifact_views(
                input.project_id,
                target,
                snapshot_id,
                input.chapter_id,
                &documents,
            )
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
        let mut inputs = match &existing {
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
        if input.kind == AuthoringDocumentKind::AdaptationPlan && target == AdaptationTarget::Manga
        {
            let source = sources
                .iter()
                .find(|source| Some(source.source_chapter_id) == input.chapter_id)
                .ok_or_else(|| AppError::NotFound("漫画改编原文章节不存在".into()))?;
            inputs.preprocessing_fingerprint = Some(
                self.current_manga_preprocessing_fingerprint(
                    input.project_id,
                    source.source_chapter_id,
                    &source.content_sha256,
                    source.chapter_number,
                )
                .await?,
            );
        }
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
            self.database.executor(),
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
            self.database.executor(),
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
            self.database.executor(),
            input.project_id,
            record.id,
            input.expected_revision,
        )
        .await
        .map_err(authoring_error)
    }

    async fn delete(
        &self,
        user_id: i64,
        input: DeleteAuthoringDocumentInput,
    ) -> Result<(), AppError> {
        self.verify_project_permission(input.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        records::delete_authoring_document(
            self.database.executor(),
            input.project_id,
            input.record_id,
            input.kind,
            input.expected_revision,
            &input.expected_impact_fingerprint,
        )
        .await
        .map_err(authoring_delete_error)?;
        Ok(())
    }

    async fn preview_delete(
        &self,
        user_id: i64,
        input: PreviewDeleteAuthoringDocumentInput,
    ) -> Result<AuthoringDocumentDeletionImpact, AppError> {
        self.verify_project_permission(input.project_id, user_id, ProjectAccessRole::Editor)
            .await?;
        records::preview_delete_authoring_document(
            self.database.executor(),
            input.project_id,
            input.record_id,
            input.kind,
            input.expected_revision,
        )
        .await
        .map_err(authoring_delete_error)
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
        stream_reporter: Option<&MediaAuthoringStreamReporter<'_>>,
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
                stream_reporter,
                cancel_token: None,
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
        let renderer = PromptRendererAdapter(renderer);
        Ok(serde_json::to_value(
            self.preview_adaptation_plan_request(user_id, input, &renderer)
                .await?,
        )?)
    }
}
