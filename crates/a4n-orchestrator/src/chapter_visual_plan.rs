use std::collections::HashMap;

use a4n_api_contract::chapter_visual_plan::{
    ApplyChapterVisualPlanProposalInput, ChapterVisualPlanRecordInput,
    ChapterVisualPlanSourceInput, GenerateChapterVisualPlanDraftInput,
    ProposeChapterVisualPlanRevisionInput, SaveChapterVisualPlanDraftInput,
};
use a4n_core::{
    config::app_config::llm_token_profiles,
    error::AppError,
    source_spans::{SourceSpan, source_span_prompt_values, source_spans_for_content},
};
use a4n_db::{
    chapter_visual_plan::{
        ChapterVisualPlanDbError, ChapterVisualPlanSet, ConfirmChapterVisualPlanOutcome,
        ExpectedVisualPlanDraft, ParsedChapterVisualPlan,
    },
    project::ResolvedChapterSource,
};
use a4n_domain::{
    chapter_visual_plan::{
        ChapterVisualPlanAuthoringMode, ChapterVisualPlanSchemaVersion, ChapterVisualPlanV1,
        SourceSpanDispositionKindV1, SourceSpanDispositionV1, VisualCameraStateV1, VisualFrameV1,
        VisualInterpolationV1, VisualKeyframeV1, VisualNarrativeUnitV1, VisualShotPlanV1,
    },
    shooting_script::{ShootingCoverageV1, ShootingDirectingStatementV1, ShootingEditingModeV1},
};
use a4n_llm::provider::{ChatRequest, LlmTraceContext};
use serde_json::json;
use tokio_util::sync::CancellationToken;

use crate::{
    OrchestratorContext,
    structured_llm::{ChatJsonStreamInput, parse_strict_json_response},
};

pub const CHAPTER_VISUAL_PLAN_PROGRESS_TOTAL: u32 = 100;
const CHAPTER_VISUAL_PLAN_WORKFLOW: &str = "chapter_visual_plan";
const OUTLINE_PROMPT: &str = "chapter_visual_plan_outline";
const REVISION_PROMPT: &str = "chapter_visual_plan_revision";

pub type ChapterVisualPlanProgressReporter<'a> =
    &'a (dyn Fn(u32, u32, &str) -> Result<(), AppError> + Send + Sync);
pub type ChapterVisualPlanStreamReporter<'a> = &'a (dyn Fn(&str) + Send + Sync);

#[derive(Debug, Clone)]
pub struct ResolvedChapterVisualPlanSet {
    pub source: ResolvedChapterSource,
    pub source_spans: Vec<SourceSpan>,
    pub starter_plan: ChapterVisualPlanV1,
    pub plans: ChapterVisualPlanSet,
}

#[derive(Debug, Clone)]
pub struct ChapterVisualPlanProposal {
    pub record_id: i64,
    pub base_revision: i32,
    pub proposal: ChapterVisualPlanV1,
}

#[derive(Debug, Clone)]
pub struct PreparedChapterVisualPlanRequest {
    pub source: ResolvedChapterSource,
    pub request: ChatRequest,
    pub title: String,
}

impl OrchestratorContext {
    pub async fn current_chapter_visual_plan_for_user_validated(
        &self,
        user_id: i64,
        input: ChapterVisualPlanSourceInput,
    ) -> Result<ResolvedChapterVisualPlanSet, AppError> {
        self.verify_project_or_derived_family_access_validated(user_id, input.project_id)
            .await?;
        self.load_chapter_visual_plan_set(input).await
    }

    pub async fn save_chapter_visual_plan_draft_for_user_validated(
        &self,
        user_id: i64,
        input: SaveChapterVisualPlanDraftInput,
    ) -> Result<ParsedChapterVisualPlan, AppError> {
        self.verify_project_or_derived_family_editor_validated(user_id, input.project_id)
            .await?;
        let source = self
            .resolve_visual_plan_source(input.project_id, input.source_chapter_id)
            .await?;
        let expected = expected_draft(input.record_id, input.expected_revision)?;
        a4n_db::chapter_visual_plan::save_chapter_visual_plan_draft(
            self.database.connection(),
            a4n_db::chapter_visual_plan::SaveChapterVisualPlanDraftInput {
                source,
                author_user_id: user_id,
                authoring_mode: input.authoring_mode,
                expected,
                plan: input.plan,
            },
        )
        .await
        .map_err(visual_plan_db_error)
    }

