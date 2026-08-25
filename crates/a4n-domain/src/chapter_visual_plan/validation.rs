use std::collections::{BTreeMap, BTreeSet, HashMap};

use thiserror::Error;

use super::contracts::*;
use crate::shooting_script::ShootingTransitionTypeV1;

#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("章节视觉编排字段 {path} 无效: {message}")]
pub struct ChapterVisualPlanValidationError {
    pub path: String,
    pub message: String,
}

impl ChapterVisualPlanV1 {
    pub fn validate(&self) -> Result<(), ChapterVisualPlanValidationError> {
        validate_plan(self)
    }

    /// 返回稳定的、按合同字段序列化的 JSON；数据库和 API 回读都应基于此结果。
    pub fn canonical_json(&self) -> Result<serde_json::Value, ChapterVisualPlanValidationError> {
        self.validate()?;
        serde_json::to_value(self).map_err(|error| ChapterVisualPlanValidationError {
            path: "$".into(),
            message: format!("序列化失败: {error}"),
        })
    }

    /// 校验当前方案恰好处理了本次正文的全部稳定 source span。
    pub fn validate_source_span_coverage(
        &self,
        available_source_span_ids: impl IntoIterator<Item = impl AsRef<str>>,
    ) -> Result<(), ChapterVisualPlanValidationError> {
        self.validate()?;
        let available = available_source_span_ids
            .into_iter()
            .map(|value| value.as_ref().to_string())
            .collect::<BTreeSet<_>>();
        let disposed = self
            .source_dispositions
            .iter()
            .map(|value| value.source_span_id.clone())
            .collect::<BTreeSet<_>>();
        if available == disposed {
            return Ok(());
        }
        let missing = available.difference(&disposed).cloned().collect::<Vec<_>>();
        let unknown = disposed.difference(&available).cloned().collect::<Vec<_>>();
        invalid(
            "source_dispositions",
            format!("必须且只能覆盖当前正文 source span；missing={missing:?}, unknown={unknown:?}"),
        )
    }
}

fn validate_plan(plan: &ChapterVisualPlanV1) -> Result<(), ChapterVisualPlanValidationError> {
    if plan.frame.width == 0 || plan.frame.height == 0 {
        return invalid("frame", "画幅宽高必须大于 0");
    }
    if plan.frame.duration_ms == 0 {
        return invalid("frame.duration_ms", "总时长必须大于 0");
    }
    required_text(
        &plan.directing_statement.central_dramatic_question,
        "directing_statement.central_dramatic_question",
    )?;
    required_text(
        &plan.directing_statement.protagonist_throughline,
        "directing_statement.protagonist_throughline",
    )?;
    required_text(
        &plan.directing_statement.audience_experience,
        "directing_statement.audience_experience",
    )?;
    required_text(
        &plan.directing_statement.viewpoint_strategy,
        "directing_statement.viewpoint_strategy",
    )?;
    required_text(
        &plan.directing_statement.visual_strategy,
        "directing_statement.visual_strategy",
    )?;
    required_text(
        &plan.directing_statement.spatial_strategy,
        "directing_statement.spatial_strategy",
    )?;
    required_text(
        &plan.directing_statement.sound_strategy,
        "directing_statement.sound_strategy",
    )?;
    required_text(
        &plan.directing_statement.rhythm_strategy,
        "directing_statement.rhythm_strategy",
    )?;

    let mut entity_ids = BTreeSet::new();
    let mut addition_ids = BTreeSet::new();
    let mut unit_ids = BTreeSet::new();
    let mut shot_ids = BTreeSet::new();
    let mut keyframe_ids = BTreeSet::new();
    let mut event_ids = BTreeSet::new();
    let adapted_source_span_ids = plan
        .source_dispositions
        .iter()
        .filter(|disposition| disposition.disposition == SourceSpanDispositionKindV1::Adapt)
        .map(|disposition| disposition.source_span_id.clone())
        .collect::<BTreeSet<_>>();

    for (index, addition) in plan.user_additions.iter().enumerate() {
        let path = format!("user_additions[{index}]");
        unique_text(&addition.id, &format!("{path}.id"), &mut addition_ids)?;
        required_text(&addition.description, &format!("{path}.description"))?;
        required_text(&addition.rationale, &format!("{path}.rationale"))?;
        if addition
            .introduced_at_ms
            .is_some_and(|at| at >= plan.frame.duration_ms)
        {
            return invalid(format!("{path}.introduced_at_ms"), "必须严格早于画面总时长");
        }
    }
    validate_disposition_shapes(plan)?;
    for (index, entity) in plan.entities.iter().enumerate() {
        let path = format!("entities[{index}]");
        unique_text(&entity.id, &format!("{path}.id"), &mut entity_ids)?;
        required_text(&entity.label, &format!("{path}.label"))?;
        required_text(&entity.description, &format!("{path}.description"))?;
        require_known_ids(
            &entity.source_span_ids,
            &adapted_source_span_ids,
            &format!("{path}.source_span_ids"),
        )?;
        require_known_ids(
            &entity.user_addition_ids,
            &addition_ids,
            &format!("entities[{index}].user_addition_ids"),
        )?;
        require_provenance(&entity.source_span_ids, &entity.user_addition_ids, &path)?;
    }

    if plan.narrative_units.is_empty() {
        return invalid("narrative_units", "至少包含一个叙事单元");
    }
    let mut previous_unit_end = 0;
    let mut sequence_indices = BTreeSet::new();
    let mut timeline_positions = BTreeSet::new();
    for (index, unit) in plan.narrative_units.iter().enumerate() {
        let path = format!("narrative_units[{index}]");
        unique_text(&unit.id, &format!("{path}.id"), &mut unit_ids)?;
        required_text(&unit.title, &format!("{path}.title"))?;
        required_text(
            &unit.narrative_purpose,
            &format!("{path}.narrative_purpose"),
        )?;
        require_positive_index(unit.sequence_index, &format!("{path}.sequence_index"))?;
        require_positive_index(unit.timeline_position, &format!("{path}.timeline_position"))?;
        if !sequence_indices.insert(unit.sequence_index) {
            return invalid(format!("{path}.sequence_index"), "全章必须唯一");
        }
        if !timeline_positions.insert(unit.timeline_position) {
            return invalid(format!("{path}.timeline_position"), "全章必须唯一");
        }
        validate_intercut_group_shape(unit, &path)?;
        validate_range(
            unit.start_ms,
            unit.end_ms,
            plan.frame.duration_ms,
            &format!("{path}.time"),
        )?;
        if unit.start_ms != previous_unit_end {
            return invalid(
                format!("{path}.start_ms"),
                "叙事单元必须从 0 开始连续覆盖时间线",
            );
        }
        previous_unit_end = unit.end_ms;
        require_known_ids(
            &unit.source_span_ids,
            &adapted_source_span_ids,
            &format!("{path}.source_span_ids"),
        )?;
        require_known_ids(
            &unit.participant_entity_ids,
            &entity_ids,
            &format!("{path}.participant_entity_ids"),
        )?;
        if let Some(focal) = unit.focal_entity_id.as_deref() {
            require_known_id(focal, &entity_ids, &format!("{path}.focal_entity_id"))?;
            if !unit.participant_entity_ids.iter().any(|id| id == focal) {
                return invalid(
                    format!("{path}.focal_entity_id"),
                    "必须属于 participant_entity_ids",
                );
            }
        }
        require_known_ids(
            &unit.user_addition_ids,
            &addition_ids,
            &format!("{path}.user_addition_ids"),
        )?;
        require_provenance(&unit.source_span_ids, &unit.user_addition_ids, &path)?;
        if unit.shots.is_empty() {
            return invalid(format!("{path}.shots"), "至少包含一个镜头");
        }
        let mut context = ShotValidationContext {
            frame_duration_ms: plan.frame.duration_ms,
            entity_ids: &entity_ids,
            addition_ids: &addition_ids,
            adapted_source_span_ids: &adapted_source_span_ids,
            shot_ids: &mut shot_ids,
            keyframe_ids: &mut keyframe_ids,
            event_ids: &mut event_ids,
        };
        validate_shots(unit, &mut context)?;
    }
    if previous_unit_end != plan.frame.duration_ms {
        return invalid("narrative_units", "叙事单元必须连续覆盖至画面总时长");
    }
    let expected_positions = (1..=u32::try_from(plan.narrative_units.len()).map_err(|_| {
        ChapterVisualPlanValidationError {
            path: "narrative_units".into(),
            message: "数量超出 u32 范围".into(),
        }
    })?)
        .collect::<BTreeSet<_>>();
    if timeline_positions != expected_positions {
        return invalid("narrative_units[].timeline_position", "必须从 1 连续覆盖");
    }
    if sequence_indices != expected_positions {
        return invalid("narrative_units[].sequence_index", "必须从 1 连续覆盖");
    }
    validate_intercut_groups(plan)?;
    validate_transitions(plan, &shot_ids)?;
    validate_dispositions(plan, &unit_ids, &shot_ids)?;
    Ok(())
}

