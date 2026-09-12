use std::collections::{BTreeSet, HashMap};

use crate::OrchestratorContext;
mod tools;
use a4n_api_contract::chapter_visual_plan::{
    ChapterVisualPlanRecordInput, ChapterVisualPlanSourceInput, SaveChapterVisualPlanDraftInput,
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
        ChapterVisualPlanSchemaVersion, ChapterVisualPlanV1, ChapterVisualPlanV2,
        SourceSpanDispositionKindV1, SourceSpanDispositionV1, VisualCameraEventV1,
        VisualEntityEventV1, VisualEntityV1, VisualFrameV1, VisualFrameV2, VisualKeyframeV1,
        VisualNarrativeUnitV1, VisualSceneV2, VisualShotPlanV1, VisualSoundEventV1,
        VisualTechniqueEventV1, VisualTransitionV1,
    },
    shooting_script::ShootingDirectingStatementV1,
    source_spans::SourceSpan,
};
use a4n_error::AppError;

#[derive(Debug, Clone)]
pub struct ResolvedChapterVisualPlanSet {
    pub source: ResolvedChapterSource,
    pub starter_plan: ChapterVisualPlanV2,
    pub plans: ChapterVisualPlanSet,
}

impl OrchestratorContext {
    pub(crate) async fn current_chapter_visual_plan_for_user_validated(
        &self,
        user_id: i64,
        input: ChapterVisualPlanSourceInput,
    ) -> Result<ResolvedChapterVisualPlanSet, AppError> {
        self.verify_project_or_derived_family_access_validated(user_id, input.project_id)
            .await?;
        self.load_chapter_visual_plan_set(input).await
    }

