//! 拍摄剧本与已确认导演编排的逐镜头同步。
//!
//! 修改导演编排的某个镜头只应让该镜头（及其所在叙事单元的重生成需求）过时：
//! 确认新方案时拍摄剧本被变基，内容未变的镜头原样保留，内容变化的镜头保留旧约束作为
//! 过时标记，随后通过手动确认（重绑定）或模型辅助重生成所在场景来更新。

use std::collections::{BTreeSet, HashMap, HashSet};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::chapter_visual_plan::{
    ChapterVisualPlanV3, ChapterVisualPlanValidationError, VisualSegmentProjectionV3,
};

use super::director_projection::{
    DirectorScriptPlan, project_visual_plan_for_shooting_script, segment_addition_id,
};
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
    /// 导演编排中存在、剧本尚未覆盖的镜头；需要模型辅助生成所在场景。
    pub missing_shot_ids: Vec<String>,
    /// 含过时或缺失镜头的叙事单元（即拍摄场景身份），模型辅助更新按此粒度重生成。
    pub stale_unit_ids: Vec<String>,
}

impl ShootingScriptPlanSyncV1 {
    pub fn is_current(&self) -> bool {
        self.stale_shot_ids.is_empty() && self.missing_shot_ids.is_empty()
    }
}

impl ShootingShotV1 {
    /// 把镜头绑定到给定方案镜头：约束、计划时间、标题和叙事目的按方案确定性写入，
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
        self.title = planned.title.clone();
        self.narrative_purpose = planned.prompt.clone();
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
    pub fn plan_sync(&self, plan: &DirectorScriptPlan) -> ShootingScriptPlanSyncV1 {
        let shots = self
            .scenes
            .iter()
            .flat_map(|scene| &scene.shots)
            .map(|shot| (shot.shot_id.as_str(), shot))
            .collect::<HashMap<_, _>>();
        let mut sync = ShootingScriptPlanSyncV1::default();
        for unit in &plan.narrative_units {
            let mut unit_stale = false;
            for planned in &unit.shots {
                match shots.get(planned.segment_id.as_str()) {
                    None => sync.missing_shot_ids.push(planned.segment_id.clone()),
                    Some(shot) if !shot.is_current_for(planned) => {
                        sync.stale_shot_ids.push(planned.segment_id.clone())
                    }
                    Some(_) => continue,
                }
                unit_stale = true;
            }
            if unit_stale {
                sync.stale_unit_ids.push(unit.id.clone());
            }
        }
        sync
    }

    pub fn plan_sync_for(
        &self,
        plan: &ChapterVisualPlanV3,
    ) -> Result<ShootingScriptPlanSyncV1, ChapterVisualPlanValidationError> {
        Ok(self.plan_sync(&project_visual_plan_for_shooting_script(plan, &[])?))
    }

    /// 变基到新确认的导演编排。新叙事单元的镜头全部来自同一个旧场景时沿用该场景（一个旧场景
    /// 只沿用一次）；跨场景重组或全新的单元不保留旧内容，留待模型辅助生成。沿用场景里内容
    /// 未变的镜头只更新计划位置与标题，其 visual_constraints 保持原字节，保证视频片段中的
    /// 导演约束仍然精确匹配。
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
        let mut used = HashSet::new();
        let carried = plan
            .narrative_units
            .iter()
            .map(|unit| {
                let sources = unit
                    .shots
                    .iter()
                    .filter_map(|shot| scene_by_shot.get(shot.segment_id.as_str()).copied())
                    .collect::<BTreeSet<_>>();
                match sources.iter().collect::<Vec<_>>()[..] {
                    [&scene] if used.insert(scene) => Some(scene),
                    _ => None,
                }
            })
            .collect::<Vec<_>>();
        let mut unit_indices = HashMap::new();
        for (unit, scene) in carried.iter().enumerate() {
            if let Some(scene) = scene {
                unit_indices.insert(self.scenes[*scene].narrative_unit_index, index(unit)?);
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
                    old.shots
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
                        shot.title = planned.title.clone();
                    }
                    Ok(shot)
                })
                .collect::<Result<Vec<_>, ShootingScriptValidationError>>()?;
            let present = scene
                .shots
                .iter()
                .map(|shot| segment_addition_id(&shot.shot_id))
                .collect::<HashSet<_>>();
            scene.user_addition_ids.retain(|id| {
                !id.starts_with("director:") || present.contains(id)
            });
            scene.scene_index = index(scenes.len())?;
            scene.timeline_position = scene.scene_index;
            scene.sequence_index = unit.sequence_index;
            scene.narrative_unit_index = unit_index;
            scene.scene_identity = unit.id.clone();
            scene.visual_narrative_unit_id = Some(unit.id.clone());
            scene.title = unit.title.clone();
            scene.editing_mode = unit.editing_mode;
            scene.intercut_group_id = unit.intercut_group_id.clone();
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

        let referenced = scenes
            .iter()
            .flat_map(|scene| {
                scene.user_addition_ids.iter().chain(scene.shots.iter().flat_map(|shot| {
                    shot.user_addition_ids.iter().chain(
                        shot.performance_beats
                            .iter()
                            .flat_map(|beat| &beat.user_addition_ids),
                    )
                }))
            })
            .collect::<HashSet<_>>();
        let mut user_additions = plan.user_additions.clone();
        let planned_ids = user_additions
            .iter()
            .map(|addition| addition.id.clone())
            .collect::<HashSet<_>>();
        user_additions.extend(
            self.user_additions
                .iter()
                .filter(|addition| {
                    !planned_ids.contains(&addition.id) && referenced.contains(&addition.id)
                })
                .cloned(),
        );
        Ok(Self {
            schema_version: self.schema_version,
            visual_plan_source: Some(source),
            user_additions,
            directing_statement: Some(plan.directing_statement.clone()),
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