struct ShotValidationContext<'a> {
    frame_duration_ms: u64,
    entity_ids: &'a BTreeSet<String>,
    addition_ids: &'a BTreeSet<String>,
    adapted_source_span_ids: &'a BTreeSet<String>,
    shot_ids: &'a mut BTreeSet<String>,
    keyframe_ids: &'a mut BTreeSet<String>,
    event_ids: &'a mut BTreeSet<String>,
}

fn validate_shots(
    unit: &VisualNarrativeUnitV1,
    context: &mut ShotValidationContext<'_>,
) -> Result<(), ChapterVisualPlanValidationError> {
    let path = format!("narrative_units[id={}]", unit.id);
    let mut cursor = unit.start_ms;
    for (index, shot) in unit.shots.iter().enumerate() {
        let shot_path = format!("{path}.shots[{index}]");
        unique_text(&shot.id, &format!("{shot_path}.id"), context.shot_ids)?;
        required_text(&shot.title, &format!("{shot_path}.title"))?;
        required_text(
            &shot.narrative_purpose,
            &format!("{shot_path}.narrative_purpose"),
        )?;
        validate_range(
            shot.start_ms,
            shot.end_ms,
            context.frame_duration_ms,
            &format!("{shot_path}.time"),
        )?;
        if shot.start_ms != cursor {
            return invalid(
                format!("{shot_path}.start_ms"),
                "镜头必须连续覆盖所在叙事单元",
            );
        }
        cursor = shot.end_ms;
        require_known_ids(
            &shot.source_span_ids,
            context.adapted_source_span_ids,
            &format!("{shot_path}.source_span_ids"),
        )?;
        if !is_subset(&shot.source_span_ids, &unit.source_span_ids) {
            return invalid(
                format!("{shot_path}.source_span_ids"),
                "必须属于叙事单元来源范围",
            );
        }
        require_known_ids(
            &shot.user_addition_ids,
            context.addition_ids,
            &format!("{shot_path}.user_addition_ids"),
        )?;
        require_provenance(&shot.source_span_ids, &shot.user_addition_ids, &shot_path)?;
        if shot.keyframes.len() < 2 {
            return invalid(
                format!("{shot_path}.keyframes"),
                "每个镜头至少需要首尾两个关键帧",
            );
        }
        let mut keyframe_cursor = shot.start_ms;
        for (keyframe_index, keyframe) in shot.keyframes.iter().enumerate() {
            let keyframe_path = format!("{shot_path}.keyframes[{keyframe_index}]");
            unique_text(
                &keyframe.id,
                &format!("{keyframe_path}.id"),
                context.keyframe_ids,
            )?;
            required_text(&keyframe.label, &format!("{keyframe_path}.label"))?;
            if keyframe.at_ms < shot.start_ms || keyframe.at_ms > shot.end_ms {
                return invalid(format!("{keyframe_path}.at_ms"), "必须位于镜头时间范围内");
            }
            if keyframe.at_ms < keyframe_cursor {
                return invalid(format!("{keyframe_path}.at_ms"), "关键帧必须按时间升序");
            }
            keyframe_cursor = keyframe.at_ms;
            let mut keyframe_entity_ids = BTreeSet::new();
            for state in &keyframe.entity_states {
                require_known_id(
                    &state.entity_id,
                    context.entity_ids,
                    &format!("{keyframe_path}.entity_states[].entity_id"),
                )?;
                if !keyframe_entity_ids.insert(state.entity_id.clone()) {
                    return invalid(
                        format!("{keyframe_path}.entity_states[].entity_id"),
                        "同一关键帧不能重复定义实体状态",
                    );
                }
                validate_entity_state(state, context.entity_ids, &keyframe_path)?;
            }
            validate_camera_state(
                &keyframe.camera,
                context.entity_ids,
                &keyframe_entity_ids,
                &keyframe_path,
            )?;
            for relation in &keyframe.relations {
                unique_text(
                    &relation.id,
                    &format!("{keyframe_path}.relations[].id"),
                    context.event_ids,
                )?;
                require_known_id(
                    &relation.source_entity_id,
                    context.entity_ids,
                    &format!("{keyframe_path}.relations[].source_entity_id"),
                )?;
                require_known_id(
                    &relation.target_entity_id,
                    context.entity_ids,
                    &format!("{keyframe_path}.relations[].target_entity_id"),
                )?;
                if relation.source_entity_id == relation.target_entity_id {
                    return invalid(
                        format!("{keyframe_path}.relations[].target_entity_id"),
                        "不能与源实体相同",
                    );
                }
                if !keyframe_entity_ids.contains(&relation.source_entity_id) {
                    return invalid(
                        format!("{keyframe_path}.relations[].source_entity_id"),
                        "必须在当前关键帧定义实体状态",
                    );
                }
                if !keyframe_entity_ids.contains(&relation.target_entity_id) {
                    return invalid(
                        format!("{keyframe_path}.relations[].target_entity_id"),
                        "必须在当前关键帧定义实体状态",
                    );
                }
                if let Some(label) = relation.label.as_deref() {
                    required_text(label, &format!("{keyframe_path}.relations[].label"))?;
                } else if relation.kind == VisualRelationKindV1::Custom {
                    return invalid(
                        format!("{keyframe_path}.relations[].label"),
                        "自定义关系必须填写说明",
                    );
                }
            }
        }
        if unit.shots[index]
            .keyframes
            .first()
            .is_none_or(|keyframe| keyframe.at_ms != shot.start_ms)
            || unit.shots[index]
                .keyframes
                .last()
                .is_none_or(|keyframe| keyframe.at_ms != shot.end_ms)
        {
            return invalid(
                format!("{shot_path}.keyframes"),
                "首个和末个关键帧必须锁定镜头边界",
            );
        }
        validate_events(shot, context, &shot_path)?;
    }
    if cursor != unit.end_ms {
        return invalid(format!("{path}.shots"), "镜头必须覆盖叙事单元完整时长");
    }
    Ok(())
}

