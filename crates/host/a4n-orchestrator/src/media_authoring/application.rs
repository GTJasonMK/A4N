use a4n_api_contract::media_authoring::*;
use a4n_domain::collaboration::ProjectAccessRole;
use a4n_error::AppError;
use serde_json::{Value, json};

use super::{GenerateAdaptation, authoring_error, authoring_source, parse_adaptation, records};
use crate::{OrchestratorContext, prompt_records::PromptRendererAdapter};

impl OrchestratorContext {
    pub(super) async fn authoring_sources(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: Option<i64>,
        role: ProjectAccessRole,
    ) -> Result<
        (
            AdaptationTarget,
            i64,
            Vec<a4n_domain::chapter::ResolvedChapterSource>,
        ),
        AppError,
    > {
        self.verify_project_permission(project_id, user_id, role)
            .await?;
        let family = self.derived_project_family_context(project_id).await?;
        self.verify_project_permission(family.root_project.id, user_id, ProjectAccessRole::Viewer)
            .await?;
        let target = match family.project.project_kind.as_str() {
            "comic" => AdaptationTarget::Manga,
            "galgame" => AdaptationTarget::Galgame,
            _ => {
                return Err(AppError::InvalidArgument(
                    "独立改编计划只用于漫画和 Galgame 派生项目".into(),
                ));
            }
        };
        let snapshot_id = family.source_snapshot.id;
        let ids = match chapter_id {
            Some(id) if id > 0 => vec![id],
            Some(_) => return Err(AppError::InvalidArgument("章节 ID 必须大于 0".into())),
            None => {
                a4n_db::project::source_snapshot_chapters(self.database.connection(), project_id)
                    .await
                    .map_err(crate::project::project_mutation_error_to_app_error)?
                    .into_iter()
                    .map(|chapter| chapter.source_chapter_id)
                    .collect()
            }
        };
        let sources = a4n_db::project::resolve_snapshot_sources(
            self.database.connection(),
            project_id,
            family.root_project.id,
            snapshot_id,
            &ids,
        )
        .await
        .map_err(crate::shooting_script::errors::resolve_source_error)?;
        Ok((target, snapshot_id, sources))
    }

    pub(crate) async fn authoring_inputs_for_sources(
        &self,
        sources: &[a4n_domain::chapter::ResolvedChapterSource],
        kind: AuthoringDocumentKind,
    ) -> Result<AuthoringDocumentInputs, AppError> {
        let mut dependencies = Vec::new();
        for source in sources {
            if kind == AuthoringDocumentKind::AdaptationPlan {
                continue;
            }
            let record = records::current_authoring_document(
                self.database.connection(),
                source.requested_project_id,
                authoring_source(source)?.source_snapshot_id,
                Some(source.source_chapter_id),
                AuthoringDocumentKind::AdaptationPlan,
                AuthoringDocumentStatus::Adopted,
            )
            .await
            .map_err(authoring_error)?
            .ok_or_else(|| AppError::InvalidState {
                current: "adaptation_not_adopted".into(),
                operation: format!("请先采用第 {} 章的改编计划", source.chapter_number),
            })?;
            dependencies.push(record.reference());
        }
        Ok(AuthoringDocumentInputs {
            sources: sources
                .iter()
                .map(authoring_source)
                .collect::<Result<_, _>>()?,
            video_reference: None,
            dependencies,
        })
    }