    pub async fn confirm_chapter_visual_plan_for_user_validated(
        &self,
        user_id: i64,
        input: ChapterVisualPlanRecordInput,
    ) -> Result<ConfirmChapterVisualPlanOutcome, AppError> {
        self.verify_project_or_derived_family_editor_validated(user_id, input.project_id)
            .await?;
        let (source_result, current_result) = futures::join!(
            self.resolve_visual_plan_source(input.project_id, input.source_chapter_id),
            a4n_db::chapter_visual_plan::chapter_visual_plan_by_id(
                self.database.connection(),
                input.record_id,
            ),
        );
        let source = source_result?;
        let current = current_result.map_err(visual_plan_db_error)?;
        ensure_plan_belongs_to_source(&current, &source)?;
        a4n_db::chapter_visual_plan::confirm_chapter_visual_plan(
            self.database.connection(),
            a4n_db::chapter_visual_plan::ConfirmChapterVisualPlanInput {
                record_id: input.record_id,
                expected_revision: input.expected_revision,
            },
        )
        .await
        .map_err(visual_plan_db_error)
    }

    pub async fn discard_chapter_visual_plan_draft_for_user_validated(
        &self,
        user_id: i64,
        input: ChapterVisualPlanRecordInput,
    ) -> Result<(), AppError> {
        self.verify_project_or_derived_family_editor_validated(user_id, input.project_id)
            .await?;
        let (source_result, current_result) = futures::join!(
            self.resolve_visual_plan_source(input.project_id, input.source_chapter_id),
            a4n_db::chapter_visual_plan::chapter_visual_plan_by_id(
                self.database.connection(),
                input.record_id,
            ),
        );
        let source = source_result?;
        let current = current_result.map_err(visual_plan_db_error)?;
        ensure_plan_belongs_to_source(&current, &source)?;
        a4n_db::chapter_visual_plan::discard_chapter_visual_plan_draft(
            self.database.connection(),
            a4n_db::chapter_visual_plan::DiscardChapterVisualPlanDraftInput {
                record_id: input.record_id,
                expected_revision: input.expected_revision,
            },
        )
        .await
        .map_err(visual_plan_db_error)
    }

    pub async fn prepare_chapter_visual_plan_request_for_user_validated(
        &self,
        user_id: i64,
        input: GenerateChapterVisualPlanDraftInput,
    ) -> Result<PreparedChapterVisualPlanRequest, AppError> {
        self.verify_project_or_derived_family_editor_validated(user_id, input.project_id)
            .await?;
        let source = self
            .resolve_visual_plan_source(input.project_id, input.source_chapter_id)
            .await?;
        let source_spans = visual_plan_source_spans(&source)?;
        let starter_plan = starter_plan(&source, &source_spans)?;
        let request = self.build_visual_plan_request(
            &source,
            OUTLINE_PROMPT,
            "outline",
            json!({
                "task": "generate_chapter_visual_plan",
                "authoring_mode": input.authoring_mode,
                "user_direction": input.user_direction,
                "current_source": source_payload(&source, &source_spans),
                "starter_plan": starter_plan,
                "contract_guide": contract_guide(),
            }),
        )?;
        Ok(PreparedChapterVisualPlanRequest {
            title: format!("第 {} 章视觉编排", source.chapter_number),
            source,
            request,
        })
    }

