use a4n_api_contract::manga::{
    DeleteMangaPipelineArtifactInput, MangaPipelineArtifactApplicationPort,
    MangaPipelineArtifactDeletionItem, MangaPipelineArtifactDeletionPreview,
    PreviewMangaPipelineArtifactDeletionInput, SaveMangaPipelineArtifactInput,
};
use a4n_db::manga::{MangaPipelineArtifactProjectionReset, MangaPipelineArtifactProjectionUpdate};
use a4n_domain::{
    collaboration::ProjectAccessRole,
    manga::{
        MangaPageAdaptationV1, MangaPageContentV1, MangaPageDrawingPromptV2,
        MangaPageGenerationLayoutV4, MangaPageGenerationSchemaVersion, MangaPagePipelineStage,
        MangaPagePipelineStateV1, MangaPagePipelineStatus, MangaPagePromptRecord,
        MangaPagePromptSourceSummary, MangaPageReferenceDiscoveryV1, MangaPageSlotsV1, MangaPageV4,
        MangaPipelineArtifactDraft, MangaPipelineArtifactRecord, MangaPipelineArtifactStage,
        MangaPreprocessingV3, MangaReferenceBindingSummary,
    },
    source_spans::source_spans_for_content,
    source_text_hash,
};
use a4n_error::AppError;
use serde_json::{Value, json};

use super::output_sync::MangaPagePromptDraft;
use super::pipeline::json::{
    parse_manga_page_adaptation, parse_manga_page_slots, parse_manga_page_storyboard,
    validate_manga_page_slots,
};
use super::reference_assets::validate_manga_page_reference_discovery_candidates;
use super::{
    OrchestratorContext, ensure_manga_source_can_write, layout_rules::manga_page_pipeline_layout,
};

