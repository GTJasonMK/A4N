//! 拍摄剧本与已确认导演编排的逐镜头同步。
//!
//! 修改导演编排的某个镜头只应让该镜头过时：
//! 确认新方案时拍摄剧本被变基，内容未变的镜头原样保留，内容变化的镜头保留旧约束作为
//! 过时标记，随后通过手动确认（重绑定）或模型辅助更新指定镜头。

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::chapter_visual_plan::VisualSegmentProjectionV3;

use super::director_projection::{DirectorScriptPlan, segment_addition_id};
use super::{
    ChapterShootingScriptV1, ShootingScriptValidationError, ShootingShotV1,
    ShootingTransitionContractV1, ShootingVisualPlanSourceV1,
};

/// 拍摄剧本相对当前已确认导演编排的逐镜头同步状态。
#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ShootingScriptPlanSyncV1 {
    /// 导演编排内容已变化的镜头；剧本保留旧内容，等待手动确认或模型辅助更新。
    pub stale_shot_ids: Vec<String>,
    /// 导演编排中存在、剧本尚未覆盖的镜头；需要补全对应拍摄设计。
    pub missing_shot_ids: Vec<String>,
}

impl ShootingScriptPlanSyncV1 {
    pub fn is_current(&self) -> bool {
        self.stale_shot_ids.is_empty() && self.missing_shot_ids.is_empty()
    }
}

impl ShootingShotV1 {
    /// 只重绑定导演锁定的约束和时间，保留作者的拍摄设计与标题。
    /// 表演节拍时长按新镜头时长等比缩放。
    pub fn bind_director_constraints(
        &mut self,
        planned: &VisualSegmentProjectionV3,
    ) -> Result<(), ShootingScriptValidationError> {
        let duration_ms = planned
            .end_ms
            .checked_sub(planned.start_ms)
            .filter(|duration| *duration > 0)
            .ok_or_else(|| invalid(format!("shots[{}]", self.shot_id), "镜头时间范围无效"))?;
        let duration = duration_ms as f64 / 1000.0;
        let seconds = self
            .performance_beats
            .iter()
            .map(|beat| beat.estimated_seconds)
            .sum::<f64>();
        if !self.performance_beats.is_empty() {
            if !seconds.is_finite() || seconds <= 0.0 {
                return Err(invalid(
                    format!("shots[{}].performance_beats", self.shot_id),
                    "表演节拍时长必须为有效正数",
                ));
            }
            for beat in &mut self.performance_beats {
                beat.estimated_seconds *= duration / seconds;
            }
        }
        self.visual_shot_id = Some(planned.segment_id.clone());
        self.planned_start_ms = Some(planned.start_ms);
        self.planned_end_ms = Some(planned.end_ms);
        self.visual_constraints = Some(planned.clone());
        self.estimated_natural_seconds = duration;
        Ok(())
    }

    fn is_current_for(&self, planned: &VisualSegmentProjectionV3) -> bool {
        self.visual_constraints
            .as_ref()
            .is_some_and(|constraints| constraints.same_content(planned))
    }
}

impl ChapterShootingScriptV1 {
    /// 下游只依赖目标镜头、所在场景的共享内容和实际引用的作者补充；版本号与显示顺序不参与。
    pub fn shot_content_fingerprints(
        &self,
        plan: &DirectorScriptPlan,
    ) -> Result<BTreeMap<String, String>, serde_json::Error> {
        let mut fingerprints = BTreeMap::new();
        for scene in &self.scenes {
            let mut shared = scene.clone();
            shared.shots.clear();
            let mut shared = serde_json::to_value(shared)?;
            let object = shared.as_object_mut().expect("scene serializes as object");
            for field in [
                "scene_index",
                "scene_identity",
                "visual_narrative_unit_id",
                "sequence_index",
                "narrative_unit_index",
                "timeline_position",
                "title",
                "shots",
                "source_span_ids",
                "user_addition_ids",
            ] {
                object.remove(field);
            }
            for (value, transition) in object["transition_contracts"]
                .as_array_mut()
                .expect("transitions serialize as array")
                .iter_mut()
                .zip(&scene.transition_contracts)
            {
                // 未来单元尚未生成时也必须有稳定身份；不能由当前已完成场景推断。
                let target = transition
                    .to_narrative_unit_index
                    .checked_sub(1)
                    .and_then(|index| plan.narrative_units.get(index as usize))
                    .ok_or_else(|| {
                        <serde_json::Error as serde::ser::Error>::custom(format!(
                            "转场目标 {} 不在导演编排中",
                            transition.to_narrative_unit_index
                        ))
                    })?;
                value["to_narrative_unit_index"] = serde_json::to_value(&target.id)?;
            }
            for shot in &scene.shots {
                let mut content = shot.clone();
                content.shot_index = 0;
                content.title.clear();
                content.planned_end_ms = content
                    .planned_start_ms
                    .zip(content.planned_end_ms)
                    .map(|(start, end)| end.saturating_sub(start));
                content.planned_start_ms = content.planned_start_ms.map(|_| 0);
                content.visual_constraints = content
                    .visual_constraints
                    .as_ref()
                    .map(VisualSegmentProjectionV3::content_view);
                let additions = self
                    .user_additions
                    .iter()
                    .filter(|addition| {
                        shot.user_addition_ids.contains(&addition.id)
                            || shot
                                .performance_beats
                                .iter()
                                .any(|beat| beat.user_addition_ids.contains(&addition.id))
                            || (!addition.id.starts_with("director:")
                                && scene.user_addition_ids.contains(&addition.id))
                    })
                    .map(|addition| {
                        let mut addition = addition.clone();
                        addition.introduced_at_ms = None;
                        addition
                    })
                    .collect::<Vec<_>>();
                fingerprints.insert(shot.shot_id.clone(), crate::generation::canonical_json_sha256(
                    &serde_json::json!({ "scene": &shared, "shot": content, "user_additions": additions, "directing_statement": self.directing_statement })
                )?);
            }
        }
        Ok(fingerprints)
    }