    pub async fn generate_chapter_visual_plan_draft_for_user_validated(
        &self,
        user_id: i64,
        input: GenerateChapterVisualPlanDraftInput,
        progress: ChapterVisualPlanProgressReporter<'_>,
        stream_reporter: Option<ChapterVisualPlanStreamReporter<'_>>,
        cancel_token: Option<&CancellationToken>,
    ) -> Result<ParsedChapterVisualPlan, AppError> {
        let mode = input.authoring_mode;
        let project_id = input.project_id;
        let source_chapter_id = input.source_chapter_id;
        progress(
            0,
            CHAPTER_VISUAL_PLAN_PROGRESS_TOTAL,
            "正在读取章节和导演意图",
        )?;
        let prepared = self
            .prepare_chapter_visual_plan_request_for_user_validated(user_id, input)
            .await?;
        let proposal = self
            .run_visual_plan_request(
                prepared.request,
                "章节视觉编排",
                progress,
                stream_reporter,
                cancel_token,
            )
            .await?;
        validate_plan_for_source(&proposal, &prepared.source)?;
        progress(
            88,
            CHAPTER_VISUAL_PLAN_PROGRESS_TOTAL,
            "方案校验完成，正在保存草稿",
        )?;
        let saved = a4n_db::chapter_visual_plan::save_chapter_visual_plan_draft(
            self.database.connection(),
            a4n_db::chapter_visual_plan::SaveChapterVisualPlanDraftInput {
                source: prepared.source,
                author_user_id: user_id,
                authoring_mode: mode,
                expected: None,
                plan: proposal,
            },
        )
        .await
        .map_err(|error| match error {
            ChapterVisualPlanDbError::RevisionConflict { .. } => AppError::InvalidState {
                current: format!(
                    "visual_plan_draft_already_exists:project={project_id},chapter={source_chapter_id}"
                ),
                operation: "生成章节视觉编排草稿".into(),
            },
            other => visual_plan_db_error(other),
        })?;
        progress(
            100,
            CHAPTER_VISUAL_PLAN_PROGRESS_TOTAL,
            "章节视觉编排草稿已保存",
        )?;
        Ok(saved)
    }

    pub async fn propose_chapter_visual_plan_revision_for_user_validated(
        &self,
        user_id: i64,
        input: ProposeChapterVisualPlanRevisionInput,
        progress: ChapterVisualPlanProgressReporter<'_>,
        stream_reporter: Option<ChapterVisualPlanStreamReporter<'_>>,
        cancel_token: Option<&CancellationToken>,
    ) -> Result<ChapterVisualPlanProposal, AppError> {
        self.verify_project_or_derived_family_editor_validated(user_id, input.project_id)
            .await?;
        if input.instruction.trim().is_empty() {
            return Err(AppError::InvalidArgument("修改指令不能为空".into()));
        }
        let (source_result, current_result) = futures::join!(
            self.resolve_visual_plan_source(input.project_id, input.source_chapter_id),
            a4n_db::chapter_visual_plan::chapter_visual_plan_by_id(
                self.database.connection(),
                input.record_id,
            ),
        );
        let source = source_result?;
        let current = current_result.map_err(visual_plan_db_error)?;
        ensure_plan_belongs_to_source(&current, &source)?;
        ensure_base_revision(&current, input.base_revision)?;
        let source_spans = visual_plan_source_spans(&source)?;
        let request = self.build_visual_plan_request(
            &source,
            REVISION_PROMPT,
            "revision",
            json!({
                "task": "propose_chapter_visual_plan_revision",
                "instruction": input.instruction,
                "current_source": source_payload(&source, &source_spans),
                "base_plan": current.plan,
                "contract_guide": contract_guide(),
            }),
        )?;
        let proposal = self
            .run_visual_plan_request(
                request,
                "章节视觉编排修改提案",
                progress,
                stream_reporter,
                cancel_token,
            )
            .await?;
        validate_plan_for_source(&proposal, &source)?;
        Ok(ChapterVisualPlanProposal {
            record_id: current.record.id,
            base_revision: current.record.revision,
            proposal,
        })
    }

    pub async fn apply_chapter_visual_plan_proposal_for_user_validated(
        &self,
        user_id: i64,
        input: ApplyChapterVisualPlanProposalInput,
    ) -> Result<ParsedChapterVisualPlan, AppError> {
        self.verify_project_or_derived_family_editor_validated(user_id, input.project_id)
            .await?;
        let (source_result, current_result) = futures::join!(
            self.resolve_visual_plan_source(input.project_id, input.source_chapter_id),
            a4n_db::chapter_visual_plan::chapter_visual_plan_by_id(
                self.database.connection(),
                input.record_id,
            ),
        );
        let source = source_result?;
        let current = current_result.map_err(visual_plan_db_error)?;
        ensure_plan_belongs_to_source(&current, &source)?;
        a4n_db::chapter_visual_plan::save_chapter_visual_plan_draft(
            self.database.connection(),
            a4n_db::chapter_visual_plan::SaveChapterVisualPlanDraftInput {
                source,
                author_user_id: user_id,
                authoring_mode: ChapterVisualPlanAuthoringMode::Collaborative,
                expected: Some(ExpectedVisualPlanDraft {
                    record_id: input.record_id,
                    revision: input.base_revision,
                }),
                plan: input.proposal,
            },
        )
        .await
        .map_err(visual_plan_db_error)
    }

