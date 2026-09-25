//! 拍摄剧本领域不变量校验。

use thiserror::Error;

use super::contracts::{ChapterShootingScriptV1, ShootingDirectingStatementV1, ShootingSceneV1};

mod rules;
mod scene;
mod script;
#[cfg(test)]
mod tests;

#[derive(Debug, Clone, Error, PartialEq, Eq)]
#[error("拍摄剧本字段 {path} 无效: {message}")]
pub struct ShootingScriptValidationError {
    pub path: String,
    pub message: String,
}

impl ShootingDirectingStatementV1 {
    pub fn validate(&self) -> Result<(), ShootingScriptValidationError> {
        script::validate_statement(self)
    }
}

impl ShootingSceneV1 {
    /// 调用输出经确定性装配后复用发布时的内容规则；跨场景引用仍由整章校验负责。
    pub fn validate_content(
        &self,
        user_additions: &[crate::chapter_visual_plan::VisualUserAdditionV1],
        requires_visual_plan: bool,
    ) -> Result<(), ShootingScriptValidationError> {
        scene::validate_scene(
            self,
            &format!("scenes[{}]", self.scene_index),
            &mut std::collections::HashSet::new(),
            &mut std::collections::HashSet::new(),
            &user_additions
                .iter()
                .map(|addition| addition.id.as_str())
                .collect(),
            requires_visual_plan,
        )
    }
}

impl ChapterShootingScriptV1 {
    /// 逐镜头校验导演约束。与当前方案内容一致的镜头必须落在当前计划时间；内容已变化的
    /// 过时镜头只能原样保留上一版本的约束，等待手动或模型辅助更新，不能被改写成其他约束。
    pub fn validate_director_plan(
        &self,
        plan: &crate::chapter_visual_plan::ChapterVisualPlanV3,
        previous: Option<&ChapterShootingScriptV1>,
    ) -> Result<(), ShootingScriptValidationError> {
        let mut expected = plan
            .project()
            .map_err(|error| ShootingScriptValidationError {
                path: error.path,
                message: error.message,
            })?
            .into_iter();
        let previous_constraints = previous
            .into_iter()
            .flat_map(|script| &script.scenes)
            .flat_map(|scene| &scene.shots)
            .filter_map(|shot| {
                shot.visual_constraints
                    .as_ref()
                    .map(|constraints| (shot.shot_id.as_str(), constraints))
            })
            .collect::<std::collections::HashMap<_, _>>();
        let actual = self
            .scenes
            .iter()
            .flat_map(|scene| &scene.shots)
            .collect::<Vec<_>>();
        // 作者可以删去场景；保留的镜头仍必须按导演时间线顺序出现并保持约束。
        for shot in actual {
            let director = expected
                .find(|director| director.segment_id == shot.shot_id)
                .ok_or_else(|| ShootingScriptValidationError {
                    path: format!("shots[{}]", shot.shot_id),
                    message: "镜头必须来自导演时间线，且不能重复或改变顺序".into(),
                })?;
            let constraints = shot
                .visual_constraints
                .as_ref()
                .filter(|_| shot.visual_shot_id.as_deref() == Some(director.segment_id.as_str()))
                .filter(|constraints| {
                    constraints.same_content(&director)
                        || previous_constraints.get(shot.shot_id.as_str()) == Some(constraints)
                })
                .ok_or_else(|| ShootingScriptValidationError {
                    path: format!("shots[{}].visual_constraints", shot.shot_id),
                    message: "拍摄剧本不能改变已确认的镜头边界、图片、场景和变化描述".into(),
                })?;
            let planned = if constraints.same_content(&director) {
                (Some(director.start_ms), Some(director.end_ms))
            } else {
                (shot.planned_start_ms, shot.planned_end_ms)
            };
            let duration_ms = constraints.end_ms.saturating_sub(constraints.start_ms);
            if (shot.planned_start_ms, shot.planned_end_ms) != planned
                || planned
                    .0
                    .zip(planned.1)
                    .and_then(|(start, end)| end.checked_sub(start))
                    != Some(duration_ms)
            {
                return Err(ShootingScriptValidationError {
                    path: format!("shots[{}].planned_start_ms", shot.shot_id),
                    message: "镜头计划时间必须与导演编排一致".into(),
                });
            }
            if (shot.estimated_natural_seconds - duration_ms as f64 / 1000.0).abs() > 0.001 {
                return Err(ShootingScriptValidationError {
                    path: format!("shots[{}].estimated_natural_seconds", shot.shot_id),
                    message: "镜头时长必须与导演编排一致".into(),
                });
            }
        }
        Ok(())
    }
    pub fn validate(&self) -> Result<(), ShootingScriptValidationError> {
        script::validate_script(self, None)
    }

    pub fn validate_partial(
        &self,
        completed_unit_count: usize,
        total_unit_count: usize,
    ) -> Result<(), ShootingScriptValidationError> {
        if completed_unit_count == 0
            || completed_unit_count > total_unit_count
            || self.scenes.len() > completed_unit_count
            || self
                .scenes
                .iter()
                .any(|scene| scene.narrative_unit_index as usize > completed_unit_count)
        {
            return Err(ShootingScriptValidationError {
                path: "progress.completed_unit_count".into(),
                message: "生成游标、场景范围或总单元数不一致".into(),
            });
        }
        script::validate_script(self, Some((completed_unit_count, total_unit_count)))
    }
}