    pub fn plan_sync(&self, plan: &DirectorScriptPlan) -> ShootingScriptPlanSyncV1 {
        let shots = self
            .scenes
            .iter()
            .flat_map(|scene| &scene.shots)
            .map(|shot| (shot.shot_id.as_str(), shot))
            .collect::<HashMap<_, _>>();
        let mut sync = ShootingScriptPlanSyncV1::default();
        for unit in &plan.narrative_units {
            for planned in &unit.shots {
                match shots.get(planned.segment_id.as_str()) {
                    None => sync.missing_shot_ids.push(planned.segment_id.clone()),
                    Some(shot) if !shot.is_current_for(planned) => {
                        sync.stale_shot_ids.push(planned.segment_id.clone())
                    }
                    Some(_) => continue,
                }
            }
        }
        sync
    }

    /// 按稳定镜头 ID 同步到新方案，场景拆分或合并也保留原有镜头。
    /// 平移只更新计划位置，保留内容未变的约束，避免连带作废已有片段。
    pub fn rebased_onto(
        &self,
        plan: &DirectorScriptPlan,
        source: ShootingVisualPlanSourceV1,
    ) -> Result<Self, ShootingScriptValidationError> {
        let scene_by_shot = self
            .scenes
            .iter()
            .enumerate()
            .flat_map(|(offset, scene)| {
                scene
                    .shots
                    .iter()
                    .map(move |shot| (shot.shot_id.as_str(), offset))
            })
            .collect::<HashMap<_, _>>();
        let carried = plan
            .narrative_units
            .iter()
            .map(|unit| {
                unit.shots
                    .iter()
                    .filter_map(|planned| {
                        let offset = *scene_by_shot.get(planned.segment_id.as_str())?;
                        let shot = self.scenes[offset]
                            .shots
                            .iter()
                            .find(|shot| shot.shot_id == planned.segment_id)?;
                        Some((
                            offset,
                            shot.visual_constraints
                                .as_ref()
                                .is_some_and(|old| old.scene_binding == unit.scene_binding),
                        ))
                    })
                    .max_by_key(|(offset, same_binding)| {
                        (*same_binding, std::cmp::Reverse(*offset))
                    })
                    .map(|(offset, _)| offset)
            })
            .collect::<Vec<_>>();
        let mut unit_indices = HashMap::new();
        for (unit, scene) in carried.iter().enumerate() {
            if let Some(scene) = scene {
                unit_indices
                    .entry(self.scenes[*scene].narrative_unit_index)
                    .or_insert(index(unit)?);
            }
        }

        let mut scenes = Vec::with_capacity(self.scenes.len());
        for (unit_offset, (unit, carried_scene)) in
            plan.narrative_units.iter().zip(&carried).enumerate()
        {
            let Some(old) = carried_scene.map(|scene| &self.scenes[scene]) else {
                continue;
            };
            let unit_index = index(unit_offset)?;
            let mut scene = old.clone();
            scene.shots = unit
                .shots
                .iter()
                .filter_map(|planned| {
                    self.scenes[*scene_by_shot.get(planned.segment_id.as_str())?]
                        .shots
                        .iter()
                        .find(|shot| shot.shot_id == planned.segment_id)
                        .map(|shot| (shot, planned))
                })
                .enumerate()
                .map(|(shot_offset, (shot, planned))| {
                    let mut shot = shot.clone();
                    shot.shot_index = index(shot_offset)?;
                    if shot.is_current_for(planned) {
                        shot.planned_start_ms = Some(planned.start_ms);
                        shot.planned_end_ms = Some(planned.end_ms);
                    }
                    Ok(shot)
                })
                .collect::<Result<Vec<_>, ShootingScriptValidationError>>()?;
            let present = scene
                .shots
                .iter()
                .map(|shot| segment_addition_id(&shot.shot_id))
                .collect::<HashSet<_>>();
            scene
                .user_addition_ids
                .retain(|id| !id.starts_with("director:") || present.contains(id));
            scene.scene_index = index(scenes.len())?;
            scene.timeline_position = scene.scene_index;
            scene.sequence_index = unit.sequence_index;
            scene.narrative_unit_index = unit_index;
            scene.scene_identity = unit.id.clone();
            scene.visual_narrative_unit_id = Some(unit.id.clone());
            scene.source_span_ids = scene
                .shots
                .iter()
                .flat_map(|shot| shot.source_span_ids.iter().cloned())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            scene.transition_contracts = old
                .transition_contracts
                .iter()
                .filter_map(|transition| {
                    let target = *unit_indices.get(&transition.to_narrative_unit_index)?;
                    (target != unit_index).then(|| ShootingTransitionContractV1 {
                        to_narrative_unit_index: target,
                        ..transition.clone()
                    })
                })
                .collect();
            scenes.push(scene);
        }

        let mut user_additions = plan.user_additions.clone();
        let planned_ids = user_additions
            .iter()
            .map(|addition| addition.id.clone())
            .collect::<HashSet<_>>();
        user_additions.extend(
            self.user_additions
                .iter()
                .filter(|addition| {
                    !planned_ids.contains(&addition.id) && !addition.id.starts_with("director:")
                })
                .cloned(),
        );
        Ok(Self {
            schema_version: self.schema_version,
            visual_plan_source: Some(source),
            user_additions,
            directing_statement: self.directing_statement.clone(),
            scenes,
        })
    }