    pub(crate) async fn confirmed_visual_plan_for_shooting_script(
        &self,
        source: &ResolvedChapterSource,
        requested_id: Option<i64>,
    ) -> Result<ParsedChapterVisualPlan, AppError> {
        let confirmed = a4n_db::chapter_visual_plan::confirmed_chapter_visual_plan_for_source(
            self.database.connection(),
            source,
        )
        .await
        .map_err(visual_plan_db_error)?
        .ok_or_else(|| {
            AppError::NotFound(format!(
                "第 {} 章缺少已确认的章节视觉编排方案；请先在“导演编排”中确认方案",
                source.chapter_number
            ))
        })?;
        if requested_id.is_some_and(|id| id != confirmed.record.id) {
            return Err(AppError::InvalidArgument(format!(
                "visual_plan_id 不是当前正文的 confirmed 方案: requested={requested_id:?}, current={}",
                confirmed.record.id
            )));
        }
        Ok(confirmed)
    }

    async fn load_chapter_visual_plan_set(
        &self,
        input: ChapterVisualPlanSourceInput,
    ) -> Result<ResolvedChapterVisualPlanSet, AppError> {
        let source = self
            .resolve_visual_plan_source(input.project_id, input.source_chapter_id)
            .await?;
        let source_spans = visual_plan_source_spans(&source)?;
        let starter_plan = starter_plan(&source, &source_spans)?;
        let plans = a4n_db::chapter_visual_plan::chapter_visual_plans_for_source(
            self.database.connection(),
            &source,
        )
        .await
        .map_err(visual_plan_db_error)?;
        Ok(ResolvedChapterVisualPlanSet {
            source,
            source_spans,
            starter_plan,
            plans,
        })
    }

    async fn resolve_visual_plan_source(
        &self,
        project_id: i64,
        source_chapter_id: i64,
    ) -> Result<ResolvedChapterSource, AppError> {
        a4n_db::project::resolve_chapter_source(
            self.database.connection(),
            project_id,
            source_chapter_id,
        )
        .await
        .map_err(crate::shooting_script::errors::resolve_source_error)
    }

    fn build_visual_plan_request(
        &self,
        source: &ResolvedChapterSource,
        prompt_name: &str,
        step_key: &str,
        payload: serde_json::Value,
    ) -> Result<ChatRequest, AppError> {
        let system_prompt = self.prompt_render_template(
            prompt_name,
            &HashMap::new(),
            Some(source.requested_project_id),
        )?;
        let user_prompt = serde_json::to_string_pretty(&payload)?;
        Ok(ChatRequest::system_user(
            system_prompt,
            user_prompt,
            Some(0.2),
            None,
            Some(llm_token_profiles::EXTRA_LONG.into()),
        )
        .with_json_response_format()
        .with_stream_usage()
        .with_trace_context(LlmTraceContext {
            project_id: Some(source.requested_project_id),
            root_project_id: Some(source.root_project_id),
            workflow: CHAPTER_VISUAL_PLAN_WORKFLOW.into(),
            step_key: step_key.into(),
            generation_run_id: None,
            workflow_run_id: None,
            call_key: Some(format!("{step_key}:{}", source.source_chapter_id)),
            input_fingerprint_sha256: None,
            context_plan_json: None,
            context_pack_refs_json: None,
        }))
    }