fn validate_events(
    shot: &VisualShotPlanV1,
    context: &mut ShotValidationContext<'_>,
    path: &str,
) -> Result<(), ChapterVisualPlanValidationError> {
    for (index, event) in shot.entity_events.iter().enumerate() {
        let event_path = format!("{path}.entity_events[{index}]");
        validate_event_identity(&event.id, &event_path, context.event_ids)?;
        validate_event_range(
            event.start_ms,
            event.end_ms,
            shot,
            &format!("{event_path}.time"),
        )?;
        require_known_id(
            &event.entity_id,
            context.entity_ids,
            &format!("{event_path}.entity_id"),
        )?;
        if let Some(target) = event.target_entity_id.as_deref() {
            require_known_id(
                target,
                context.entity_ids,
                &format!("{event_path}.target_entity_id"),
            )?;
        }
        required_text(&event.description, &format!("{event_path}.description"))?;
        validate_event_provenance(
            &event.source_span_ids,
            &event.user_addition_ids,
            context.adapted_source_span_ids,
            context.addition_ids,
            &shot.source_span_ids,
            &event_path,
        )?;
    }
    for (index, event) in shot.camera_events.iter().enumerate() {
        let event_path = format!("{path}.camera_events[{index}]");
        validate_event_identity(&event.id, &event_path, context.event_ids)?;
        validate_event_range(
            event.start_ms,
            event.end_ms,
            shot,
            &format!("{event_path}.time"),
        )?;
        if let Some(target) = event.target_entity_id.as_deref() {
            require_known_id(
                target,
                context.entity_ids,
                &format!("{event_path}.target_entity_id"),
            )?;
        }
        required_text(&event.description, &format!("{event_path}.description"))?;
        validate_event_provenance(
            &event.source_span_ids,
            &event.user_addition_ids,
            context.adapted_source_span_ids,
            context.addition_ids,
            &shot.source_span_ids,
            &event_path,
        )?;
    }
    for (index, event) in shot.sound_events.iter().enumerate() {
        let event_path = format!("{path}.sound_events[{index}]");
        validate_event_identity(&event.id, &event_path, context.event_ids)?;
        validate_event_range(
            event.start_ms,
            event.end_ms,
            shot,
            &format!("{event_path}.time"),
        )?;
        if let Some(speaker) = event.speaker_entity_id.as_deref() {
            require_known_id(
                speaker,
                context.entity_ids,
                &format!("{event_path}.speaker_entity_id"),
            )?;
        }
        required_text(&event.content, &format!("{event_path}.content"))?;
        validate_event_provenance(
            &event.source_span_ids,
            &event.user_addition_ids,
            context.adapted_source_span_ids,
            context.addition_ids,
            &shot.source_span_ids,
            &event_path,
        )?;
    }
    for (index, event) in shot.technique_events.iter().enumerate() {
        let event_path = format!("{path}.technique_events[{index}]");
        validate_event_identity(&event.id, &event_path, context.event_ids)?;
        validate_event_range(
            event.start_ms,
            event.end_ms,
            shot,
            &format!("{event_path}.time"),
        )?;
        required_text(&event.description, &format!("{event_path}.description"))?;
        validate_event_provenance(
            &event.source_span_ids,
            &event.user_addition_ids,
            context.adapted_source_span_ids,
            context.addition_ids,
            &shot.source_span_ids,
            &event_path,
        )?;
    }
    Ok(())
}

fn validate_transitions(
    plan: &ChapterVisualPlanV1,
    shot_ids: &BTreeSet<String>,
) -> Result<(), ChapterVisualPlanValidationError> {
    let mut transition_ids = BTreeSet::new();
    let mut transition_pairs = BTreeSet::new();
    let shots = plan
        .narrative_units
        .iter()
        .flat_map(|unit| unit.shots.iter().map(move |shot| (unit, shot)))
        .collect::<Vec<_>>();
    let shot_positions = shots
        .iter()
        .enumerate()
        .map(|(index, (_, shot))| (shot.id.as_str(), index))
        .collect::<std::collections::BTreeMap<_, _>>();
    for (index, transition) in plan.transitions.iter().enumerate() {
        let path = format!("transitions[{index}]");
        unique_text(&transition.id, &format!("{path}.id"), &mut transition_ids)?;
        if !transition_pairs.insert((
            transition.from_shot_id.as_str(),
            transition.to_shot_id.as_str(),
        )) {
            return invalid(path, "同一镜头对只能定义一个转场");
        }
        require_known_id(
            &transition.from_shot_id,
            shot_ids,
            &format!("{path}.from_shot_id"),
        )?;
        require_known_id(
            &transition.to_shot_id,
            shot_ids,
            &format!("{path}.to_shot_id"),
        )?;
        if transition.from_shot_id == transition.to_shot_id {
            return invalid(format!("{path}.to_shot_id"), "不能连接同一个镜头");
        }
        let from_index = shot_positions[transition.from_shot_id.as_str()];
        let to_index = shot_positions[transition.to_shot_id.as_str()];
        if to_index != from_index + 1 {
            return invalid(format!("{path}.to_shot_id"), "转场只能连接时间线相邻镜头");
        }
        let (_, from_shot) = shots[from_index];
        let (_, to_shot) = shots[to_index];
        if transition.at_ms != from_shot.end_ms || transition.at_ms != to_shot.start_ms {
            return invalid(
                format!("{path}.at_ms"),
                "转场切点必须等于前镜头结束和后镜头开始",
            );
        }
        if transition.at_ms > plan.frame.duration_ms
            || transition.duration_ms > plan.frame.duration_ms - transition.at_ms
        {
            return invalid(format!("{path}.at_ms"), "转场不能超出画面总时长");
        }
        if transition.kind == ShootingTransitionTypeV1::Continuous && transition.duration_ms != 0 {
            return invalid(
                format!("{path}.duration_ms"),
                "continuous 转场不应有额外时长",
            );
        }
        if transition.duration_ms > to_shot.end_ms - to_shot.start_ms {
            return invalid(
                format!("{path}.duration_ms"),
                "转场时长不能超过目标镜头时长",
            );
        }
    }
    Ok(())
}

