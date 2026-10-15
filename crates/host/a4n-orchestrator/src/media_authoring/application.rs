use a4n_api_contract::media_authoring::*;
use a4n_domain::collaboration::ProjectAccessRole;
use a4n_domain::project::ProjectSourceMode;
use a4n_error::AppError;
use serde_json::Value;

use super::{authoring_error, authoring_source, parse_adaptation, records};
use crate::OrchestratorContext;

impl OrchestratorContext {
    pub(crate) async fn current_manga_preprocessing_input_fingerprint(
        &self,
        project_id: i64,
        source_content_sha256: &str,
        chapter_number: i32,
    ) -> Result<String, AppError> {
        let portraits = self.manga_character_portrait_refs(project_id).await?;
        let reference_assets = a4n_manga_core::manga_reference_assets_json(&portraits);
        Ok(a4n_domain::manga::manga_preprocessing_input_fingerprint(
            source_content_sha256,
            chapter_number,
            &reference_assets,
        ))
    }

    pub(super) async fn current_manga_preprocessing_fingerprint(
        &self,
        project_id: i64,
        chapter_id: i64,
        source_content_sha256: &str,
        chapter_number: i32,
    ) -> Result<String, AppError> {
        let input_fingerprint = self
            .current_manga_preprocessing_input_fingerprint(
                project_id,
                source_content_sha256,
                chapter_number,
            )
            .await?;
        let artifact = self
            .manga_pipeline_artifact(
                project_id,
                chapter_id,
                a4n_domain::manga::MangaPipelineArtifactStage::Preprocessing,
                None,
            )
            .await?
            .ok_or_else(|| AppError::InvalidState {
                current: "manga_preprocessing_missing".into(),
                operation: "请先完成漫画预处理".into(),
            })?;
        if artifact.stale || artifact.input_fingerprint != input_fingerprint {
            return Err(AppError::InvalidState {
                current: "manga_preprocessing_stale".into(),
                operation: "请先重新运行漫画预处理".into(),
            });
        }
        let preprocessing: a4n_domain::manga::MangaPreprocessingV3 =
            serde_json::from_value(artifact.payload)?;
        preprocessing
            .validate()
            .map_err(AppError::InvalidArgument)?;
        Ok(artifact.output_fingerprint)
    }

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
        let project = self
            .project_by_id(project_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
        let source_mode = project
            .source_mode()
            .map_err(|error| AppError::InvalidArgument(format!("项目 {project_id}: {error}")))?;
        if source_mode == ProjectSourceMode::Standalone {
            let chapter_id = chapter_id.ok_or_else(|| {
                AppError::InvalidArgument("独立漫画改编必须先选择文本来源".into())
            })?;
            let source = a4n_db::project::resolve_chapter_source(
                self.database.executor(),
                project_id,
                chapter_id,
            )
            .await
            .map_err(crate::shooting_script::errors::resolve_source_error)?;
            let snapshot_id = source
                .source_snapshot_id
                .ok_or_else(|| AppError::InvalidState {
                    current: "standalone_comic_snapshot_missing".into(),
                    operation: "读取独立漫画文本来源".into(),
                })?;
            return Ok((AdaptationTarget::Manga, snapshot_id, vec![source]));
        }
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
        let sources = match chapter_id {
            Some(id) if id > 0 => a4n_db::project::resolve_snapshot_sources(
                self.database.executor(),
                project_id,
                family.root_project.id,
                snapshot_id,
                &[id],
            )
            .await
            .map_err(crate::shooting_script::errors::resolve_source_error)?,
            Some(_) => return Err(AppError::InvalidArgument("章节 ID 必须大于 0".into())),
            None => a4n_db::project::resolve_snapshot_sources_for_scope(
                self.database.executor(),
                project_id,
                family.root_project.id,
                snapshot_id,
            )
            .await
            .map_err(crate::shooting_script::errors::resolve_source_error)?,
        };
        Ok((target, snapshot_id, sources))
    }

    pub(crate) async fn authoring_inputs_for_sources(
        &self,
        sources: &[a4n_domain::chapter::ResolvedChapterSource],
        kind: AuthoringDocumentKind,
    ) -> Result<AuthoringDocumentInputs, AppError> {
        let mut dependencies = Vec::new();
        if kind != AuthoringDocumentKind::AdaptationPlan {
            for source_batch in sources.chunks(records::AUTHORING_READ_BATCH_SIZE) {
                let Some(first_source) = source_batch.first() else {
                    continue;
                };
                let first_reference = authoring_source(first_source)?;
                let source_chapter_ids = source_batch
                    .iter()
                    .map(|source| source.source_chapter_id)
                    .collect::<Vec<_>>();
                let rows = records::adopted_authoring_document_rows_by_source_chapters(
                    self.database.executor(),
                    first_source.requested_project_id,
                    first_reference.source_snapshot_id,
                    AuthoringDocumentKind::AdaptationPlan,
                    &source_chapter_ids,
                )
                .await
                .map_err(authoring_error)?;
                for (source, row) in source_batch.iter().zip(rows) {
                    authoring_source(source)?;
                    let record = row
                        .map_err(authoring_error)?
                        .map(records::UnparsedAuthoringDocument::parse)
                        .transpose()
                        .map_err(authoring_error)?
                        .ok_or_else(|| AppError::InvalidState {
                            current: "adaptation_not_adopted".into(),
                            operation: format!("请先采用第 {} 章的改编计划", source.chapter_number),
                        })?;
                    dependencies.push(record.reference());
                }
            }
        }
        Ok(AuthoringDocumentInputs {
            sources: sources
                .iter()
                .map(authoring_source)
                .collect::<Result<_, _>>()?,
            video_reference: None,
            dependencies,
            preprocessing_fingerprint: None,
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
                if target == AdaptationTarget::Manga {
                    let current_fingerprint = self
                        .current_manga_preprocessing_fingerprint(
                            project_id,
                            source.source_chapter_id,
                            &source.content_sha256,
                            source.chapter_number,
                        )
                        .await?;
                    if inputs.preprocessing_fingerprint.as_deref()
                        != Some(current_fingerprint.as_str())
                    {
                        return Err(AppError::InvalidState {
                            current: "manga_preprocessing_changed".into(),
                            operation: "请先重新生成漫画改编计划".into(),
                        });
                    }
                }
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
                if target == AdaptationTarget::Manga {
                    plan.validate_manga_page_count()
                        .map_err(AppError::InvalidArgument)?;
                }
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
                    self.database.executor(),
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
                        self.database.executor(),
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

    pub(super) async fn prepare_adaptation_request(
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
                    self.database.executor(),
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
        };
        Ok((target, source, inputs, existing))
    }

    pub(super) async fn authoring_edit_base(
        &self,
        project_id: i64,
        record_id: Option<i64>,
        expected_revision: Option<i32>,
    ) -> Result<Option<AuthoringDocumentRecord>, AppError> {
        match (record_id, expected_revision) {
            (None, None) => Ok(None),
            (Some(id), Some(revision)) if revision > 0 => {
                let record = records::authoring_document(self.database.executor(), project_id, id)
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