    async fn run_visual_plan_request(
        &self,
        request: ChatRequest,
        label: &str,
        progress: ChapterVisualPlanProgressReporter<'_>,
        stream_reporter: Option<ChapterVisualPlanStreamReporter<'_>>,
        cancel_token: Option<&CancellationToken>,
    ) -> Result<ChapterVisualPlanV1, AppError> {
        progress(
            8,
            CHAPTER_VISUAL_PLAN_PROGRESS_TOTAL,
            "正在生成结构化导演编排",
        )?;
        let mut streamed = String::new();
        let mut stream_sink = stream_reporter.map(|reporter| {
            move |delta: &str| {
                streamed.push_str(delta);
                reporter(&streamed);
            }
        });
        let response = self
            .chat_json_stream_result_with_sink(
                ChatJsonStreamInput {
                    request,
                    prompt_name: CHAPTER_VISUAL_PLAN_WORKFLOW,
                    truncated_message: "章节视觉编排输出达到 max_tokens 上限，JSON 未完整闭合",
                    structured_progress: None,
                    cancel_token,
                    cancelled_message: "章节视觉编排生成已停止",
                },
                |_current, message| {
                    progress(70, CHAPTER_VISUAL_PLAN_PROGRESS_TOTAL, message)
                        .map_err(|error| error.to_string())
                },
                stream_sink
                    .as_mut()
                    .map(|sink| sink as &mut (dyn FnMut(&str) + Send)),
            )
            .await
            .map_err(|error| {
                if cancel_token.is_some_and(CancellationToken::is_cancelled) {
                    AppError::Cancelled
                } else {
                    AppError::LlmError(error)
                }
            })?;
        progress(
            80,
            CHAPTER_VISUAL_PLAN_PROGRESS_TOTAL,
            "模型已返回，正在校验方案",
        )?;
        let plan =
            parse_strict_json_response::<ChapterVisualPlanV1>(&response.response_content, label)?;
        plan.validate()
            .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
        Ok(plan)
    }
}

pub fn starter_plan(
    source: &ResolvedChapterSource,
    source_spans: &[SourceSpan],
) -> Result<ChapterVisualPlanV1, AppError> {
    if source_spans.is_empty() {
        return Err(AppError::InvalidArgument(
            "章节正文无法拆分出可编排 source span".into(),
        ));
    }
    let shot_duration = 2_500_u64;
    let duration_ms = shot_duration
        .checked_mul(
            u64::try_from(source_spans.len())
                .map_err(|_| AppError::InvalidArgument("章节 source span 数量超限".into()))?,
        )
        .ok_or_else(|| AppError::InvalidArgument("章节视觉编排时长超限".into()))?;
    let mut shots = Vec::with_capacity(source_spans.len());
    let mut dispositions = Vec::with_capacity(source_spans.len());
    for (offset, span) in source_spans.iter().enumerate() {
        let start_ms = shot_duration
            * u64::try_from(offset)
                .map_err(|_| AppError::InvalidArgument("章节 source span 数量超限".into()))?;
        let end_ms = start_ms + shot_duration;
        let shot_id = format!("shot_{}", offset + 1);
        shots.push(VisualShotPlanV1 {
            id: shot_id.clone(),
            title: format!("画面 {}", offset + 1),
            start_ms,
            end_ms,
            source_span_ids: vec![span.id.clone()],
            user_addition_ids: Vec::new(),
            narrative_purpose: span.text.clone(),
            coverage: ShootingCoverageV1::MustShow,
            shot_size: None,
            camera_angle: None,
            camera_height: None,
            lens_intent: None,
            composition: None,
            keyframes: vec![
                starter_keyframe(format!("kf_{}_start", offset + 1), start_ms, "开始"),
                starter_keyframe(format!("kf_{}_end", offset + 1), end_ms, "结束"),
            ],
            entity_events: Vec::new(),
            camera_events: Vec::new(),
            sound_events: Vec::new(),
            technique_events: Vec::new(),
            continuity_notes: Vec::new(),
        });
        dispositions.push(SourceSpanDispositionV1 {
            source_span_id: span.id.clone(),
            disposition: SourceSpanDispositionKindV1::Adapt,
            narrative_unit_id: Some("unit_1".into()),
            shot_ids: vec![shot_id],
            reason: None,
        });
    }
    let plan = ChapterVisualPlanV1 {
        schema_version: ChapterVisualPlanSchemaVersion::V1,
        frame: VisualFrameV1 {
            width: 1920,
            height: 1080,
            duration_ms,
            background: None,
        },
        directing_statement: ShootingDirectingStatementV1 {
            central_dramatic_question: "本章的核心悬念如何在画面中逐步揭示".into(),
            protagonist_throughline: "保持原文主角行动与情绪变化的连续性".into(),
            audience_experience: "清楚理解情节，同时保留原文节奏".into(),
            viewpoint_strategy: "以原文叙事焦点为主，用户可继续调整".into(),
            visual_strategy: "先按原文顺序建立可编辑关键画面".into(),
            spatial_strategy: "使用观众画框坐标明确实体位置关系".into(),
            sound_strategy: "对白、环境和效果音由用户按镜头补充".into(),
            rhythm_strategy: "每个 source span 建立一个等长初始镜头".into(),
        },
        entities: Vec::new(),
        user_additions: Vec::new(),
        narrative_units: vec![VisualNarrativeUnitV1 {
            id: "unit_1".into(),
            sequence_index: 1,
            timeline_position: 1,
            title: source.title.clone().unwrap_or_else(|| "本章".into()),
            start_ms: 0,
            end_ms: duration_ms,
            editing_mode: ShootingEditingModeV1::Continuity,
            intercut_group_id: None,
            narrative_purpose: "按原文顺序完成本章初始视觉编排".into(),
            location: None,
            time_of_day: None,
            participant_entity_ids: Vec::new(),
            focal_entity_id: None,
            source_span_ids: source_spans.iter().map(|span| span.id.clone()).collect(),
            user_addition_ids: Vec::new(),
            shots,
        }],
        transitions: Vec::new(),
        source_dispositions: dispositions,
    };
    validate_plan_for_source(&plan, source)?;
    Ok(plan)
}