fn validate_dispositions(
    plan: &ChapterVisualPlanV1,
    unit_ids: &BTreeSet<String>,
    shot_ids: &BTreeSet<String>,
) -> Result<(), ChapterVisualPlanValidationError> {
    let dispositions_by_source_span = plan
        .source_dispositions
        .iter()
        .map(|disposition| (disposition.source_span_id.as_str(), disposition))
        .collect::<HashMap<_, _>>();
    let mut units_by_id = HashMap::with_capacity(plan.narrative_units.len());
    let mut shots_by_id = HashMap::new();
    let mut shot_units = HashMap::new();
    for unit in &plan.narrative_units {
        units_by_id.insert(unit.id.as_str(), unit);
        for source_span_id in &unit.source_span_ids {
            let Some(disposition) = dispositions_by_source_span.get(source_span_id.as_str()) else {
                return invalid("source_dispositions", "叙事单元引用了未登记的 source span");
            };
            if disposition.disposition != SourceSpanDispositionKindV1::Adapt
                || disposition.narrative_unit_id.as_deref() != Some(unit.id.as_str())
            {
                return invalid(
                    "source_dispositions",
                    "source span 与叙事单元的引用必须双向一致",
                );
            }
        }
        for shot in &unit.shots {
            shots_by_id.insert(shot.id.as_str(), shot);
            shot_units.insert(shot.id.as_str(), unit.id.as_str());
        }
    }
    for (index, disposition) in plan.source_dispositions.iter().enumerate() {
        let path = format!("source_dispositions[{index}]");
        match disposition.disposition {
            SourceSpanDispositionKindV1::Omit => {}
            SourceSpanDispositionKindV1::Adapt => {
                let Some(unit_id) = disposition.narrative_unit_id.as_deref() else {
                    continue;
                };
                require_known_id(unit_id, unit_ids, &format!("{path}.narrative_unit_id"))?;
                for shot_id in &disposition.shot_ids {
                    if shot_units.get(shot_id.as_str()).copied() != Some(unit_id) {
                        return invalid(format!("{path}.shot_ids"), "绑定镜头必须属于指定叙事单元");
                    }
                }
            }
        }
        require_known_ids(&disposition.shot_ids, shot_ids, &format!("{path}.shot_ids"))?;
    }
    for unit in &plan.narrative_units {
        for shot in &unit.shots {
            for source_span_id in &shot.source_span_ids {
                let Some(disposition) = dispositions_by_source_span.get(source_span_id.as_str())
                else {
                    continue;
                };
                if disposition.disposition != SourceSpanDispositionKindV1::Adapt
                    || disposition.narrative_unit_id.as_deref() != Some(unit.id.as_str())
                    || !disposition.shot_ids.iter().any(|id| id == &shot.id)
                {
                    return invalid(
                        "source_dispositions",
                        "source span 与叙事单元/镜头的引用必须双向一致",
                    );
                }
            }
        }
    }
    for disposition in &plan.source_dispositions {
        if disposition.disposition != SourceSpanDispositionKindV1::Adapt {
            continue;
        }
        let Some(unit_id) = disposition.narrative_unit_id.as_deref() else {
            return invalid("source_dispositions", "adapt source span 缺少叙事单元");
        };
        let Some(unit) = units_by_id.get(unit_id).copied() else {
            return invalid(
                "source_dispositions",
                "adapt source span 引用了不存在的叙事单元",
            );
        };
        if !unit
            .source_span_ids
            .iter()
            .any(|source_span_id| source_span_id == &disposition.source_span_id)
        {
            return invalid(
                "source_dispositions",
                "adapt source span 必须出现在绑定叙事单元的 source_span_ids 中",
            );
        }
        for shot_id in &disposition.shot_ids {
            if shot_units.get(shot_id.as_str()).copied() != Some(unit_id) {
                return invalid(
                    "source_dispositions",
                    "adapt source span 引用了不属于叙事单元的镜头",
                );
            }
            let Some(shot) = shots_by_id.get(shot_id.as_str()).copied() else {
                return invalid(
                    "source_dispositions",
                    "adapt source span 引用了不属于叙事单元的镜头",
                );
            };
            if !shot
                .source_span_ids
                .iter()
                .any(|source_span_id| source_span_id == &disposition.source_span_id)
            {
                return invalid(
                    "source_dispositions",
                    "adapt source span 必须出现在所有绑定镜头的 source_span_ids 中",
                );
            }
        }
    }
    Ok(())
}

fn validate_disposition_shapes(
    plan: &ChapterVisualPlanV1,
) -> Result<(), ChapterVisualPlanValidationError> {
    let mut seen = BTreeSet::new();
    for (index, disposition) in plan.source_dispositions.iter().enumerate() {
        let path = format!("source_dispositions[{index}]");
        unique_text(
            &disposition.source_span_id,
            &format!("{path}.source_span_id"),
            &mut seen,
        )?;
        match disposition.disposition {
            SourceSpanDispositionKindV1::Omit => {
                if disposition
                    .reason
                    .as_deref()
                    .is_none_or(|reason| reason.trim().is_empty())
                {
                    return invalid(format!("{path}.reason"), "omit 必须说明原因");
                }
                if disposition.narrative_unit_id.is_some() || !disposition.shot_ids.is_empty() {
                    return invalid(
                        format!("{path}.narrative_unit_id"),
                        "omit 不能绑定叙事单元或镜头",
                    );
                }
            }
            SourceSpanDispositionKindV1::Adapt => {
                if disposition.narrative_unit_id.is_none() {
                    return invalid(
                        format!("{path}.narrative_unit_id"),
                        "adapt 必须绑定叙事单元",
                    );
                }
                if disposition.shot_ids.is_empty() {
                    return invalid(format!("{path}.shot_ids"), "adapt 必须落到至少一个镜头");
                }
            }
        }
    }
    Ok(())
}