    pub(crate) async fn validate_authoring_payload(
        &self,
        project_id: i64,
        target: AdaptationTarget,
        kind: AuthoringDocumentKind,
        sources: &[a4n_domain::chapter::ResolvedChapterSource],
        inputs: &AuthoringDocumentInputs,
        value: &Value,
    ) -> Result<(), AppError> {
        if inputs.sources
            != sources
                .iter()
                .map(authoring_source)
                .collect::<Result<Vec<_>, _>>()?
        {
            return Err(AppError::InvalidArgument(
                "文档来源与当前原文快照不一致".into(),
            ));
        }
        match kind {
            AuthoringDocumentKind::AdaptationPlan => {
                let source = sources
                    .first()
                    .filter(|_| sources.len() == 1)
                    .ok_or_else(|| AppError::InvalidArgument("改编计划必须选择一章".into()))?;
                let plan: AdaptationPlanV1 = serde_json::from_value(value.clone())?;
                if plan.target != target {
                    return Err(AppError::InvalidArgument("改编计划目标媒介不一致".into()));
                }
                let spans = a4n_domain::source_spans::source_spans_for_content(
                    source.source_chapter_id,
                    source.chapter_number,
                    &source.content,
                );
                plan.validate(&spans.into_iter().map(|span| span.id).collect())
                    .map_err(AppError::InvalidArgument)?;
            }
            AuthoringDocumentKind::MangaPagePlan => {
                if target != AdaptationTarget::Manga
                    || inputs.dependencies.len() != 1
                    || sources.len() != 1
                {
                    return Err(AppError::InvalidArgument(
                        "漫画分页需要当前章节的已采用改编计划".into(),
                    ));
                }
                let record = records::authoring_document(
                    self.database.connection(),
                    project_id,
                    inputs.dependencies[0].id,
                )
                .await
                .map_err(authoring_error)?;
                let adaptation = parse_adaptation(record, &sources[0], target)?;
                crate::manga::parse_manga_page_plan(value, &adaptation, "保存分页分镜")?;
            }
            AuthoringDocumentKind::GalgameBranchPlan => {
                if target != AdaptationTarget::Galgame || inputs.dependencies.len() != sources.len()
                {
                    return Err(AppError::InvalidArgument(
                        "游戏分支需要范围内全部章节的已采用改编计划".into(),
                    ));
                }
                let mut context = self.galgame_pipeline_prompt_context(project_id).await?;
                context.adaptations.clear();
                for (source, dependency) in sources.iter().zip(&inputs.dependencies) {
                    let record = records::authoring_document(
                        self.database.connection(),
                        project_id,
                        dependency.id,
                    )
                    .await
                    .map_err(authoring_error)?;
                    if record.reference() != *dependency {
                        return Err(AppError::InvalidArgument(
                            "分支依赖的改编计划版本已变化".into(),
                        ));
                    }
                    let adaptation = parse_adaptation(record, source, target)?;
                    context
                        .adaptations
                        .push(crate::galgame::GalgameChapterAdaptation {
                            source_chapter_id: source.source_chapter_id,
                            source_snapshot_chapter_id: authoring_source(source)?
                                .source_snapshot_chapter_id,
                            chapter_number: source.chapter_number,
                            adaptation_plan_id: adaptation.record.id,
                            adaptation_plan_revision: adaptation.record.revision,
                            plan: adaptation.plan,
                        });
                }
                let projection = crate::galgame::galgame_adaptation_projection(&context)?;
                let branches =
                    crate::galgame::branch_design_from_value(value.clone(), "保存分支计划")?;
                crate::galgame::validate_galgame_branch_design(&branches, &projection)?;
            }
        }
        Ok(())
    }

    async fn prepare_adaptation_request(
        &self,
        user_id: i64,
        input: &GenerateAdaptationPlanInput,
    ) -> Result<
        (
            AdaptationTarget,
            a4n_domain::chapter::ResolvedChapterSource,
            AuthoringDocumentInputs,
            Option<AuthoringDocumentRecord>,
        ),
        AppError,
    > {
        if input.instruction.trim().is_empty() || input.task_id.trim().is_empty() {
            return Err(AppError::InvalidArgument(
                "改编要求和任务 ID 不能为空".into(),
            ));
        }
        let (target, _, mut sources) = self
            .authoring_sources(
                user_id,
                input.project_id,
                Some(input.chapter_id),
                ProjectAccessRole::Editor,
            )
            .await?;
        let source = sources
            .pop()
            .ok_or_else(|| AppError::NotFound("改编原文章节不存在".into()))?;
        let existing = self
            .authoring_edit_base(input.project_id, input.record_id, input.expected_revision)
            .await?;
        if let Some(record) = &existing {
            if Some(record.source_snapshot_id) == source.source_snapshot_id {
                parse_adaptation(record.clone(), &source, target)?;
            } else {
                let previous = a4n_db::project::resolve_snapshot_sources(
                    self.database.connection(),
                    input.project_id,
                    source.root_project_id,
                    record.source_snapshot_id,
                    &[source.source_chapter_id],
                )
                .await
                .map_err(crate::shooting_script::errors::resolve_source_error)?;
                let previous = previous
                    .first()
                    .ok_or_else(|| AppError::NotFound("原改编计划的冻结正文不存在".into()))?;
                parse_adaptation(record.clone(), previous, target)?;
            }
        }
        if input.clear_video_reference && input.video_reference.is_some() {
            return Err(AppError::InvalidArgument(
                "不能同时移除和选择视频参考".into(),
            ));
        }
        let reference = match &input.video_reference {
            Some(selection) => Some(
                self.freeze_video_reference(user_id, &source, target, selection)
                    .await?,
            ),
            None if input.clear_video_reference => None,
            None => existing
                .as_ref()
                .and_then(|record| record.inputs.video_reference.clone()),
        };
        let inputs = AuthoringDocumentInputs {
            sources: vec![authoring_source(&source)?],
            video_reference: reference,
            dependencies: Vec::new(),
        };
        Ok((target, source, inputs, existing))
    }

    async fn authoring_edit_base(
        &self,
        project_id: i64,
        record_id: Option<i64>,
        expected_revision: Option<i32>,
    ) -> Result<Option<AuthoringDocumentRecord>, AppError> {
        match (record_id, expected_revision) {
            (None, None) => Ok(None),
            (Some(id), Some(revision)) if revision > 0 => {
                let record =
                    records::authoring_document(self.database.connection(), project_id, id)
                        .await
                        .map_err(authoring_error)?;
                if record.revision != revision {
                    return Err(authoring_error(
                        records::AuthoringDocumentError::RevisionConflict(id),
                    ));
                }
                Ok(Some(record))
            }
            _ => Err(AppError::InvalidArgument(
                "文档 ID 和预期 revision 必须同时提供且大于 0".into(),
            )),
        }
    }
}

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