fn starter_keyframe(id: String, at_ms: u64, label: &str) -> VisualKeyframeV1 {
    VisualKeyframeV1 {
        id,
        at_ms,
        label: label.into(),
        interpolation_to_next: VisualInterpolationV1::Hold,
        camera: VisualCameraStateV1 {
            center_x: 0.5,
            center_y: 0.5,
            zoom: 1.0,
            rotation_degrees: 0.0,
            focus_entity_id: None,
            framing_note: None,
        },
        entity_states: Vec::new(),
        relations: Vec::new(),
    }
}

fn visual_plan_source_spans(source: &ResolvedChapterSource) -> Result<Vec<SourceSpan>, AppError> {
    let spans = source_spans_for_content(
        source.source_chapter_id,
        source.chapter_number,
        &source.content,
    );
    if spans.is_empty() {
        Err(AppError::InvalidArgument(
            "章节正文无法拆分出可编排 source span".into(),
        ))
    } else {
        Ok(spans)
    }
}

fn validate_plan_for_source(
    plan: &ChapterVisualPlanV1,
    source: &ResolvedChapterSource,
) -> Result<(), AppError> {
    let source_spans = visual_plan_source_spans(source)?;
    plan.validate_source_span_coverage(source_spans.iter().map(|span| span.id.as_str()))
        .map_err(|error| AppError::InvalidArgument(error.to_string()))
}

fn source_payload(
    source: &ResolvedChapterSource,
    source_spans: &[SourceSpan],
) -> serde_json::Value {
    json!({
        "chapter_number": source.chapter_number,
        "title": source.title,
        "summary": source.summary,
        "analysis": source.analysis_data_json,
        "content": source.content,
        "content_sha256": source.content_sha256,
        "source_spans": source_span_prompt_values(source_spans),
    })
}

fn contract_guide() -> serde_json::Value {
    json!({
        "time_unit": "integer milliseconds",
        "frame_coordinates": "normalized x/y/width/height, top-left (0,0), bottom-right (1,1)",
        "entity_kinds": ["character", "object", "environment", "text", "effect"],
        "entity_event_kinds": ["enter", "exit", "move", "action", "dialogue", "expression", "gaze", "pose", "interaction", "transform", "custom"],
        "camera_event_kinds": ["cut", "pan", "tilt", "push_in", "pull_out", "track", "dolly", "crane", "handheld", "whip_pan", "zoom", "focus", "shake", "orbit", "reframe", "custom"],
        "sound_event_kinds": ["dialogue", "voice_over", "narration", "ambient", "effect", "music", "silence"],
        "technique_kinds": ["montage", "split_screen", "slow_motion", "fast_motion", "freeze_frame", "flashback", "dream", "subjective", "symbolic", "silhouette", "reflection", "overlay", "text_on_screen", "lighting", "color_grade", "match_action", "offscreen", "custom"],
        "relation_kinds": ["left_of", "right_of", "above", "below", "in_front_of", "behind", "inside", "overlapping", "facing", "looking_at", "touching", "holding", "following", "custom"],
        "relation_constraint": "Each relation endpoint must have a visible entity_state in the same keyframe. Do not connect hidden or absent entities.",
        "editing_modes": ["continuity", "intercut", "montage", "flashback", "parallel_action", "sound_bridge"],
        "transition_types": ["continuous", "hard_cut", "match_cut", "whip_pan", "action_wipe", "sound_bridge", "dissolve", "fade", "focus_transition", "morph"],
        "important_shape": "Return every field shown in starter_plan. Preserve null, [], and {} where no value is intended. Do not add unknown fields."
    })
}