fn validate_intercut_group_shape(
    unit: &VisualNarrativeUnitV1,
    path: &str,
) -> Result<(), ChapterVisualPlanValidationError> {
    let requires_group = matches!(
        unit.editing_mode,
        crate::shooting_script::ShootingEditingModeV1::Intercut
            | crate::shooting_script::ShootingEditingModeV1::ParallelAction
    );
    match unit.intercut_group_id.as_deref() {
        Some(group_id) if group_id.trim().is_empty() => {
            invalid(format!("{path}.intercut_group_id"), "不能为空")
        }
        None if requires_group => invalid(
            format!("{path}.intercut_group_id"),
            "intercut 或 parallel_action 必须设置交叉剪辑组",
        ),
        Some(_) if !requires_group => invalid(
            format!("{path}.intercut_group_id"),
            "只有 intercut 或 parallel_action 可以设置交叉剪辑组",
        ),
        _ => Ok(()),
    }
}

fn validate_intercut_groups(
    plan: &ChapterVisualPlanV1,
) -> Result<(), ChapterVisualPlanValidationError> {
    let mut groups = BTreeMap::<&str, Vec<(usize, &VisualNarrativeUnitV1)>>::new();
    for (index, unit) in plan.narrative_units.iter().enumerate() {
        if let Some(group_id) = unit.intercut_group_id.as_deref() {
            groups.entry(group_id).or_default().push((index, unit));
        }
    }
    for (group_id, units) in groups {
        if units.len() < 2 {
            return invalid(
                format!("narrative_units[{}].intercut_group_id", units[0].0),
                format!("交叉剪辑组 {group_id} 至少需要两个叙事单元"),
            );
        }
        let mode = units[0].1.editing_mode;
        if units.iter().any(|(_, unit)| unit.editing_mode != mode) {
            return invalid(
                format!("narrative_units[{}].editing_mode", units[1].0),
                format!("交叉剪辑组 {group_id} 内的 editing_mode 必须一致"),
            );
        }
    }
    Ok(())
}

fn validate_camera_state(
    camera: &VisualCameraStateV1,
    entity_ids: &BTreeSet<String>,
    keyframe_entity_ids: &BTreeSet<String>,
    path: &str,
) -> Result<(), ChapterVisualPlanValidationError> {
    if !camera.center_x.is_finite()
        || !camera.center_y.is_finite()
        || !camera.rotation_degrees.is_finite()
        || !camera.zoom.is_finite()
        || !(0.0..=1.0).contains(&camera.center_x)
        || !(0.0..=1.0).contains(&camera.center_y)
        || camera.zoom <= 0.0
    {
        return invalid(
            format!("{path}.camera"),
            "相机数值必须有限，中心点范围为 0..=1，zoom 必须大于 0",
        );
    }
    if let Some(focus) = camera.focus_entity_id.as_deref() {
        require_known_id(focus, entity_ids, &format!("{path}.camera.focus_entity_id"))?;
        if !keyframe_entity_ids.contains(focus) {
            return invalid(
                format!("{path}.camera.focus_entity_id"),
                "焦点实体必须在当前关键帧中定义状态",
            );
        }
    }
    Ok(())
}

fn validate_entity_state(
    state: &VisualEntityStateV1,
    entity_ids: &BTreeSet<String>,
    path: &str,
) -> Result<(), ChapterVisualPlanValidationError> {
    let bounds = &state.bounds;
    if [
        bounds.x,
        bounds.y,
        bounds.width,
        bounds.height,
        state.rotation_degrees,
        state.opacity,
    ]
    .into_iter()
    .any(|value| !value.is_finite())
        || bounds.x < 0.0
        || bounds.y < 0.0
        || bounds.width <= 0.0
        || bounds.height <= 0.0
        || bounds.x + bounds.width > 1.0
        || bounds.y + bounds.height > 1.0
        || !(0.0..=1.0).contains(&state.opacity)
    {
        return invalid(
            format!("{path}.entity_states"),
            "位置、尺寸、旋转和透明度必须是有限值，透明度范围为 0..=1",
        );
    }
    if let Some(target) = state.gaze_target_entity_id.as_deref() {
        require_known_id(
            target,
            entity_ids,
            &format!("{path}.entity_states[].gaze_target_entity_id"),
        )?;
    }
    Ok(())
}

fn validate_range(
    start: u64,
    end: u64,
    duration: u64,
    path: &str,
) -> Result<(), ChapterVisualPlanValidationError> {
    if start >= end || end > duration {
        return invalid(path, "必须满足 start < end 且不超过总时长");
    }
    Ok(())
}

fn validate_event_range(
    start: u64,
    end: u64,
    shot: &VisualShotPlanV1,
    path: &str,
) -> Result<(), ChapterVisualPlanValidationError> {
    if start < shot.start_ms || start >= end || end > shot.end_ms {
        return invalid(path, "必须位于镜头时间范围内且满足 start < end");
    }
    Ok(())
}

fn validate_event_identity(
    id: &str,
    path: &str,
    ids: &mut BTreeSet<String>,
) -> Result<(), ChapterVisualPlanValidationError> {
    unique_text(id, &format!("{path}.id"), ids)
}

fn required_text(value: &str, path: &str) -> Result<(), ChapterVisualPlanValidationError> {
    if value.trim().is_empty() {
        invalid(path, "不能为空")
    } else {
        Ok(())
    }
}

fn require_provenance(
    source_span_ids: &[String],
    user_addition_ids: &[String],
    path: &str,
) -> Result<(), ChapterVisualPlanValidationError> {
    if source_span_ids.is_empty() && user_addition_ids.is_empty() {
        invalid(path, "必须引用原文来源或用户新增事实")
    } else {
        Ok(())
    }
}

fn validate_event_provenance(
    source_span_ids: &[String],
    user_addition_ids: &[String],
    adapted_source_span_ids: &BTreeSet<String>,
    addition_ids: &BTreeSet<String>,
    shot_source_span_ids: &[String],
    path: &str,
) -> Result<(), ChapterVisualPlanValidationError> {
    require_known_ids(
        source_span_ids,
        adapted_source_span_ids,
        &format!("{path}.source_span_ids"),
    )?;
    if !is_subset(source_span_ids, shot_source_span_ids) {
        return invalid(
            format!("{path}.source_span_ids"),
            "必须属于所在镜头的来源范围",
        );
    }
    require_known_ids(
        user_addition_ids,
        addition_ids,
        &format!("{path}.user_addition_ids"),
    )?;
    require_provenance(source_span_ids, user_addition_ids, path)
}

