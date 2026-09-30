use std::collections::{BTreeSet, HashMap, HashSet};

use crate::chapter_visual_plan::VisualSegmentProjectionV3;

use super::super::{
    ChapterShootingScriptV2, ShootingScriptValidationError, ShootingShotV2,
    ShootingTransitionContractV1, ShootingVisualPlanSourceV1,
    director_projection::{DirectorScriptPlan, segment_addition_id},
};

impl ShootingShotV2 {
    /// 只重绑定导演锁定的约束和时间，保留作者的拍摄设计与标题。
    pub fn bind_director_constraints(
        &mut self,
        planned: &VisualSegmentProjectionV3,
    ) -> Result<(), ShootingScriptValidationError> {
        planned
            .end_ms
            .checked_sub(planned.start_ms)
            .filter(|duration| *duration > 0)
            .ok_or_else(|| invalid(format!("shots[{}]", self.shot_id), "镜头时间范围无效"))?;
        self.visual_shot_id = Some(planned.segment_id.clone());
        self.planned_start_ms = Some(planned.start_ms);
        self.planned_end_ms = Some(planned.end_ms);
        self.visual_constraints = Some(planned.clone());
        Ok(())
    }

    fn is_current_for(&self, planned: &VisualSegmentProjectionV3) -> bool {
        self.visual_constraints
            .as_ref()
            .is_some_and(|constraints| constraints.same_content(planned))
    }
}

impl ChapterShootingScriptV2 {
    pub fn plan_sync(&self, plan: &DirectorScriptPlan) -> super::ShootingScriptPlanSyncV1 {
        let shots = self
            .scenes
            .iter()
            .flat_map(|scene| &scene.shots)
            .map(|shot| (shot.shot_id.as_str(), shot))
            .collect::<HashMap<_, _>>();
        let mut sync = super::ShootingScriptPlanSyncV1::default();
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