    /// 只采用模型返回的指定镜头，保留其他镜头和已编辑的场景信息。
    pub fn with_updated_shots(
        &self,
        plan: &DirectorScriptPlan,
        generated_scenes: Vec<super::ShootingSceneV1>,
        shot_ids: &[String],
    ) -> Result<Self, ShootingScriptValidationError> {
        plan.validate_shot_ids(shot_ids)
            .map_err(|error| invalid(error.path, error.message))?;
        let selected = shot_ids.iter().collect::<HashSet<_>>();
        let returned = generated_scenes
            .iter()
            .flat_map(|scene| &scene.shots)
            .collect::<Vec<_>>();
        if returned.len() != selected.len()
            || returned
                .iter()
                .map(|shot| &shot.shot_id)
                .collect::<HashSet<_>>()
                != selected
        {
            return Err(invalid("shots", "局部生成结果必须恰好覆盖请求的镜头"));
        }
        let mut result = self.clone();
        result.scenes.retain(|scene| {
            !scene
                .shots
                .iter()
                .all(|shot| selected.contains(&shot.shot_id))
                || !scene.shots.iter().any(|shot| {
                    shot.visual_constraints
                        .as_ref()
                        .zip(plan.shot(&shot.shot_id))
                        .is_some_and(|(old, current)| old.scene_binding != current.scene_binding)
                })
        });
        for scene in &mut result.scenes {
            scene.shots.retain(|shot| !selected.contains(&shot.shot_id));
        }
        for scene in generated_scenes {
            match result
                .scenes
                .iter_mut()
                .find(|old| old.scene_identity == scene.scene_identity)
            {
                Some(old) => {
                    old.shots.extend(scene.shots);
                }
                None => result.scenes.push(scene),
            }
        }
        let source = self
            .visual_plan_source
            .clone()
            .ok_or_else(|| invalid("visual_plan_source", "局部更新需要已绑定导演编排的拍摄剧本"))?;
        result.rebased_onto(plan, source)
    }

    /// 手动更新：用户已按新方案修订这些过时镜头后，把它们重新绑定到当前方案。
    /// 内容本就未变的镜头保持原约束，避免无谓地改写视频片段依赖的约束字节。
    pub fn rebind_shots(
        &mut self,
        plan: &DirectorScriptPlan,
        shot_ids: &[String],
    ) -> Result<(), ShootingScriptValidationError> {
        plan.validate_shot_ids(shot_ids)
            .map_err(|error| invalid(error.path, error.message))?;
        for shot_id in shot_ids {
            let planned = plan.shot(shot_id).ok_or_else(|| {
                invalid(
                    format!("rebind_shot_ids[{shot_id}]"),
                    "镜头不在当前已确认的导演编排中",
                )
            })?;
            let shot = self
                .scenes
                .iter_mut()
                .flat_map(|scene| &mut scene.shots)
                .find(|shot| &shot.shot_id == shot_id)
                .ok_or_else(|| {
                    invalid(
                        format!("rebind_shot_ids[{shot_id}]"),
                        "拍摄剧本中没有该镜头；新增镜头请用模型辅助生成所在场景",
                    )
                })?;
            if !shot.is_current_for(planned) {
                shot.bind_director_constraints(planned)?;
            }
        }
        Ok(())
    }
}

fn index(offset: usize) -> Result<u32, ShootingScriptValidationError> {
    u32::try_from(offset + 1).map_err(|_| invalid("scenes", "场景或镜头数量超出 u32 范围"))
}

fn invalid(path: impl Into<String>, message: impl Into<String>) -> ShootingScriptValidationError {
    ShootingScriptValidationError {
        path: path.into(),
        message: message.into(),
    }
}

#[cfg(test)]
mod tests;