fn unique_text(
    value: &str,
    path: &str,
    ids: &mut BTreeSet<String>,
) -> Result<(), ChapterVisualPlanValidationError> {
    required_text(value, path)?;
    if !ids.insert(value.to_string()) {
        invalid(path, "必须唯一")
    } else {
        Ok(())
    }
}

fn require_positive_index(value: u32, path: &str) -> Result<(), ChapterVisualPlanValidationError> {
    if value == 0 {
        invalid(path, "必须大于 0")
    } else {
        Ok(())
    }
}

fn require_known_id(
    value: &str,
    ids: &BTreeSet<String>,
    path: &str,
) -> Result<(), ChapterVisualPlanValidationError> {
    if ids.contains(value) {
        Ok(())
    } else {
        invalid(path, "引用了不存在的 ID")
    }
}

fn require_known_ids(
    values: &[String],
    ids: &BTreeSet<String>,
    path: &str,
) -> Result<(), ChapterVisualPlanValidationError> {
    let mut seen = BTreeSet::new();
    for value in values {
        required_text(value, path)?;
        if !seen.insert(value) {
            return invalid(path, "不能重复引用同一个 ID");
        }
        require_known_id(value, ids, path)?;
    }
    Ok(())
}

fn is_subset(values: &[String], superset: &[String]) -> bool {
    values.iter().all(|value| superset.contains(value))
}