    pub(crate) async fn save_chapter_visual_plan_draft_for_user_validated(
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
                expected_source_hash: input.expected_source_hash,
                plan: input.plan,
            },
        )
        .await
        .map_err(visual_plan_db_error)
    }

    pub(crate) async fn confirm_chapter_visual_plan_for_user_validated(
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

    pub(crate) async fn discard_chapter_visual_plan_draft_for_user_validated(
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
        let starter_plan = starter_plan();
        let plans = a4n_db::chapter_visual_plan::chapter_visual_plans_for_source(
            self.database.connection(),
            &source,
        )
        .await
        .map_err(visual_plan_db_error)?;
        Ok(ResolvedChapterVisualPlanSet {
            source,
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
}

#[async_trait::async_trait]
impl a4n_api_contract::chapter_visual_plan::ChapterVisualPlanReadApplicationPort
    for OrchestratorContext
{
    fn project_editor(
        &self,
        plan: &a4n_api_contract::chapter_visual_plan::ChapterVisualPlanV2,
        view: &a4n_api_contract::chapter_visual_plan::VisualPlanReadView,
    ) -> Result<a4n_api_contract::chapter_visual_plan::VisualPlanProjection, AppError> {
        tools::project_editor_visual_plan(plan, view)
    }

    async fn get(
        &self,
        user_id: i64,
        input: ChapterVisualPlanSourceInput,
    ) -> Result<a4n_api_contract::chapter_visual_plan::ChapterVisualPlanSetView, AppError> {
        self.current_chapter_visual_plan_for_user_validated(user_id, input)
            .await
            .map(chapter_visual_plan_set_view)
    }

    async fn read(
        &self,
        user_id: i64,
        input: a4n_api_contract::chapter_visual_plan::ReadChapterVisualPlanInput,
    ) -> Result<a4n_api_contract::chapter_visual_plan::ChapterVisualPlanReadResult, AppError> {
        self.read_chapter_visual_plan_for_user(user_id, input).await
    }

    async fn validate(
        &self,
        user_id: i64,
        input: a4n_api_contract::chapter_visual_plan::ValidateChapterVisualPlanInput,
    ) -> Result<a4n_api_contract::chapter_visual_plan::ChapterVisualPlanValidationResult, AppError>
    {
        self.validate_chapter_visual_plan_for_user(user_id, input)
            .await
    }
}

#[async_trait::async_trait]
impl a4n_api_contract::chapter_visual_plan::ChapterVisualPlanMutationApplicationPort
    for OrchestratorContext
{
    async fn save_draft(
        &self,
        user_id: i64,
        input: SaveChapterVisualPlanDraftInput,
    ) -> Result<a4n_api_contract::chapter_visual_plan::ChapterVisualPlanView, AppError> {
        self.save_chapter_visual_plan_draft_for_user_validated(user_id, input)
            .await
            .map(chapter_visual_plan_view)
    }

    async fn edit_draft(
        &self,
        user_id: i64,
        input: a4n_api_contract::chapter_visual_plan::EditChapterVisualPlanDraftInput,
    ) -> Result<a4n_api_contract::chapter_visual_plan::ChapterVisualPlanEditResult, AppError> {
        self.edit_chapter_visual_plan_draft_for_user(user_id, input)
            .await
    }

    async fn confirm(
        &self,
        user_id: i64,
        input: a4n_api_contract::chapter_visual_plan::ChapterVisualPlanRecordInput,
    ) -> Result<a4n_api_contract::chapter_visual_plan::ConfirmedChapterVisualPlanView, AppError>
    {
        let outcome = self
            .confirm_chapter_visual_plan_for_user_validated(user_id, input)
            .await?;
        Ok(
            a4n_api_contract::chapter_visual_plan::ConfirmedChapterVisualPlanView {
                visual_plan: chapter_visual_plan_view(outcome.visual_plan),
                invalidated_project_ids: outcome.invalidated_project_ids,
            },
        )
    }

    async fn discard(
        &self,
        user_id: i64,
        input: a4n_api_contract::chapter_visual_plan::ChapterVisualPlanRecordInput,
    ) -> Result<(), AppError> {
        self.discard_chapter_visual_plan_draft_for_user_validated(user_id, input)
            .await
    }
}

fn chapter_visual_plan_set_view(
    value: ResolvedChapterVisualPlanSet,
) -> a4n_api_contract::chapter_visual_plan::ChapterVisualPlanSetView {
    a4n_api_contract::chapter_visual_plan::ChapterVisualPlanSetView {
        source: a4n_api_contract::chapter_visual_plan::ChapterVisualPlanSourceView {
            chapter_number: value.source.chapter_number,
            title: value.source.title,
            content: value.source.content,
            content_sha256: value.source.content_sha256,
        },
        starter_plan: value.starter_plan,
        draft: value.plans.draft.map(chapter_visual_plan_view),
        confirmed: value.plans.confirmed.map(chapter_visual_plan_view),
    }
}

fn chapter_visual_plan_view(
    value: ParsedChapterVisualPlan,
) -> a4n_api_contract::chapter_visual_plan::ChapterVisualPlanView {
    a4n_api_contract::chapter_visual_plan::ChapterVisualPlanView {
        record: a4n_api_contract::chapter_visual_plan::ChapterVisualPlanRecordSummary {
            id: value.record.id,
            plan_version: value.record.plan_version,
            revision: value.record.revision,
            status: value.status,
            authoring_mode: value.authoring_mode,
            source_content_sha256: value.record.source_content_sha256,
            plan_sha256: value.record.plan_sha256,
            confirmed_at: value.record.confirmed_at.map(|value| value.to_string()),
        },
        plan: value.plan,
    }
}

fn starter_plan() -> ChapterVisualPlanV2 {
    ChapterVisualPlanV2 {
        schema_version: ChapterVisualPlanSchemaVersion::V2,
        frame: VisualFrameV2 {
            width: 1920,
            height: 1080,
            background: None,
        },
        entities: Vec::new(),
        scenes: Vec::new(),
    }
}

/// 把不携带正文绑定的 V2 临时投影为拍摄剧本仍在使用的 V1 执行输入。
/// source span 只存在于这一步，正文变化不会修改或阻塞 V2。
pub(crate) fn project_visual_plan_for_shooting_script(
    plan: &ChapterVisualPlanV2,
    source_spans: &[SourceSpan],
) -> Result<ChapterVisualPlanV1, AppError> {
    if source_spans.is_empty() {
        return Err(AppError::InvalidArgument(
            "章节正文无法拆分出可引用 source span".into(),
        ));
    }
    let projection = plan
        .project()
        .map_err(|error| AppError::InvalidArgument(error.to_string()))?;

    let source_ids = source_spans
        .iter()
        .map(|span| span.id.clone())
        .collect::<Vec<_>>();
    if source_ids.len() < projection.scenes.len() {
        return Err(AppError::InvalidArgument(format!(
            "source span 数量不足以为每个场景建立来源绑定: scenes={}, source_spans={}",
            projection.scenes.len(),
            source_ids.len()
        )));
    }

    // 先按场景顺序分桶；场景内镜头不足时只复用本场景的来源，不能按整条时间线循环到别的场景。
    let scene_source_ids = projection
        .scenes
        .iter()
        .enumerate()
        .map(|(scene_index, _)| {
            let start = scene_index * source_ids.len() / projection.scenes.len();
            let end = (scene_index + 1) * source_ids.len() / projection.scenes.len();
            source_ids[start..end].to_vec()
        })
        .collect::<Vec<_>>();
    let mut entity_source_span_ids = HashMap::<&str, BTreeSet<&str>>::new();
    for (scene_index, scene_projection) in projection.scenes.iter().enumerate() {
        for entity_id in scene_entity_ids(&scene_projection.scene) {
            entity_source_span_ids
                .entry(entity_id)
                .or_default()
                .extend(scene_source_ids[scene_index].iter().map(String::as_str));
        }
    }
    let entities = plan
        .entities
        .iter()
        .filter_map(|entity| {
            let source_span_ids = entity_source_span_ids.get(entity.id.as_str())?;
            Some(VisualEntityV1 {
                id: entity.id.clone(),
                label: entity.label.clone(),
                kind: entity.kind,
                description: entity.description.clone(),
                source_span_ids: source_span_ids.iter().map(|id| (*id).to_string()).collect(),
                user_addition_ids: Vec::new(),
            })
        })
        .collect::<Vec<_>>();
    let mut scenes = Vec::with_capacity(projection.scenes.len());
    let mut transitions = Vec::new();
    let mut source_bindings = source_ids
        .iter()
        .cloned()
        .map(|source_id| (source_id, (None::<String>, Vec::<String>::new())))
        .collect::<HashMap<_, _>>();
    if source_bindings.len() != source_ids.len() {
        return Err(AppError::InvalidArgument(
            "章节 source span ID 不能重复".into(),
        ));
    }

    for (scene_index, scene_projection) in projection.scenes.iter().enumerate() {
        let scene_source_ids = scene_source_ids[scene_index].clone();
        let mut shots = Vec::with_capacity(scene_projection.shots.len());
        for (scene_shot_index, shot_projection) in scene_projection.shots.iter().enumerate() {
            let shot = &shot_projection.shot;
            let shot_source_start =
                scene_shot_index * scene_source_ids.len() / scene_projection.shots.len();
            let shot_source_end =
                (scene_shot_index + 1) * scene_source_ids.len() / scene_projection.shots.len();
            let source_ids_for_shot = if shot_source_start < shot_source_end {
                scene_source_ids[shot_source_start..shot_source_end].to_vec()
            } else {
                // 当前场景镜头多于来源时允许重复绑定当前场景来源，避免跨场景伪造归属。
                vec![scene_source_ids[shot_source_start % scene_source_ids.len()].clone()]
            };
            let scene_id = scene_projection.scene.id.clone();
            for source_id in &source_ids_for_shot {
                let (bound_scene_id, shot_ids) = source_bindings
                    .get_mut(source_id)
                    .ok_or_else(|| AppError::InvalidArgument("章节 source span 不存在".into()))?;
                if let Some(bound_scene_id) = bound_scene_id {
                    if bound_scene_id != &scene_id {
                        return Err(AppError::InvalidArgument(
                            "source span 不能跨场景绑定".into(),
                        ));
                    }
                } else {
                    *bound_scene_id = Some(scene_id.clone());
                }
                shot_ids.push(shot.id.clone());
            }
            let keyframes = shot_projection
                .keyframes
                .iter()
                .map(|item| VisualKeyframeV1 {
                    id: item.keyframe.id.clone(),
                    at_ms: item.at_ms,
                    label: item.keyframe.label.clone(),
                    interpolation_to_next: item.keyframe.interpolation_to_next,
                    camera: item.keyframe.camera.clone(),
                    entity_states: item.keyframe.entity_states.clone(),
                    relations: item.keyframe.relations.clone(),
                })
                .collect::<Vec<_>>();
            let entity_events = shot_projection
                .entity_events
                .iter()
                .map(|item| VisualEntityEventV1 {
                    id: item.event.id.clone(),
                    kind: item.event.kind,
                    start_ms: item.start_ms,
                    end_ms: item.end_ms,
                    entity_id: item.event.entity_id.clone(),
                    target_entity_id: item.event.target_entity_id.clone(),
                    description: item.event.description.clone(),
                    source_span_ids: source_ids_for_shot.clone(),
                    user_addition_ids: Vec::new(),
                    parameters: item.event.parameters.clone(),
                })
                .collect::<Vec<_>>();
            let camera_events = shot_projection
                .camera_events
                .iter()
                .map(|item| VisualCameraEventV1 {
                    id: item.event.id.clone(),
                    kind: item.event.kind,
                    start_ms: item.start_ms,
                    end_ms: item.end_ms,
                    target_entity_id: item.event.target_entity_id.clone(),
                    description: item.event.description.clone(),
                    source_span_ids: source_ids_for_shot.clone(),
                    user_addition_ids: Vec::new(),
                    parameters: item.event.parameters.clone(),
                })
                .collect::<Vec<_>>();
            let sound_events = shot_projection
                .sound_events
                .iter()
                .map(|item| VisualSoundEventV1 {
                    id: item.event.id.clone(),
                    kind: item.event.kind,
                    start_ms: item.start_ms,
                    end_ms: item.end_ms,
                    speaker_entity_id: item.event.speaker_entity_id.clone(),
                    content: item.event.content.clone(),
                    source_span_ids: source_ids_for_shot.clone(),
                    user_addition_ids: Vec::new(),
                    parameters: item.event.parameters.clone(),
                })
                .collect::<Vec<_>>();
            let technique_events = shot_projection
                .technique_events
                .iter()
                .map(|item| VisualTechniqueEventV1 {
                    id: item.event.id.clone(),
                    kind: item.event.kind,
                    start_ms: item.start_ms,
                    end_ms: item.end_ms,
                    description: item.event.description.clone(),
                    source_span_ids: source_ids_for_shot.clone(),
                    user_addition_ids: Vec::new(),
                    parameters: item.event.parameters.clone(),
                })
                .collect::<Vec<_>>();
            if let Some(transition) = shot.transition_to_next.as_ref() {
                let next_shot_id = scene_projection
                    .shots
                    .get(scene_shot_index + 1)
                    .map(|next| next.shot.id.clone())
                    .ok_or_else(|| AppError::InvalidArgument("最后一个镜头不能携带转场".into()))?;
                transitions.push(VisualTransitionV1 {
                    id: format!("transition_{}", shot.id),
                    from_shot_id: shot.id.clone(),
                    to_shot_id: next_shot_id,
                    at_ms: shot_projection.end_ms,
                    duration_ms: transition.duration_ms,
                    kind: transition.kind,
                    preserved_action: transition.preserved_action.clone(),
                    preserved_sound: transition.preserved_sound.clone(),
                    notes: transition.notes.clone(),
                });
            }
            shots.push(VisualShotPlanV1 {
                id: shot.id.clone(),
                title: shot.title.clone(),
                start_ms: shot_projection.start_ms,
                end_ms: shot_projection.end_ms,
                source_span_ids: source_ids_for_shot,
                user_addition_ids: Vec::new(),
                narrative_purpose: shot.narrative_purpose.clone(),
                coverage: shot.coverage,
                shot_size: shot.shot_size,
                camera_angle: shot.camera_angle,
                camera_height: shot.camera_height,
                lens_intent: shot.lens_intent.clone(),
                composition: shot.composition.clone(),
                keyframes,
                entity_events,
                camera_events,
                sound_events,
                technique_events,
                continuity_notes: shot.continuity_notes.clone(),
            });
        }
        scenes.push(VisualNarrativeUnitV1 {
            id: scene_projection.scene.id.clone(),
            sequence_index: scene_projection.sequence_index,
            timeline_position: scene_projection.sequence_index,
            title: scene_projection.scene.title.clone(),
            start_ms: scene_projection.start_ms,
            end_ms: scene_projection.end_ms,
            editing_mode: scene_projection.scene.editing_mode,
            intercut_group_id: scene_projection.scene.intercut_group_id.clone(),
            narrative_purpose: scene_projection.scene.narrative_purpose.clone(),
            location: scene_projection.scene.location.clone(),
            time_of_day: scene_projection.scene.time_of_day.clone(),
            participant_entity_ids: scene_projection.scene.participant_entity_ids.clone(),
            focal_entity_id: scene_projection.scene.focal_entity_id.clone(),
            source_span_ids: scene_source_ids,
            user_addition_ids: Vec::new(),
            shots,
        });
    }
    let source_dispositions = source_ids
        .iter()
        .map(|source_id| {
            let (scene_id, shot_ids) = source_bindings
                .remove(source_id)
                .ok_or_else(|| AppError::InvalidArgument("章节 source span 未建立绑定".into()))?;
            let narrative_unit_id = scene_id
                .ok_or_else(|| AppError::InvalidArgument("章节 source span 未绑定到场景".into()))?;
            Ok(SourceSpanDispositionV1 {
                source_span_id: source_id.clone(),
                disposition: SourceSpanDispositionKindV1::Adapt,
                narrative_unit_id: Some(narrative_unit_id),
                shot_ids,
                reason: None,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    // V1 执行合同仍要求导演陈述；V2 明确不持久化该字段，因此这里只在适配输出中生成。
    let projected = ChapterVisualPlanV1 {
        frame: VisualFrameV1 {
            width: plan.frame.width,
            height: plan.frame.height,
            duration_ms: projection.duration_ms,
            background: plan.frame.background.clone(),
        },
        directing_statement: projected_directing_statement(plan),
        entities,
        user_additions: Vec::new(),
        narrative_units: scenes,
        transitions,
        source_dispositions,
    };
    projected
        .validate_source_span_coverage(source_spans.iter().map(|span| span.id.as_str()))
        .map_err(|error| AppError::InvalidArgument(error.to_string()))?;
    Ok(projected)
}

fn scene_entity_ids(scene: &VisualSceneV2) -> BTreeSet<&str> {
    let mut ids = scene
        .participant_entity_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    ids.extend(scene.focal_entity_id.as_deref());
    for shot in &scene.shots {
        for keyframe in &shot.keyframes {
            for state in &keyframe.entity_states {
                ids.insert(state.entity_id.as_str());
                ids.extend(state.gaze_target_entity_id.as_deref());
            }
            ids.extend(keyframe.camera.focus_entity_id.as_deref());
            for relation in &keyframe.relations {
                ids.insert(relation.source_entity_id.as_str());
                ids.insert(relation.target_entity_id.as_str());
            }
        }
        for event in &shot.entity_events {
            ids.insert(event.entity_id.as_str());
            ids.extend(event.target_entity_id.as_deref());
        }
        for event in &shot.camera_events {
            ids.extend(event.target_entity_id.as_deref());
        }
        for event in &shot.sound_events {
            ids.extend(event.speaker_entity_id.as_deref());
        }
    }
    ids
}

fn projected_directing_statement(plan: &ChapterVisualPlanV2) -> ShootingDirectingStatementV1 {
    ShootingDirectingStatementV1 {
        central_dramatic_question: "观众在每个场景中应该持续等待什么变化".into(),
        protagonist_throughline: "沿场景和镜头顺序推进视觉行动".into(),
        audience_experience: "先理解画面关系，再感受镜头节奏变化".into(),
        viewpoint_strategy: "以当前关键帧中明确的实体和焦点为视点依据".into(),
        visual_strategy: format!("{} 个场景共同组成连续视觉方案", plan.scenes.len()),
        spatial_strategy: "使用画幅坐标、层级和实体关系保持空间连续".into(),
        sound_strategy: "声音事件只在对应镜头的相对时间范围内发生".into(),
        rhythm_strategy: "镜头总时长由场景内镜头顺序累加得到".into(),
    }
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
        || plan.record.source_snapshot_id != source.source_snapshot_id
        || plan.record.source_snapshot_chapter_id != source.source_snapshot_chapter_id
    {
        return Err(AppError::InvalidArgument(
            "章节视觉编排与当前章节正文不匹配".into(),
        ));
    }
    Ok(())
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
    use a4n_db::project::ResolvedChapterSource;
    use a4n_domain::chapter_visual_plan::VisualEntityV2;
    use a4n_domain::source_spans::SourceSpan;

    use super::*;

    fn source() -> ResolvedChapterSource {
        ResolvedChapterSource {
            requested_project_id: 1,
            root_project_id: 1,
            source_chapter_id: 3,
            source_snapshot_id: None,
            source_snapshot_chapter_id: None,
            chapter_number: 1,
            title: Some("测试".into()),
            summary: None,
            analysis_data_json: None,
            content: "甲进入教室。乙抬头看他。".into(),
            content_sha256: "content-hash".into(),
            source_revision_key: "revision-1".into(),
        }
    }

    fn source_span(id: &str) -> SourceSpan {
        SourceSpan {
            id: id.into(),
            chapter_id: 3,
            chapter_number: 1,
            paragraph_index: 1,
            sentence_index: 1,
            fragment_index: 1,
            text: id.into(),
            char_start: 0,
            char_end: 1,
        }
    }

    fn two_scene_plan() -> ChapterVisualPlanV2 {
        let parsed = crate::shooting_script::visual_plan_fixture(&source());
        let mut plan = parsed.plan;
        let mut second_scene = plan.scenes[0].clone();
        second_scene.id = "scene-2".into();
        for (index, shot) in second_scene.shots.iter_mut().enumerate() {
            let shot_id = format!("shot-{}", index + 3);
            shot.id = shot_id.clone();
            for (keyframe_index, keyframe) in shot.keyframes.iter_mut().enumerate() {
                keyframe.id = format!(
                    "{shot_id}-{}",
                    if keyframe_index == 0 { "start" } else { "end" }
                );
            }
        }
        plan.scenes.push(second_scene);
        plan
    }

    #[test]
    fn projection_keeps_source_dispositions_inside_their_scene() {
        let plan = two_scene_plan();
        let projected = project_visual_plan_for_shooting_script(
            &plan,
            &[source_span("span-1"), source_span("span-2")],
        )
        .expect("projection");

        assert_eq!(projected.narrative_units[0].source_span_ids, ["span-1"]);
        assert_eq!(projected.narrative_units[1].source_span_ids, ["span-2"]);
        assert!(
            projected.narrative_units[0]
                .shots
                .iter()
                .all(|shot| shot.source_span_ids == ["span-1"])
        );
        assert!(
            projected.narrative_units[1]
                .shots
                .iter()
                .all(|shot| shot.source_span_ids == ["span-2"])
        );
        assert_eq!(
            projected.source_dispositions[0]
                .narrative_unit_id
                .as_deref(),
            Some("scene-1")
        );
        assert_eq!(
            projected.source_dispositions[1]
                .narrative_unit_id
                .as_deref(),
            Some("scene-2")
        );
    }

    #[test]
    fn projection_rejects_fewer_source_spans_than_scenes() {
        let error =
            project_visual_plan_for_shooting_script(&two_scene_plan(), &[source_span("span-1")])
                .expect_err("each scene needs a source span");
        assert!(
            error
                .to_string()
                .contains("source span 数量不足以为每个场景建立来源绑定")
        );
    }

    #[test]
    fn projection_reuses_only_current_scene_sources_when_scene_has_more_shots() {
        let mut plan = two_scene_plan();
        let mut extra_shot = plan.scenes[1].shots[0].clone();
        extra_shot.id = "shot-extra".into();
        for (keyframe_index, keyframe) in extra_shot.keyframes.iter_mut().enumerate() {
            keyframe.id = format!(
                "shot-extra-{}",
                if keyframe_index == 0 { "start" } else { "end" }
            );
        }
        plan.scenes[1].shots.push(extra_shot);

        let projected = project_visual_plan_for_shooting_script(
            &plan,
            &[
                source_span("span-1"),
                source_span("span-2"),
                source_span("span-3"),
            ],
        )
        .expect("projection");

        assert_eq!(projected.narrative_units[0].source_span_ids, ["span-1"]);
        assert_eq!(
            projected.narrative_units[1].source_span_ids,
            ["span-2", "span-3"]
        );
        assert_eq!(
            projected.narrative_units[1]
                .shots
                .iter()
                .map(|shot| shot.source_span_ids.clone())
                .collect::<Vec<_>>(),
            vec![
                vec![String::from("span-2")],
                vec![String::from("span-2")],
                vec![String::from("span-3")],
            ]
        );
        assert!(projected.narrative_units[0].shots.iter().all(|shot| {
            shot.source_span_ids
                .iter()
                .all(|source_span_id| source_span_id == "span-1")
        }));
        assert!(projected.narrative_units[1].shots.iter().all(|shot| {
            shot.source_span_ids
                .iter()
                .all(|source_span_id| source_span_id != "span-1")
        }));
    }

    #[test]
    fn projection_scopes_entity_sources_to_scenes_where_they_appear() {
        let mut plan = two_scene_plan();
        plan.entities = vec![
            VisualEntityV2 {
                id: "entity-1".into(),
                label: "甲".into(),
                kind: a4n_domain::chapter_visual_plan::VisualEntityKindV1::Character,
                description: "甲".into(),
            },
            VisualEntityV2 {
                id: "entity-2".into(),
                label: "乙".into(),
                kind: a4n_domain::chapter_visual_plan::VisualEntityKindV1::Character,
                description: "乙".into(),
            },
        ];
        plan.scenes[0].participant_entity_ids = vec!["entity-1".into()];
        plan.scenes[1].participant_entity_ids = vec!["entity-2".into()];

        let projected = project_visual_plan_for_shooting_script(
            &plan,
            &[source_span("span-1"), source_span("span-2")],
        )
        .expect("projection");

        assert_eq!(projected.entities[0].source_span_ids, ["span-1"]);
        assert_eq!(projected.entities[1].source_span_ids, ["span-2"]);
    }

    #[test]
    fn projection_excludes_entities_without_scene_usage() {
        let mut plan = two_scene_plan();
        plan.entities.push(VisualEntityV2 {
            id: "unused-entity".into(),
            label: "未使用".into(),
            kind: a4n_domain::chapter_visual_plan::VisualEntityKindV1::Object,
            description: "未使用实体".into(),
        });

        let projected = project_visual_plan_for_shooting_script(
            &plan,
            &[source_span("span-1"), source_span("span-2")],
        )
        .expect("unused library entities are excluded from execution projection");
        assert!(
            projected
                .entities
                .iter()
                .all(|entity| entity.id != "unused-entity")
        );
    }

    #[test]
    fn starter_plan_is_an_empty_draft() {
        let plan = starter_plan();
        assert!(plan.scenes.is_empty());
        plan.validate_draft().expect("draft validation");
    }
}