fn expected_draft(
    record_id: Option<i64>,
    revision: Option<i32>,
) -> Result<Option<ExpectedVisualPlanDraft>, AppError> {
    match (record_id, revision) {
        (None, None) => Ok(None),
        (Some(record_id), Some(revision)) if record_id > 0 && revision > 0 => {
            Ok(Some(ExpectedVisualPlanDraft {
                record_id,
                revision,
            }))
        }
        _ => Err(AppError::InvalidArgument(
            "record_id 与 expected_revision 必须同时提供且大于 0".into(),
        )),
    }
}

fn ensure_plan_belongs_to_source(
    plan: &ParsedChapterVisualPlan,
    source: &ResolvedChapterSource,
) -> Result<(), AppError> {
    if plan.record.root_project_id != source.root_project_id
        || plan.record.source_chapter_id != source.source_chapter_id
        || plan.record.source_revision_key != source.source_revision_key
        || plan.record.source_content_sha256 != source.content_sha256
    {
        return Err(AppError::InvalidArgument(
            "章节视觉编排与当前章节正文不匹配".into(),
        ));
    }
    Ok(())
}

fn ensure_base_revision(
    plan: &ParsedChapterVisualPlan,
    base_revision: i32,
) -> Result<(), AppError> {
    if plan.record.revision == base_revision {
        Ok(())
    } else {
        Err(AppError::InvalidState {
            current: format!(
                "visual_plan_revision_conflict:id={},expected={},actual={}",
                plan.record.id, base_revision, plan.record.revision
            ),
            operation: "应用章节视觉编排修改".into(),
        })
    }
}

pub(crate) fn visual_plan_db_error(error: ChapterVisualPlanDbError) -> AppError {
    match error {
        ChapterVisualPlanDbError::InvalidArgument(message) => AppError::InvalidArgument(message),
        ChapterVisualPlanDbError::InvalidState(message) => AppError::InvalidState {
            current: message,
            operation: "读写章节视觉编排".into(),
        },
        ChapterVisualPlanDbError::NotFound(message) => AppError::NotFound(message),
        ChapterVisualPlanDbError::RevisionConflict {
            record_id,
            expected_revision,
        } => AppError::InvalidState {
            current: format!(
                "visual_plan_revision_conflict:id={record_id},expected={expected_revision}"
            ),
            operation: "更新章节视觉编排".into(),
        },
        ChapterVisualPlanDbError::Database(error) => AppError::Database(error),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn starter_plan_covers_every_source_span() {
        let source = ResolvedChapterSource {
            requested_project_id: 1,
            root_project_id: 1,
            source_chapter_id: 2,
            source_snapshot_id: None,
            source_snapshot_chapter_id: None,
            chapter_number: 1,
            title: Some("雨夜".into()),
            summary: None,
            analysis_data_json: None,
            content: "甲推门。乙回头。".into(),
            content_sha256: a4n_core::source_spans::source_text_hash("甲推门。乙回头。"),
            source_revision_key: "live:2".into(),
        };
        let spans = visual_plan_source_spans(&source).expect("spans");
        let plan = starter_plan(&source, &spans).expect("starter plan");
        assert_eq!(plan.narrative_units[0].shots.len(), spans.len());
        plan.validate_source_span_coverage(spans.iter().map(|span| span.id.as_str()))
            .expect("exact coverage");
    }
}