fn invalid(
    path: impl Into<String>,
    message: impl Into<String>,
) -> Result<(), ChapterVisualPlanValidationError> {
    Err(ChapterVisualPlanValidationError {
        path: path.into(),
        message: message.into(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::shooting_script::{
        ShootingCoverageV1, ShootingDirectingStatementV1, ShootingEditingModeV1,
    };

    fn statement() -> ShootingDirectingStatementV1 {
        ShootingDirectingStatementV1 {
            central_dramatic_question: "谁先说真话".into(),
            protagonist_throughline: "试探".into(),
            audience_experience: "紧张".into(),
            viewpoint_strategy: "主观".into(),
            visual_strategy: "冷色".into(),
            spatial_strategy: "保持轴线".into(),
            sound_strategy: "留白".into(),
            rhythm_strategy: "渐快".into(),
        }
    }

    fn plan() -> ChapterVisualPlanV1 {
        let entity = |id: &str| VisualEntityV1 {
            id: id.into(),
            label: id.into(),
            kind: VisualEntityKindV1::Character,
            description: id.into(),
            source_span_ids: vec!["ch1:s1:f1".into()],
            user_addition_ids: vec![],
        };
        let state = |id: &str| VisualEntityStateV1 {
            entity_id: id.into(),
            visible: true,
            bounds: VisualRectV1 {
                x: 0.1,
                y: 0.1,
                width: 0.2,
                height: 0.4,
            },
            rotation_degrees: 0.0,
            opacity: 1.0,
            z_index: 0,
            facing: VisualFacingV1::Front,
            expression: None,
            pose: None,
            action: None,
            gaze_target_entity_id: None,
            dialogue: None,
            notes: None,
        };
        let keyframe = |id: &str, at_ms: u64| VisualKeyframeV1 {
            id: id.into(),
            at_ms,
            label: id.into(),
            interpolation_to_next: VisualInterpolationV1::Hold,
            camera: VisualCameraStateV1 {
                center_x: 0.5,
                center_y: 0.5,
                zoom: 1.0,
                rotation_degrees: 0.0,
                focus_entity_id: Some("a".into()),
                framing_note: None,
            },
            entity_states: vec![state("a"), state("b")],
            relations: vec![],
        };
        ChapterVisualPlanV1 {
            schema_version: ChapterVisualPlanSchemaVersion::V1,
            frame: VisualFrameV1 {
                width: 1920,
                height: 1080,
                duration_ms: 2_000,
                background: None,
            },
            directing_statement: statement(),
            entities: vec![entity("a"), entity("b")],
            user_additions: vec![],
            narrative_units: vec![VisualNarrativeUnitV1 {
                id: "u1".into(),
                sequence_index: 1,
                timeline_position: 1,
                title: "对峙".into(),
                start_ms: 0,
                end_ms: 2_000,
                editing_mode: ShootingEditingModeV1::Continuity,
                intercut_group_id: None,
                narrative_purpose: "试探".into(),
                location: None,
                time_of_day: None,
                participant_entity_ids: vec!["a".into(), "b".into()],
                focal_entity_id: Some("a".into()),
                source_span_ids: vec!["ch1:s1:f1".into()],
                user_addition_ids: vec![],
                shots: vec![VisualShotPlanV1 {
                    id: "s1".into(),
                    title: "全景".into(),
                    start_ms: 0,
                    end_ms: 2_000,
                    source_span_ids: vec!["ch1:s1:f1".into()],
                    user_addition_ids: vec![],
                    narrative_purpose: "建立关系".into(),
                    coverage: ShootingCoverageV1::MustShow,
                    shot_size: None,
                    camera_angle: None,
                    camera_height: None,
                    lens_intent: None,
                    composition: None,
                    keyframes: vec![keyframe("k1", 0), keyframe("k2", 2_000)],
                    entity_events: vec![],
                    camera_events: vec![],
                    sound_events: vec![],
                    technique_events: vec![],
                    continuity_notes: vec![],
                }],
            }],
            transitions: vec![],
            source_dispositions: vec![SourceSpanDispositionV1 {
                source_span_id: "ch1:s1:f1".into(),
                disposition: SourceSpanDispositionKindV1::Adapt,
                narrative_unit_id: Some("u1".into()),
                shot_ids: vec!["s1".into()],
                reason: None,
            }],
        }
    }

    #[test]
    fn valid_plan_passes() {
        plan().validate().expect("valid plan");
    }

    #[test]
    fn shot_gaps_are_rejected() {
        let mut value = plan();
        value.narrative_units[0].shots[0].start_ms = 1;
        let error = value.validate().expect_err("gap must fail");
        assert!(error.path.contains("start_ms"));
    }

    #[test]
    fn omitted_span_requires_reason() {
        let mut value = plan();
        value.source_dispositions[0].disposition = SourceSpanDispositionKindV1::Omit;
        let error = value.validate().expect_err("omit reason is required");
        assert_eq!(error.path, "source_dispositions[0].reason");
    }

    #[test]
    fn source_span_coverage_must_be_exact() {
        let value = plan();
        let error = value
            .validate_source_span_coverage(["ch1:s1:f1", "ch1:s2:f1"])
            .expect_err("missing span must fail");
        assert!(error.message.contains("ch1:s2:f1"));
    }

    #[test]
    fn narrative_units_must_cover_the_complete_timeline() {
        let mut gap = plan();
        gap.narrative_units[0].start_ms = 1;
        gap.narrative_units[0].shots[0].start_ms = 1;
        gap.narrative_units[0].shots[0].keyframes[0].at_ms = 1;
        let error = gap.validate().expect_err("unit gap must fail");
        assert!(error.path.ends_with("start_ms"));

        let mut missing_tail = plan();
        missing_tail.narrative_units[0].end_ms = 1_900;
        missing_tail.narrative_units[0].shots[0].end_ms = 1_900;
        missing_tail.narrative_units[0].shots[0].keyframes[1].at_ms = 1_900;
        let error = missing_tail
            .validate()
            .expect_err("unit tail gap must fail");
        assert_eq!(error.path, "narrative_units");
    }

    #[test]
    fn entity_bounds_must_have_area_and_stay_in_frame() {
        let mut zero_width = plan();
        zero_width.narrative_units[0].shots[0].keyframes[0].entity_states[0]
            .bounds
            .width = 0.0;
        let error = zero_width.validate().expect_err("zero width must fail");
        assert!(error.path.contains("entity_states"));

        let mut overflow = plan();
        overflow.narrative_units[0].shots[0].keyframes[0].entity_states[0]
            .bounds
            .x = 0.9;
        let error = overflow.validate().expect_err("overflow must fail");
        assert!(error.path.contains("entity_states"));
    }

    #[test]
    fn duplicate_entity_state_in_one_keyframe_is_rejected() {
        let mut value = plan();
        let duplicate = value.narrative_units[0].shots[0].keyframes[0].entity_states[0].clone();
        value.narrative_units[0].shots[0].keyframes[0]
            .entity_states
            .push(duplicate);
        let error = value.validate().expect_err("duplicate state must fail");
        assert!(error.message.contains("重复定义实体状态"));
    }

    #[test]
    fn relations_require_distinct_endpoints_in_the_current_keyframe() {
        let relation = |target: &str| VisualRelationV1 {
            id: "r1".into(),
            kind: VisualRelationKindV1::LookingAt,
            source_entity_id: "a".into(),
            target_entity_id: target.into(),
            label: None,
        };

        let mut missing_state = plan();
        missing_state.narrative_units[0].shots[0].keyframes[0]
            .entity_states
            .retain(|state| state.entity_id != "b");
        missing_state.narrative_units[0].shots[0].keyframes[0]
            .relations
            .push(relation("b"));
        let error = missing_state
            .validate()
            .expect_err("relation endpoint must exist in the current keyframe");
        assert!(error.path.ends_with("target_entity_id"));
        assert!(error.message.contains("当前关键帧"));

        let mut self_relation = plan();
        self_relation.narrative_units[0].shots[0].keyframes[0]
            .relations
            .push(relation("a"));
        let error = self_relation
            .validate()
            .expect_err("a relation cannot point to itself");
        assert!(error.message.contains("源实体相同"));
    }

    #[test]
    fn custom_relations_require_a_non_empty_label() {
        let mut value = plan();
        value.narrative_units[0].shots[0].keyframes[0]
            .relations
            .push(VisualRelationV1 {
                id: "r1".into(),
                kind: VisualRelationKindV1::Custom,
                source_entity_id: "a".into(),
                target_entity_id: "b".into(),
                label: None,
            });
        let error = value
            .validate()
            .expect_err("custom relation must explain its meaning");
        assert!(error.path.ends_with("label"));
        assert!(error.message.contains("必须填写说明"));
    }

    #[test]
    fn omitted_source_span_cannot_keep_references() {
        let mut value = plan();
        value.source_dispositions[0].disposition = SourceSpanDispositionKindV1::Omit;
        value.source_dispositions[0].reason = Some("不影响主线".into());
        let error = value
            .validate()
            .expect_err("omitted source must not remain referenced");
        assert!(error.path.contains("narrative_unit_id"));
    }

    #[test]
    fn entities_cannot_reference_omitted_source_spans() {
        let mut value = plan();
        value.source_dispositions[0].disposition = SourceSpanDispositionKindV1::Omit;
        value.source_dispositions[0].reason = Some("不影响主线".into());
        value.source_dispositions[0].narrative_unit_id = None;
        value.source_dispositions[0].shot_ids.clear();
        let error = value.validate().expect_err("entity source must be adapted");
        assert!(error.path.contains("entities[0].source_span_ids"));
    }

    #[test]
    fn source_disposition_references_must_be_bidirectional() {
        let mut unit_mismatch = plan();
        unit_mismatch.source_dispositions[0].narrative_unit_id = Some("other".into());
        let error = unit_mismatch
            .validate()
            .expect_err("unit identity must be bidirectional");
        assert!(error.message.contains("双向一致"));

        let mut shot_mismatch = plan();
        shot_mismatch.source_dispositions[0].shot_ids = vec!["missing".into()];
        let error = shot_mismatch
            .validate()
            .expect_err("shot identity must belong to the disposition unit");
        assert!(error.path.contains("shot_ids"));
    }

    #[test]
    fn transitions_require_adjacent_shots_exact_cut_and_bounded_duration() {
        let mut value = plan();
        let mut second = value.narrative_units[0].shots[0].clone();
        second.id = "s2".into();
        second.title = "近景".into();
        second.start_ms = 1_000;
        second.keyframes[0].id = "k3".into();
        second.keyframes[0].at_ms = 1_000;
        second.keyframes[1].id = "k4".into();
        value.narrative_units[0].shots[0].end_ms = 1_000;
        value.narrative_units[0].shots[0].keyframes[1].at_ms = 1_000;
        value.narrative_units[0].shots.push(second);
        value.source_dispositions[0].shot_ids.push("s2".into());

        value.transitions.push(VisualTransitionV1 {
            id: "t1".into(),
            from_shot_id: "s2".into(),
            to_shot_id: "s1".into(),
            at_ms: 2_000,
            duration_ms: 0,
            kind: ShootingTransitionTypeV1::Continuous,
            preserved_action: None,
            preserved_sound: None,
            notes: None,
        });
        let error = value.validate().expect_err("reverse transition must fail");
        assert!(error.message.contains("相邻镜头"));

        value.transitions[0].from_shot_id = "s1".into();
        value.transitions[0].to_shot_id = "s2".into();
        value.transitions[0].at_ms = 999;
        let error = value.validate().expect_err("wrong cut point must fail");
        assert!(error.message.contains("切点"));

        value.transitions[0].at_ms = 1_000;
        value.transitions[0].kind = ShootingTransitionTypeV1::Dissolve;
        value.transitions[0].duration_ms = 1_001;
        let error = value.validate().expect_err("long transition must fail");
        assert!(error.message.contains("画面总时长"));
    }

    #[test]
    fn reference_arrays_reject_empty_and_duplicate_ids() {
        let mut empty = plan();
        empty.narrative_units[0]
            .participant_entity_ids
            .push("".into());
        let error = empty.validate().expect_err("empty reference must fail");
        assert_eq!(error.path, "narrative_units[0].participant_entity_ids");
        assert_eq!(error.message, "不能为空");

        let mut duplicate = plan();
        duplicate.narrative_units[0]
            .participant_entity_ids
            .push("a".into());
        let error = duplicate
            .validate()
            .expect_err("duplicate reference must fail");
        assert_eq!(error.path, "narrative_units[0].participant_entity_ids");
        assert!(error.message.contains("重复引用"));
    }

    #[test]
    fn camera_center_and_focus_must_describe_the_current_keyframe() {
        let mut center = plan();
        center.narrative_units[0].shots[0].keyframes[0]
            .camera
            .center_x = 1.1;
        let error = center
            .validate()
            .expect_err("camera center must stay in frame");
        assert!(error.path.ends_with("camera"));
        assert!(error.message.contains("中心点"));

        let mut focus = plan();
        focus.narrative_units[0].shots[0].keyframes[0]
            .entity_states
            .retain(|state| state.entity_id != "a");
        let error = focus
            .validate()
            .expect_err("focus must have state in keyframe");
        assert!(error.path.ends_with("camera.focus_entity_id"));
        assert!(error.message.contains("当前关键帧"));
    }

    #[test]
    fn intercut_groups_require_matching_modes_and_multiple_units() {
        let mut single = plan();
        single.narrative_units[0].editing_mode = ShootingEditingModeV1::Intercut;
        single.narrative_units[0].intercut_group_id = Some("g1".into());
        let error = single
            .validate()
            .expect_err("single-unit intercut group must fail");
        assert!(error.message.contains("至少需要两个"));

        let mut forbidden = plan();
        forbidden.narrative_units[0].intercut_group_id = Some("g1".into());
        let error = forbidden
            .validate()
            .expect_err("continuity must not carry a group");
        assert!(error.message.contains("只有 intercut"));

        let mut mixed = plan_with_two_units();
        mixed.narrative_units[0].editing_mode = ShootingEditingModeV1::Intercut;
        mixed.narrative_units[1].editing_mode = ShootingEditingModeV1::ParallelAction;
        for unit in &mut mixed.narrative_units {
            unit.intercut_group_id = Some("g1".into());
        }
        let error = mixed
            .validate()
            .expect_err("mixed editing modes in one group must fail");
        assert!(error.message.contains("editing_mode 必须一致"));
    }

    #[test]
    fn camera_and_technique_events_require_valid_provenance() {
        let mut camera = plan();
        camera.narrative_units[0].shots[0]
            .camera_events
            .push(VisualCameraEventV1 {
                id: "camera-1".into(),
                kind: VisualCameraEventKindV1::PushIn,
                start_ms: 0,
                end_ms: 500,
                target_entity_id: Some("a".into()),
                description: "推近".into(),
                source_span_ids: vec![],
                user_addition_ids: vec![],
                parameters: Default::default(),
            });
        let error = camera
            .validate()
            .expect_err("camera event provenance must be explicit");
        assert!(error.path.contains("camera_events[0]"));
        assert!(error.message.contains("必须引用"));

        let mut technique = plan();
        technique.narrative_units[0].shots[0]
            .technique_events
            .push(VisualTechniqueEventV1 {
                id: "technique-1".into(),
                kind: VisualTechniqueKindV1::Symbolic,
                start_ms: 0,
                end_ms: 500,
                description: "象征".into(),
                source_span_ids: vec!["ch1:s1:f1".into(), "ch1:s1:f1".into()],
                user_addition_ids: vec![],
                parameters: Default::default(),
            });
        let error = technique
            .validate()
            .expect_err("event source references must be unique");
        assert!(error.path.ends_with("source_span_ids"));
        assert!(error.message.contains("重复引用"));
    }

    #[test]
    fn user_addition_must_be_introduced_before_the_timeline_end() {
        let mut value = plan();
        value.user_additions.push(VisualUserAdditionV1 {
            id: "addition-1".into(),
            kind: VisualUserAdditionKindV1::Other,
            description: "追加运镜".into(),
            rationale: "用户要求".into(),
            introduced_at_ms: Some(value.frame.duration_ms),
        });
        let error = value
            .validate()
            .expect_err("timeline endpoint is not an introduction instant");
        assert!(error.path.ends_with("introduced_at_ms"));
    }

    fn plan_with_two_units() -> ChapterVisualPlanV1 {
        let mut value = plan();
        let mut second = value.narrative_units[0].clone();
        second.id = "u2".into();
        second.sequence_index = 2;
        second.timeline_position = 2;
        second.start_ms = 1_000;
        second.shots[0].id = "s2".into();
        second.shots[0].start_ms = 1_000;
        second.shots[0].keyframes[0].id = "k3".into();
        second.shots[0].keyframes[0].at_ms = 1_000;
        second.shots[0].keyframes[1].id = "k4".into();
        value.narrative_units[0].end_ms = 1_000;
        value.narrative_units[0].shots[0].end_ms = 1_000;
        value.narrative_units[0].shots[0].keyframes[1].at_ms = 1_000;
        value.narrative_units.push(second);
        value.source_dispositions[0].narrative_unit_id = Some("u1".into());
        value.source_dispositions[0].shot_ids = vec!["s1".into()];
        value.narrative_units[1].source_span_ids.clear();
        value.narrative_units[1].user_addition_ids = vec!["addition-1".into()];
        value.narrative_units[1].shots[0].source_span_ids.clear();
        value.narrative_units[1].shots[0].user_addition_ids = vec!["addition-1".into()];
        value.user_additions.push(VisualUserAdditionV1 {
            id: "addition-1".into(),
            kind: VisualUserAdditionKindV1::StoryFact,
            description: "第二条行动线".into(),
            rationale: "测试交叉剪辑".into(),
            introduced_at_ms: Some(1_000),
        });
        value
    }
}
