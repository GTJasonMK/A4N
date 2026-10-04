use std::collections::HashMap;

use a4n_api_contract::{chapter_visual_plan::*, generation_tasks::GenerationTaskHandle};
use a4n_db::{
    chapter_visual_plan::ParsedChapterVisualPlan,
    generation::{GenerationRunCheckpointContext, RecordCompletedGenerationRunWithCheckpointInput},
};
use a4n_domain::{
    chapter::ResolvedChapterSource,
    generation::{GenerationRunTraceTarget, RecordCompletedGenerationRunInput},
};
use a4n_error::AppError;
use a4n_llm_context::{
    COMPRESSION_FULL_RAW, LlmContextLedger, LlmContextLedgerPackDraft, LlmContextLedgerPackInput,
    context_aware_system_user_request,
};
use a4n_llm_provider_contract::provider::{ChatRequest, LlmTraceContext};
use a4n_service_protocol::GenerationResultRef;
use serde_json::{Value, json};

use super::{ensure_plan_belongs_to_source, visual_plan_db_error};
use crate::{
    OrchestratorContext,
    generation::{
        checkpoint::GenerationCheckpointMetadata,
        generation_public_error,
        stream::{LlmGenerationCheckpoint, LlmGenerationRequest, execute_llm_generation},
    },
};

impl OrchestratorContext {
    pub(crate) async fn visual_plan_generation_source(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_id: i64,
        record_id: i64,
        revision: i32,
    ) -> Result<(ParsedChapterVisualPlan, ResolvedChapterSource), AppError> {
        if project_id <= 0 || chapter_id <= 0 || record_id <= 0 || revision <= 0 {
            return Err(AppError::InvalidArgument(
                "项目、章节、方案 ID 和 revision 必须为正整数".into(),
            ));
        }
        self.verify_project_or_derived_family_editor_validated(user_id, project_id)
            .await?;
        let source = self
            .resolve_visual_plan_source(project_id, chapter_id)
            .await?;
        let plan = a4n_db::chapter_visual_plan::chapter_visual_plan_by_id(
            self.database.executor(),
            record_id,
        )
        .await
        .map_err(visual_plan_db_error)?;
        ensure_plan_belongs_to_source(&plan, &source)?;
        if plan.record.revision != revision {
            return Err(visual_plan_db_error(
                a4n_db::chapter_visual_plan::ChapterVisualPlanDbError::RevisionConflict {
                    record_id,
                    expected_revision: revision,
                },
            ));
        }
        if plan.status != ChapterVisualPlanStatus::Draft {
            return Err(AppError::InvalidArgument(
                "请先创建导演编排草稿，再生成候选".into(),
            ));
        }
        Ok((plan, source))
    }