impl OrchestratorContext {
    pub(crate) async fn save_manga_pipeline_artifact_validated(
        &self,
        user_id: i64,
        input: SaveMangaPipelineArtifactInput,
    ) -> Result<MangaPipelineArtifactRecord, AppError> {
        self.verify_project_or_derived_family_access_for_role_validated(
            user_id,
            input.project_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let source = self
            .chapter_manga_source_by_id(input.chapter_id, input.project_id, false)
            .await?;
        ensure_manga_source_can_write(&source, "编辑漫画阶段产物")?;
        let page_number = validated_artifact_page_number(input.stage, input.page_number)?;
        let page_slots_context = if input.stage == MangaPipelineArtifactStage::PageSlots {
            Some(
                self.current_manga_adaptation_and_source_spans(input.project_id, input.chapter_id)
                    .await?,
            )
        } else {
            None
        };
        let page_prompt = if input.stage.page_scoped() {
            Some(
                self.page_prompt_for_stage(
                    input.project_id,
                    input.chapter_id,
                    page_number.expect("page-scoped stage validated"),
                )
                .await?,
            )
        } else {
            None
        };
        if let Some(prompt) = page_prompt.as_ref() {
            manga_page_pipeline_layout(prompt).map_err(AppError::InvalidArgument)?;
        }
        validate_payload(input.stage, page_number, &input.payload)?;
        match input.stage {
            MangaPipelineArtifactStage::PageSlots => {
                let (adaptation, source_spans) = page_slots_context
                    .as_ref()
                    .expect("page_slots context is prepared for this stage");
                parse_manga_page_slots(
                    &input.payload,
                    adaptation,
                    source_spans,
                    "manga_page_slots",
                )?;
            }
            MangaPipelineArtifactStage::PageContent => {
                self.validate_page_content_payload(
                    input.project_id,
                    input.chapter_id,
                    page_number.expect("page-scoped stage validated"),
                    &input.payload,
                )
                .await?;
            }
            MangaPipelineArtifactStage::PageAdaptation => {
                let page_number = page_number.expect("page-scoped stage validated");
                let content: MangaPageContentV1 = serde_json::from_value(
                    self.required_page_stage_payload(
                        input.project_id,
                        input.chapter_id,
                        MangaPipelineArtifactStage::PageContent,
                        page_number,
                    )
                    .await?,
                )?;
                parse_manga_page_adaptation(
                    &input.payload,
                    &content,
                    "manga_page_adaptation_plan",
                )?;
            }
            MangaPipelineArtifactStage::Storyboard => {
                let page_number = page_number.expect("page-scoped stage validated");
                let content: MangaPageContentV1 = serde_json::from_value(
                    self.required_page_stage_payload(
                        input.project_id,
                        input.chapter_id,
                        MangaPipelineArtifactStage::PageContent,
                        page_number,
                    )
                    .await?,
                )?;
                let adaptation = parse_manga_page_adaptation(
                    &self
                        .required_page_stage_payload(
                            input.project_id,
                            input.chapter_id,
                            MangaPipelineArtifactStage::PageAdaptation,
                            page_number,
                        )
                        .await?,
                    &content,
                    "manga_page_adaptation_plan",
                )?;
                parse_manga_page_storyboard(
                    &input.payload,
                    page_number,
                    &content,
                    &adaptation,
                    "manga_page_storyboard",
                )?;
            }
            MangaPipelineArtifactStage::ReferenceDiscovery => {
                let page_number = page_number.expect("page-scoped stage validated");
                self.validate_page_artifact_chain_to_storyboard(
                    input.project_id,
                    input.chapter_id,
                    page_number,
                )
                .await?;
                let prompt = page_prompt.as_ref().ok_or_else(|| AppError::InvalidState {
                    current: "manga_reference_discovery_page_missing".into(),
                    operation: "读取参考图发现页面".into(),
                })?;
                self.validate_reference_discovery_payload(prompt, &input.payload)
                    .await?;
            }
            MangaPipelineArtifactStage::DrawingPrompt => {
                let page_number = page_number.expect("page-scoped stage validated");
                let prompt = page_prompt.as_ref().ok_or_else(|| AppError::InvalidState {
                    current: "manga_drawing_prompt_page_missing".into(),
                    operation: "读取绘图提示词页面".into(),
                })?;
                self.validate_page_artifact_chain_to_storyboard(
                    input.project_id,
                    input.chapter_id,
                    page_number,
                )
                .await?;
                let discovery = self
                    .required_page_stage_payload(
                        input.project_id,
                        input.chapter_id,
                        MangaPipelineArtifactStage::ReferenceDiscovery,
                        page_number,
                    )
                    .await?;
                self.validate_reference_discovery_payload(prompt, &discovery)
                    .await?;
            }
            _ => {}
        }

        let existing = self
            .manga_pipeline_artifact(input.project_id, input.chapter_id, input.stage, page_number)
            .await?;
        let page_slots_changed = input.stage == MangaPipelineArtifactStage::PageSlots
            && existing
                .as_ref()
                .is_none_or(|artifact| artifact.stale || artifact.payload != input.payload);
        let source_content_sha256 = source_text_hash(&source.content);
        let input_fingerprint = if input.stage == MangaPipelineArtifactStage::PageSlots {
            let (adaptation, _) = page_slots_context
                .as_ref()
                .expect("page_slots context is prepared for this stage");
            existing
                .as_ref()
                .filter(|artifact| !artifact.stale && artifact.payload == input.payload)
                .map(|artifact| artifact.input_fingerprint.clone())
                .unwrap_or(source_text_hash(&serde_json::to_string(&json!({
                    "contract": "manga_page_slots_manual_v1",
                    "source_content_sha256": source_content_sha256,
                    "adaptation_plan": {
                        "id": adaptation.record.id,
                        "revision": adaptation.record.revision,
                        "sha256": adaptation.record.sha256,
                    },
                    "payload": input.payload,
                }))?))
        } else if input.stage == MangaPipelineArtifactStage::Preprocessing {
            self.current_manga_preprocessing_input_fingerprint(
                input.project_id,
                &source_content_sha256,
                source.chapter_number,
            )
            .await?
        } else {
            existing.as_ref().map_or_else(
                || {
                    source_text_hash(&format!(
                        "manual:{}:{}:{}",
                        source_content_sha256,
                        input.stage.key(),
                        input.page_number.unwrap_or_default()
                    ))
                },
                |artifact| artifact.input_fingerprint.clone(),
            )
        };
        let stale = input.stage != MangaPipelineArtifactStage::Preprocessing
            && input.stage != MangaPipelineArtifactStage::PageSlots
            && existing.as_ref().is_some_and(|artifact| artifact.stale);
        let projection = if let Some(page_prompt) = page_prompt {
            let expected_updated_at = input
                .expected_prompt_updated_at
                .as_deref()
                .map(str::trim)
                .filter(|expected| !expected.is_empty())
                .ok_or_else(|| {
                    AppError::InvalidArgument(
                        "保存页面阶段产物必须提供页面提示词版本，请刷新后重试".into(),
                    )
                })?;
            let mut layout =
                manga_page_pipeline_layout(&page_prompt).map_err(AppError::InvalidArgument)?;
            update_page_projection_layout(
                &mut layout,
                input.stage,
                Some(input.payload.clone()),
                Some(&input_fingerprint),
            )?;
            Some(MangaPipelineArtifactProjectionUpdate {
                page_prompt,
                layout: serde_json::to_value(layout)?,
                expected_updated_at: expected_updated_at.to_string(),
            })
        } else {
            None
        };
        let record = self
            .save_manga_pipeline_artifact_with_projection(
                MangaPipelineArtifactDraft {
                    project_id: input.project_id,
                    chapter_id: input.chapter_id,
                    stage: input.stage,
                    page_number,
                    input_fingerprint: input_fingerprint.clone(),
                    payload: input.payload.clone(),
                    stale,
                },
                projection,
            )
            .await?;
        if let Some((adaptation, _)) = page_slots_context.as_ref() {
            self.sync_page_slots_page_projections(
                input.project_id,
                input.chapter_id,
                adaptation,
                &input.payload,
                &record,
                page_slots_changed,
            )
            .await?;
        }
        Ok(record)
    }

    async fn sync_page_slots_page_projections(
        &self,
        project_id: i64,
        chapter_id: i64,
        adaptation: &crate::media_authoring::PreparedAdaptation,
        payload: &Value,
        slots_artifact: &MangaPipelineArtifactRecord,
        page_slots_changed: bool,
    ) -> Result<(), AppError> {
        let slots: MangaPageSlotsV1 = serde_json::from_value(payload.clone())?;
        let source = a4n_db::project::resolve_chapter_source(
            self.database.executor(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(crate::shooting_script::errors::resolve_source_error)?;
        let source_metadata = MangaPagePromptSourceSummary {
            root_project_id: source.root_project_id,
            source_chapter_id: source.source_chapter_id,
            source_snapshot_id: source.source_snapshot_id,
            source_snapshot_chapter_id: source.source_snapshot_chapter_id,
            source_revision_key: source.source_revision_key,
            content_sha256: source.content_sha256.clone(),
            adaptation_plan_id: adaptation.record.id,
            adaptation_plan_revision: adaptation.record.revision,
            adaptation_plan_sha256: adaptation.record.sha256.clone(),
        };
        let existing_rows = self
            .manga_page_prompts_by_chapter(project_id, chapter_id)
            .await?;
        let mut existing_by_page = existing_rows
            .iter()
            .cloned()
            .map(|row| (row.page_number, row))
            .collect::<std::collections::HashMap<_, _>>();
        let template = existing_rows
            .first()
            .map(manga_page_pipeline_layout)
            .transpose()
            .map_err(AppError::InvalidArgument)?;
        let prompt_fingerprint = template
            .as_ref()
            .map_or_else(|| json!({}), |layout| layout.prompt_fingerprint.clone());
        let page_plan_ref = template
            .as_ref()
            .and_then(|layout| layout.page_plan_ref.clone());
        let mut drafts = Vec::with_capacity(slots.pages.len());
        let mut content_artifacts = Vec::with_capacity(slots.pages.len());

        for slot in &slots.pages {
            let page_number = i32::try_from(slot.page_number)
                .map_err(|_| AppError::InvalidArgument("漫画页码超出范围".into()))?;
            let existing = existing_by_page.remove(&page_number);
            let existing_layout = existing
                .as_ref()
                .map(manga_page_pipeline_layout)
                .transpose()
                .map_err(AppError::InvalidArgument)?;
            let projection_changed = existing_layout.as_ref().is_none_or(|layout| {
                layout.pipeline.page_content.as_ref() != Some(&slot.content)
                    || layout.source != source_metadata
                    || layout.prompt_fingerprint != prompt_fingerprint
                    || layout.page_plan_ref != page_plan_ref
            });
            let content_payload = serde_json::to_value(&slot.content)?;
            let previous_content_artifact = self
                .manga_pipeline_artifact(
                    project_id,
                    chapter_id,
                    MangaPipelineArtifactStage::PageContent,
                    Some(slot.page_number),
                )
                .await?;
            let content_artifact_changed = previous_content_artifact
                .as_ref()
                .is_none_or(|artifact| artifact.stale || artifact.payload != content_payload);
            let content_input_fingerprint = previous_content_artifact
                .as_ref()
                .filter(|artifact| {
                    !page_slots_changed
                        && !projection_changed
                        && !artifact.stale
                        && artifact.payload == content_payload
                })
                .map(|artifact| artifact.input_fingerprint.clone())
                .unwrap_or(source_text_hash(&serde_json::to_string(&json!({
                    "contract": "manga_page_content_manual_v1",
                    "page_slots_output_fingerprint": slots_artifact.output_fingerprint,
                    "source_content_sha256": source.content_sha256,
                    "page_number": slot.page_number,
                    "content": slot.content,
                }))?));
            let mut layout = if let Some(mut layout) = existing_layout {
                layout.source = source_metadata.clone();
                layout.pipeline.source_fingerprint = source.content_sha256.clone();
                layout
            } else {
                let page_id = format!("page-{:04}", slot.page_number);
                MangaPageGenerationLayoutV4 {
                    schema_version: MangaPageGenerationSchemaVersion::V4,
                    page_id: page_id.clone(),
                    page_number: slot.page_number,
                    page_plan_ref: page_plan_ref.clone(),
                    pipeline: MangaPagePipelineStateV1 {
                        schema_version: a4n_domain::manga::MANGA_PAGE_PIPELINE_STATE_V1.into(),
                        page_id,
                        status: MangaPagePipelineStatus::Pending,
                        stage: MangaPagePipelineStage::Adaptation,
                        attempt: 1,
                        source_fingerprint: source.content_sha256.clone(),
                        stage_fingerprints: Default::default(),
                        page_content: None,
                        page_adaptation: None,
                        storyboard: None,
                        reference_discovery: None,
                        drawing_prompt: None,
                        image: None,
                        error: None,
                    },
                    reference_bindings: Vec::new(),
                    prompt_fingerprint: prompt_fingerprint.clone(),
                    source: source_metadata.clone(),
                }
            };
            layout.prompt_fingerprint = prompt_fingerprint.clone();
            layout.page_plan_ref = page_plan_ref.clone();
            layout.pipeline.page_content = Some(slot.content.clone());
            if page_slots_changed || projection_changed || content_artifact_changed {
                layout.pipeline.stage = MangaPagePipelineStage::Adaptation;
                layout.pipeline.status = MangaPagePipelineStatus::Pending;
                layout.pipeline.error = None;
            }
            layout
                .pipeline
                .stage_fingerprints
                .insert("content".into(), content_input_fingerprint.clone());
            let (image_prompt, dialogue_text, characters_in_frame, status) = existing
                .map(|row| {
                    let status = if row.generated_image_id.is_some()
                        || (!page_slots_changed && !projection_changed && !content_artifact_changed)
                    {
                        row.status.clone()
                    } else {
                        "pending".to_string()
                    };
                    (
                        row.image_prompt,
                        row.dialogue_text,
                        row.characters_in_frame,
                        status,
                    )
                })
                .unwrap_or_else(|| (String::new(), None, None, "pending".to_string()));
            drafts.push(MangaPagePromptDraft {
                project_id,
                chapter_id,
                page_number,
                image_prompt,
                dialogue_text,
                characters_in_frame,
                layout: Some(serde_json::to_value(layout)?),
                status,
                generation_traces: Vec::new(),
            });
            content_artifacts.push((slot.page_number, content_payload, content_input_fingerprint));
        }

        self.sync_generated_page_prompts(project_id, chapter_id, drafts)
            .await?;
        for (page_number, payload, input_fingerprint) in content_artifacts {
            self.upsert_manga_pipeline_artifact(MangaPipelineArtifactDraft {
                project_id,
                chapter_id,
                stage: MangaPipelineArtifactStage::PageContent,
                page_number: Some(page_number),
                input_fingerprint,
                payload,
                stale: false,
            })
            .await?;
        }
        Ok(())
    }

    pub(crate) async fn delete_manga_pipeline_artifact_validated(
        &self,
        user_id: i64,
        input: DeleteMangaPipelineArtifactInput,
    ) -> Result<(), AppError> {
        self.verify_project_or_derived_family_access_for_role_validated(
            user_id,
            input.project_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let source = self
            .chapter_manga_source_by_id(input.chapter_id, input.project_id, false)
            .await?;
        ensure_manga_source_can_write(&source, "删除漫画阶段产物")?;
        let page_number = validated_artifact_page_number(input.stage, input.page_number)?;
        if let Some(page_number) = page_number {
            self.page_prompt_for_stage(input.project_id, input.chapter_id, page_number)
                .await?;
        }
        let mut projection_resets = Vec::new();
        for prompt in self
            .manga_page_prompts_by_chapter(input.project_id, input.chapter_id)
            .await?
        {
            let page_number = u32::try_from(prompt.page_number)
                .map_err(|_| AppError::InvalidArgument("漫画页面页码无效".into()))?;
            if input.stage.page_scoped() && input.page_number != Some(page_number) {
                continue;
            }
            let clear_image = input.stage == MangaPipelineArtifactStage::Image
                || MangaPipelineArtifactStage::Image.depends_on(input.stage);
            let mut layout =
                manga_page_pipeline_layout(&prompt).map_err(AppError::InvalidArgument)?;
            layout.clear_deleted_stage(input.stage);
            layout
                .validate(page_number)
                .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
            let image_prompt = layout
                .pipeline
                .drawing_prompt
                .as_ref()
                .map(|prompt| prompt.image_prompt.clone())
                .unwrap_or_default();
            let expected_layout = prompt
                .layout
                .ok_or_else(|| AppError::InvalidArgument("漫画页面缺少 layout".into()))?;
            projection_resets.push(MangaPipelineArtifactProjectionReset {
                page_prompt_id: prompt.id,
                page_number,
                expected_layout,
                expected_generated_image_id: prompt.generated_image_id,
                layout: serde_json::to_value(layout)?,
                image_prompt,
                clear_generated_image: clear_image,
            });
        }
        self.delete_manga_pipeline_artifact(
            input.project_id,
            input.chapter_id,
            input.stage,
            page_number,
            &input.expected_impact_fingerprint,
            &projection_resets,
        )
        .await?;
        Ok(())
    }

    pub(crate) async fn preview_manga_pipeline_artifact_deletion_validated(
        &self,
        user_id: i64,
        input: PreviewMangaPipelineArtifactDeletionInput,
    ) -> Result<MangaPipelineArtifactDeletionPreview, AppError> {
        self.verify_project_or_derived_family_access_for_role_validated(
            user_id,
            input.project_id,
            ProjectAccessRole::Editor,
        )
        .await?;
        let source = self
            .chapter_manga_source_by_id(input.chapter_id, input.project_id, false)
            .await?;
        ensure_manga_source_can_write(&source, "预览漫画阶段产物删除")?;
        let page_number = validated_artifact_page_number(input.stage, input.page_number)?;
        if let Some(page_number) = page_number {
            self.page_prompt_for_stage(input.project_id, input.chapter_id, page_number)
                .await?;
        }
        let plan = self
            .manga_pipeline_artifact_deletion_plan(
                input.project_id,
                input.chapter_id,
                input.stage,
                page_number,
            )
            .await?;
        let artifacts = plan.artifacts;
        let to_item = |artifact: &MangaPipelineArtifactRecord| MangaPipelineArtifactDeletionItem {
            artifact_id: Some(artifact.id),
            stage: artifact.stage,
            page_number: artifact.page_number,
        };
        let target = artifacts
            .iter()
            .find(|artifact| artifact.stage == input.stage && artifact.page_number == page_number)
            .map(|artifact| artifact.id);
        Ok(MangaPipelineArtifactDeletionPreview {
            fingerprint: plan.fingerprint,
            target: MangaPipelineArtifactDeletionItem {
                artifact_id: target,
                stage: input.stage,
                page_number,
            },
            downstream: artifacts
                .iter()
                .filter(|artifact| Some(artifact.id) != target)
                .map(to_item)
                .collect(),
        })
    }

    async fn page_prompt_for_stage(
        &self,
        project_id: i64,
        chapter_id: i64,
        page_number: u32,
    ) -> Result<MangaPagePromptRecord, AppError> {
        let page_number = i32::try_from(page_number)
            .map_err(|_| AppError::InvalidArgument("漫画页码超出范围".into()))?;
        self.manga_page_prompts_by_chapter(project_id, chapter_id)
            .await?
            .into_iter()
            .find(|prompt| prompt.page_number == page_number)
            .ok_or_else(|| AppError::NotFound("漫画页面不存在".into()))
    }

    async fn validate_reference_discovery_payload(
        &self,
        page_prompt: &MangaPagePromptRecord,
        payload: &Value,
    ) -> Result<(), AppError> {
        let discovery: MangaPageReferenceDiscoveryV1 = serde_json::from_value(payload.clone())?;
        let layout = manga_page_pipeline_layout(page_prompt).map_err(AppError::InvalidArgument)?;
        let asset_project_ids = self
            .manga_reference_asset_project_ids(page_prompt.project_id)
            .await?;
        let available_images = self
            .video_reference_candidate_images(&asset_project_ids, page_prompt.chapter_id)
            .await?;
        let (_, candidates) = super::reference_assets::manga_page_reference_discovery_candidates(
            &available_images,
            page_prompt.generated_image_id,
            &layout.reference_bindings,
        );
        validate_manga_page_reference_discovery_candidates(&discovery, &candidates)
    }

    pub(in crate::manga) async fn current_manga_adaptation_and_source_spans(
        &self,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<
        (
            crate::media_authoring::PreparedAdaptation,
            Vec<a4n_domain::source_spans::SourceSpan>,
        ),
        AppError,
    > {
        let source = a4n_db::project::resolve_chapter_source(
            self.database.executor(),
            project_id,
            chapter_id,
        )
        .await
        .map_err(crate::shooting_script::errors::resolve_source_error)?;
        let preprocessing = self
            .manga_pipeline_artifact(
                project_id,
                chapter_id,
                MangaPipelineArtifactStage::Preprocessing,
                None,
            )
            .await?
            .ok_or_else(|| AppError::InvalidState {
                current: "manga_preprocessing_missing".into(),
                operation: "请先完成漫画预处理".into(),
            })?;
        let preprocessing_input_fingerprint = self
            .current_manga_preprocessing_input_fingerprint(
                project_id,
                &source_text_hash(&source.content),
                source.chapter_number,
            )
            .await?;
        if preprocessing.stale || preprocessing.input_fingerprint != preprocessing_input_fingerprint
        {
            return Err(AppError::InvalidState {
                current: "manga_preprocessing_stale".into(),
                operation: "请先重新运行漫画预处理".into(),
            });
        }
        let adaptation = self
            .require_adopted_manga_adaptation(&source, &preprocessing.output_fingerprint)
            .await?;
        let source_spans = source_spans_for_content(
            source.source_chapter_id,
            source.chapter_number,
            &source.content,
        );
        Ok((adaptation, source_spans))
    }

    async fn validate_page_content_payload(
        &self,
        project_id: i64,
        chapter_id: i64,
        page_number: u32,
        payload: &Value,
    ) -> Result<(), AppError> {
        let (adaptation, source_spans) = self
            .current_manga_adaptation_and_source_spans(project_id, chapter_id)
            .await?;
        let page_slots = self
            .manga_pipeline_artifact(
                project_id,
                chapter_id,
                MangaPipelineArtifactStage::PageSlots,
                None,
            )
            .await?
            .ok_or_else(|| AppError::InvalidState {
                current: "manga_page_slots_missing".into(),
                operation: "请先完成分页".into(),
            })?;
        if page_slots.stale {
            return Err(AppError::InvalidState {
                current: "manga_page_slots_stale".into(),
                operation: "请先重新运行分页".into(),
            });
        }
        let mut slots = parse_manga_page_slots(
            &page_slots.payload,
            &adaptation,
            &source_spans,
            "manga_page_slots",
        )?;
        let artifacts = self
            .manga_pipeline_artifacts_by_chapter(project_id, chapter_id)
            .await?;
        let mut target_page_found = false;
        for slot in &mut slots.pages {
            if slot.page_number == page_number {
                target_page_found = true;
                slot.content = serde_json::from_value(payload.clone())?;
                continue;
            }
            if let Some(artifact) = artifacts.iter().find(|artifact| {
                artifact.stage == MangaPipelineArtifactStage::PageContent
                    && artifact.page_number == Some(slot.page_number)
                    && !artifact.stale
            }) {
                slot.content = serde_json::from_value(artifact.payload.clone())?;
            }
        }
        if !target_page_found {
            return Err(AppError::InvalidArgument(
                "页面内容不属于当前分页计划".into(),
            ));
        }
        validate_manga_page_slots(&slots, &adaptation, &source_spans, "manga_page_slots")?;
        Ok(())
    }

    async fn required_page_stage_payload(
        &self,
        project_id: i64,
        chapter_id: i64,
        stage: MangaPipelineArtifactStage,
        page_number: u32,
    ) -> Result<Value, AppError> {
        let artifact = self
            .manga_pipeline_artifact(project_id, chapter_id, stage, Some(page_number))
            .await?
            .ok_or_else(|| AppError::InvalidState {
                current: format!("manga_stage_artifact_missing:{}:{page_number}", stage.key()),
                operation: format!("保存当前阶段前需要先完成 {}", stage.key()),
            })?;
        if artifact.stale {
            return Err(AppError::InvalidState {
                current: format!("manga_stage_artifact_stale:{}:{page_number}", stage.key()),
                operation: format!("请先重跑过期的前置阶段 {}", stage.key()),
            });
        }
        Ok(artifact.payload)
    }

    async fn validate_page_artifact_chain_to_storyboard(
        &self,
        project_id: i64,
        chapter_id: i64,
        page_number: u32,
    ) -> Result<(), AppError> {
        let content: MangaPageContentV1 = serde_json::from_value(
            self.required_page_stage_payload(
                project_id,
                chapter_id,
                MangaPipelineArtifactStage::PageContent,
                page_number,
            )
            .await?,
        )?;
        let adaptation = parse_manga_page_adaptation(
            &self
                .required_page_stage_payload(
                    project_id,
                    chapter_id,
                    MangaPipelineArtifactStage::PageAdaptation,
                    page_number,
                )
                .await?,
            &content,
            "manga_page_adaptation_plan",
        )?;
        parse_manga_page_storyboard(
            &self
                .required_page_stage_payload(
                    project_id,
                    chapter_id,
                    MangaPipelineArtifactStage::Storyboard,
                    page_number,
                )
                .await?,
            page_number,
            &content,
            &adaptation,
            "manga_page_storyboard",
        )?;
        Ok(())
    }
}

fn update_page_projection_layout(
    layout: &mut MangaPageGenerationLayoutV4,
    stage: MangaPipelineArtifactStage,
    payload: Option<Value>,
    input_fingerprint: Option<&str>,
) -> Result<(), AppError> {
    match stage {
        MangaPipelineArtifactStage::PageContent => {
            layout.pipeline.page_content = payload
                .map(serde_json::from_value::<MangaPageContentV1>)
                .transpose()?;
        }
        MangaPipelineArtifactStage::PageAdaptation => {
            layout.pipeline.page_adaptation = payload
                .map(serde_json::from_value::<MangaPageAdaptationV1>)
                .transpose()?;
        }
        MangaPipelineArtifactStage::Storyboard => {
            layout.pipeline.storyboard = payload
                .map(serde_json::from_value::<MangaPageV4>)
                .transpose()?;
        }
        MangaPipelineArtifactStage::ReferenceDiscovery => {
            layout.pipeline.reference_discovery = payload
                .map(serde_json::from_value::<MangaPageReferenceDiscoveryV1>)
                .transpose()?;
        }
        MangaPipelineArtifactStage::ReferenceBindings => {
            layout.reference_bindings = payload
                .map(serde_json::from_value::<Vec<MangaReferenceBindingSummary>>)
                .transpose()?
                .unwrap_or_default();
        }
        MangaPipelineArtifactStage::DrawingPrompt => {
            layout.pipeline.drawing_prompt = payload
                .map(serde_json::from_value::<MangaPageDrawingPromptV2>)
                .transpose()?;
        }
        MangaPipelineArtifactStage::Image => {
            return Err(AppError::InvalidArgument(
                "成图阶段只能通过独立成图操作更新".into(),
            ));
        }
        _ => return Err(AppError::InvalidArgument("当前阶段没有页面布局投影".into())),
    }
    match input_fingerprint {
        Some(fingerprint) => {
            layout
                .pipeline
                .stage_fingerprints
                .insert(stage.fingerprint_key().into(), fingerprint.to_string());
        }
        None => {
            layout
                .pipeline
                .stage_fingerprints
                .remove(stage.fingerprint_key());
        }
    }
    Ok(())
}

fn validate_payload(
    stage: MangaPipelineArtifactStage,
    page_number: Option<u32>,
    payload: &Value,
) -> Result<(), AppError> {
    let page = page_number.unwrap_or_default();
    match stage {
        MangaPipelineArtifactStage::Preprocessing => {
            let preprocessing: MangaPreprocessingV3 = serde_json::from_value(payload.clone())?;
            preprocessing.validate().map_err(AppError::InvalidArgument)
        }
        MangaPipelineArtifactStage::PageSlots => {
            let slots: MangaPageSlotsV1 = serde_json::from_value(payload.clone())?;
            slots.validate().map_err(AppError::InvalidArgument)
        }
        MangaPipelineArtifactStage::PageContent => {
            let content: MangaPageContentV1 = serde_json::from_value(payload.clone())?;
            content.validate().map_err(AppError::InvalidArgument)
        }
        MangaPipelineArtifactStage::PageAdaptation => {
            let adaptation: MangaPageAdaptationV1 = serde_json::from_value(payload.clone())?;
            adaptation
                .validate_artifact()
                .map_err(AppError::InvalidArgument)
        }
        MangaPipelineArtifactStage::Storyboard => {
            let storyboard: MangaPageV4 = serde_json::from_value(payload.clone())?;
            if storyboard.page_number != page {
                return Err(AppError::InvalidArgument(format!("分镜页码必须是 {page}")));
            }
            storyboard
                .validate()
                .map_err(|error| AppError::InvalidArgument(error.to_string()))
        }
        MangaPipelineArtifactStage::ReferenceDiscovery => {
            let discovery: MangaPageReferenceDiscoveryV1 = serde_json::from_value(payload.clone())?;
            discovery.validate().map_err(AppError::InvalidArgument)
        }
        MangaPipelineArtifactStage::DrawingPrompt => {
            let prompt: MangaPageDrawingPromptV2 = serde_json::from_value(payload.clone())?;
            if prompt.page_number != page || prompt.image_prompt.trim().is_empty() {
                return Err(AppError::InvalidArgument(
                    "绘图提示词页码无效或内容为空".into(),
                ));
            }
            Ok(())
        }
        MangaPipelineArtifactStage::ReferenceBindings | MangaPipelineArtifactStage::Image => Err(
            AppError::InvalidArgument("该阶段需通过专用操作修改产物".into()),
        ),
    }
}

fn validated_artifact_page_number(
    stage: MangaPipelineArtifactStage,
    page_number: Option<u32>,
) -> Result<Option<u32>, AppError> {
    match (stage.page_scoped(), page_number) {
        (true, Some(page)) if page > 0 => Ok(Some(page)),
        (true, _) => Err(AppError::InvalidArgument(
            "页级漫画阶段必须指定正整数页码".into(),
        )),
        (false, None) => Ok(None),
        (false, Some(_)) => Err(AppError::InvalidArgument(
            "章节级漫画阶段不能指定页码".into(),
        )),
    }
}

#[async_trait::async_trait]
impl MangaPipelineArtifactApplicationPort for OrchestratorContext {
    async fn list_artifacts(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
    ) -> Result<Vec<MangaPipelineArtifactRecord>, AppError> {
        self.verify_project_or_derived_family_access_for_role_validated(
            user_id,
            project_id,
            ProjectAccessRole::Viewer,
        )
        .await?;
        let source = self
            .chapter_manga_source_by_id(chapter_id, project_id, false)
            .await?;
        let mut artifacts = self
            .manga_pipeline_artifacts_by_chapter(project_id, chapter_id)
            .await?;
        if let Some(artifact) = artifacts
            .iter_mut()
            .find(|artifact| artifact.stage == MangaPipelineArtifactStage::Preprocessing)
        {
            let input_fingerprint = self
                .current_manga_preprocessing_input_fingerprint(
                    project_id,
                    &source_text_hash(&source.content),
                    source.chapter_number,
                )
                .await?;
            artifact.stale |= artifact.input_fingerprint != input_fingerprint;
        }
        Ok(artifacts)
    }

    async fn save_artifact(
        &self,
        user_id: i64,
        input: SaveMangaPipelineArtifactInput,
    ) -> Result<MangaPipelineArtifactRecord, AppError> {
        self.save_manga_pipeline_artifact_validated(user_id, input)
            .await
    }

    async fn preview_artifact_deletion(
        &self,
        user_id: i64,
        input: PreviewMangaPipelineArtifactDeletionInput,
    ) -> Result<MangaPipelineArtifactDeletionPreview, AppError> {
        self.preview_manga_pipeline_artifact_deletion_validated(user_id, input)
            .await
    }

    async fn delete_artifact(
        &self,
        user_id: i64,
        input: DeleteMangaPipelineArtifactInput,
    ) -> Result<(), AppError> {
        self.delete_manga_pipeline_artifact_validated(user_id, input)
            .await
    }
}