    pub(crate) async fn visual_plan_scene_context<'a>(
        &self,
        bindings: impl IntoIterator<Item = &'a SceneBindingV3>,
        source: &ResolvedChapterSource,
    ) -> Result<Vec<Value>, AppError> {
        let ids = bindings
            .into_iter()
            .flat_map(|scene| [scene.entry_scene_id(), scene.exit_scene_id()])
            .collect::<std::collections::BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let scenes =
            a4n_db::project::scene_profiles_for_source(self.database.executor(), source, &ids)
                .await
                .map_err(crate::shooting_script::errors::resolve_source_error)?;
        Ok(scenes.into_iter().map(|scene| json!({"id":scene.id,"name":scene.name,"description":scene.base_description,"visual_description":scene.visual_description,"spatial_layout":scene.spatial_layout,"atmosphere":scene.atmosphere})).collect())
    }

    pub(crate) async fn preview_visual_plan_point_generation(
        &self,
        user_id: i64,
        project_id: i64,
        options: &VisualPlanPointGenerationOptions,
    ) -> Result<VisualPlanPointGenerationPreview, AppError> {
        let (current, source) = self
            .visual_plan_generation_source(
                user_id,
                project_id,
                options.source_chapter_id,
                options.record_id,
                options.expected_revision,
            )
            .await?;
        validate_description(&options.user_description)?;
        let point = current
            .plan
            .point(&options.point_id)
            .ok_or_else(|| AppError::NotFound("图片点不存在".into()))?;
        let segment = current
            .plan
            .segments
            .iter()
            .find(|segment| {
                let start = current
                    .plan
                    .point(&segment.start_point_id)
                    .expect("validated start")
                    .at_ms;
                let end = current
                    .plan
                    .point(&segment.end_point_id)
                    .expect("validated end")
                    .at_ms;
                point.at_ms >= start && point.at_ms < end
            })
            .or_else(|| {
                (point.at_ms == current.plan.duration_ms())
                    .then(|| current.plan.segments.last())
                    .flatten()
            })
            .ok_or_else(|| AppError::InvalidArgument("图片点没有所属镜头段".into()))?;
        let scenes = self
            .visual_plan_scene_context(segment.scene_binding.iter(), &source)
            .await?;
        let context = json!({
            "frame":current.plan.frame,
            "point_description":point.description,
            "segment_description":segment.prompt,
            "scenes":scenes,"user_description":options.user_description,
        });
        let aspect_ratio = current
            .plan
            .frame
            .aspect_ratio()
            .map_err(AppError::InvalidArgument)?;
        if let Some(size) = options.size {
            if size.width <= 0 || size.height <= 0 {
                return Err(AppError::InvalidArgument("图片生成尺寸必须是正整数".into()));
            }
            if i64::from(size.width) * i64::from(current.plan.frame.height)
                != i64::from(size.height) * i64::from(current.plan.frame.width)
            {
                return Err(AppError::InvalidArgument(format!(
                    "图片生成尺寸 {}x{} 与方案画幅 {} 不匹配",
                    size.width, size.height, aspect_ratio
                )));
            }
        }
        Ok(VisualPlanPointGenerationPreview {
            prompt: format!(
                "为指定图片点生成一张完整分镜图片。遵循画幅、当前点的静态画面描述、所属镜头与场景资料，以及用户描述。参考图用于保持明确指定的主体与画面特征。不要画时间条、字幕或编辑器界面。\n{}",
                serde_json::to_string_pretty(&context)?
            ),
            reference_image_ids: point.reference_image_ids.clone(),
            aspect_ratio,
            plan_sha256: current.record.plan_sha256,
        })
    }

    pub(crate) async fn visual_plan_point_batch_item(
        &self,
        user_id: i64,
        project_id: i64,
        options: &VisualPlanPointGenerationOptions,
    ) -> Result<Value, AppError> {
        let preview = self
            .preview_visual_plan_point_generation(user_id, project_id, options)
            .await?;
        Ok(json!({
            "prompt":preview.prompt,"image_type":a4n_domain::image::IMAGE_TYPE_DIRECTOR_STORYBOARD_FRAME,
            "chapter_id":options.source_chapter_id,"config_id":options.config_id,
            "aspect_ratio":preview.aspect_ratio,"reference_image_ids":preview.reference_image_ids,
            "width":options.size.map(|size| size.width),"height":options.size.map(|size| size.height),
            "metadata_json":{"director_candidate":{
                "record_id":options.record_id,"plan_revision":options.expected_revision,
                "plan_sha256":preview.plan_sha256,"point_id":options.point_id,
                "user_description":options.user_description,
            }},
        }))
    }

    async fn prepare_visual_plan_prompt(
        &self,
        user_id: i64,
        input: &StartVisualPlanPromptGenerationInput,
    ) -> Result<(ChatRequest, VisualPlanPromptCandidate, Value), AppError> {
        let (current, source) = self
            .visual_plan_generation_source(
                user_id,
                input.project_id,
                input.source_chapter_id,
                input.record_id,
                input.expected_revision,
            )
            .await?;
        validate_description(&input.user_description)?;
        let segment = current
            .plan
            .segments
            .iter()
            .find(|segment| segment.id == input.segment_id)
            .ok_or_else(|| AppError::NotFound("镜头段不存在".into()))?;
        let start = current
            .plan
            .point(&segment.start_point_id)
            .expect("validated start")
            .at_ms;
        let end = current
            .plan
            .point(&segment.end_point_id)
            .expect("validated end");
        let points = current.plan.points.iter().filter(|point| point.at_ms >= start && point.at_ms < end.at_ms)
            .map(|point| json!({"relative_at_ms":point.at_ms - start,"label":point.label,"description":point.description})).collect::<Vec<_>>();
        let scenes = self
            .visual_plan_scene_context(segment.scene_binding.iter(), &source)
            .await?;
        let payload = json!({
            "duration_ms":end.at_ms-start,
            "existing_prompt":(segment.prompt.trim() != input.user_description.trim()).then_some(&segment.prompt),
            "scene_binding":segment.scene_binding,
            "keyframes":points,
            "explicit_end_frame":segment.use_end_point_image.then(|| json!({"relative_at_ms":end.at_ms-start,"description":end.description})),
            "user_description":input.user_description,
        });
        let system = self.prompt_render_template(
            DIRECTOR_SEGMENT_PROMPT,
            &HashMap::new(),
            Some(input.project_id),
        )?;
        let mut ledger = LlmContextLedger::new();
        ledger.add_pack(LlmContextLedgerPackInput {
            scope_kind: "chapter".into(),
            pack_kind: "director_scene_context".into(),
            source_refs_json: Some(json!({"record_id": input.record_id})),
            config_identity_json: json!({"contract": CHAPTER_VISUAL_PLAN_V3, "kind": "scene_context"}),
            required: true,
            stable_prefix: true,
            variants: vec![LlmContextLedgerPackDraft::new(COMPRESSION_FULL_RAW, json!({
                "chapter_title": source.title, "chapter_summary": source.summary, "scenes": scenes,
            }))],
        })?;
        ledger.add_pack(LlmContextLedgerPackInput {
            scope_kind: "chapter".into(),
            pack_kind: DIRECTOR_SEGMENT_PROMPT.into(),
            source_refs_json: Some(
                json!({"record_id":input.record_id,"plan_sha256":current.record.plan_sha256}),
            ),
            config_identity_json: json!({"contract":CHAPTER_VISUAL_PLAN_V3}),
            required: true,
            stable_prefix: false,
            variants: vec![LlmContextLedgerPackDraft::new(
                COMPRESSION_FULL_RAW,
                payload.clone(),
            )],
        })?;
        let request = context_aware_system_user_request(
            system,
            ledger.into_manifest("生成当前镜头段的变化描述", None),
            Some(0.4),
            None,
        )?
        .with_trace_context(LlmTraceContext {
            project_id: Some(input.project_id),
            root_project_id: None,
            workflow: DIRECTOR_SEGMENT_PROMPT.to_string(),
            step_key: format!("segment:{}", input.segment_id),
            generation_run_id: None,
            workflow_run_id: None,
            call_key: Some(format!("segment:{}", input.segment_id)),
            input_fingerprint_sha256: None,
            context_plan_json: None,
            context_pack_refs_json: None,
        });
        let candidate = VisualPlanPromptCandidate {
            record_id: input.record_id,
            source_chapter_id: input.source_chapter_id,
            plan_revision: input.expected_revision,
            plan_sha256: current.record.plan_sha256,
            segment_id: input.segment_id.clone(),
            prompt: String::new(),
            user_description: input.user_description.clone(),
            model: self.llm_model_name(),
        };
        Ok((request, candidate, payload))
    }

    pub(crate) async fn run_visual_plan_prompt_generation(
        &self,
        user_id: i64,
        input: StartVisualPlanPromptGenerationInput,
        handle: GenerationTaskHandle,
    ) -> Result<(), String> {
        let Some(execution) = self
            .acquire_generation_task_execution(&handle)
            .await
            .map_err(|error| error.to_string())?
        else {
            return Ok(());
        };
        let (request, mut candidate, payload) =
            match self.prepare_visual_plan_prompt(user_id, &input).await {
                Ok(prepared) => prepared,
                Err(error) => {
                    return Err(execution
                        .fail(generation_public_error(
                            "generation_request_invalid",
                            DIRECTOR_SEGMENT_PROMPT,
                            error,
                        ))
                        .await);
                }
            };
        let session = execution.session.clone();
        let output = match execute_llm_generation(
            self.llm_backend(),
            &execution.publisher,
            LlmGenerationRequest {
                request,
                stream_id: "prompt".into(),
                cancellation: execution.permit.cancellation_token(),
                usage_base: None,
                checkpoint: Some(LlmGenerationCheckpoint {
                    writer: &execution.checkpoint_writer,
                    session: &session,
                    metadata: GenerationCheckpointMetadata {
                        step_key: Some(DIRECTOR_SEGMENT_PROMPT.into()),
                        provider_model: Some(candidate.model.clone()),
                        ..Default::default()
                    },
                }),
            },
        )
        .await
        {
            Ok(output) => output,
            Err(error) => return Err(execution.fail(error).await),
        };
        candidate.prompt = output.content.trim().to_string();
        if candidate.prompt.is_empty() || candidate.prompt.chars().count() > 100_000 {
            return Err(execution
                .fail(generation_public_error(
                    "generation_validation_failed",
                    DIRECTOR_SEGMENT_PROMPT,
                    "模型未返回有效的镜头变化描述",
                ))
                .await);
        }
        let candidate_json = match serde_json::to_value(&candidate) {
            Ok(value) => value,
            Err(error) => {
                return Err(execution
                    .fail(generation_public_error(
                        "generation_persist_failed",
                        DIRECTOR_SEGMENT_PROMPT,
                        error,
                    ))
                    .await);
            }
        };
        let hash = match a4n_db::generation::canonical_json_sha256(&candidate_json) {
            Ok(hash) => hash,
            Err(error) => {
                return Err(execution
                    .fail(generation_public_error(
                        "generation_persist_failed",
                        DIRECTOR_SEGMENT_PROMPT,
                        error,
                    ))
                    .await);
            }
        };
        let input_hash = match a4n_db::generation::canonical_json_sha256(&payload) {
            Ok(hash) => hash,
            Err(error) => {
                return Err(execution
                    .fail(generation_public_error(
                        "generation_persist_failed",
                        DIRECTOR_SEGMENT_PROMPT,
                        error,
                    ))
                    .await);
            }
        };
        let record = a4n_db::generation::record_completed_run_with_checkpoint_transaction(self.database.executor(), RecordCompletedGenerationRunWithCheckpointInput {
            run:RecordCompletedGenerationRunInput {
                project_id:input.project_id,workflow:DIRECTOR_SEGMENT_PROMPT.into(),step_key:input.segment_id.clone(),title:"导演镜头段 prompt 候选".into(),model_kind:"llm".into(),provider:None,model:Some(candidate.model.clone()),
                prompt_template_names_json:Some(json!([DIRECTOR_SEGMENT_PROMPT])),prompt_template_hashes_json:None,
                rendered_messages_json:None,rendered_prompts_json:None,input_variables_json:Some(payload.clone()),
                source_refs_json:Some(json!({"record_id":input.record_id,"task_id":handle.task_id,"plan_sha256":candidate.plan_sha256})),
                media_refs_json:None,request_json:Some(payload),response_summary_json:Some(candidate_json.clone()),
                targets:vec![GenerationRunTraceTarget {target_kind:"input".into(),target_table:"chapter_visual_plan".into(),target_id:input.record_id}],
            },
            checkpoint:GenerationRunCheckpointContext {parsed_output_json:Some(candidate_json),input_fingerprint_sha256:Some(input_hash),validation_json:Some(json!({"candidate_sha256":hash})),..Default::default()},
        }).await;
        let record = match record {
            Ok(record) => record,
            Err(error) => {
                return Err(execution
                    .fail(generation_public_error(
                        "generation_persist_failed",
                        DIRECTOR_SEGMENT_PROMPT,
                        error,
                    ))
                    .await);
            }
        };
        execution
            .complete_with_result(
                DIRECTOR_SEGMENT_PROMPT,
                GenerationResultRef {
                    kind: "visual_plan_prompt_candidate".into(),
                    id: record.to_string(),
                    revision: hash,
                },
                "镜头变化描述已生成，等待采用".into(),
                vec![],
                None,
            )
            .await
    }

    pub(crate) async fn read_visual_plan_prompt_candidate(
        &self,
        user_id: i64,
        input: ReadVisualPlanPromptCandidateInput,
    ) -> Result<VisualPlanPromptCandidate, AppError> {
        if input.project_id <= 0 || input.generation_run_id <= 0 {
            return Err(AppError::InvalidArgument(
                "项目与候选 ID 必须为正整数".into(),
            ));
        }
        self.verify_project_or_derived_family_access_validated(user_id, input.project_id)
            .await?;
        let record = a4n_db::chapter_visual_plan::visual_plan_prompt_candidate_record(
            self.database.executor(),
            input.project_id,
            input.generation_run_id,
        )
        .await?
        .ok_or_else(|| AppError::NotFound("生成候选不存在".into()))?;
        let value = record
            .parsed_output_json
            .ok_or_else(|| AppError::InvalidArgument("生成候选缺少已持久化输出".into()))?;
        let expected_hash = record
            .validation_json
            .as_ref()
            .and_then(|value| value.get("candidate_sha256"))
            .and_then(Value::as_str);
        let actual_hash = a4n_db::generation::canonical_json_sha256(&value)?;
        if expected_hash != Some(actual_hash.as_str()) {
            return Err(AppError::InvalidArgument("生成候选内容哈希不一致".into()));
        }
        serde_json::from_value(value)
            .map_err(|error| AppError::InvalidArgument(format!("生成候选无效: {error}")))
    }
}

fn validate_description(value: &str) -> Result<(), AppError> {
    if value.chars().count() > 100_000 {
        return Err(AppError::InvalidArgument("生成描述最多 100000 字符".into()));
    }
    Ok(())
}
