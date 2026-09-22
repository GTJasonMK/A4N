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
    /// 单场景调用完成后复用发布时的内容规则；跨场景引用仍由整章校验负责。
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
    pub fn validate_director_plan(
        &self,
        plan: &crate::chapter_visual_plan::ChapterVisualPlanV3,
    ) -> Result<(), ShootingScriptValidationError> {
        let expected = plan
            .project()
            .map_err(|error| ShootingScriptValidationError {
                path: error.path,
                message: error.message,
            })?;
        let actual = self
            .scenes
            .iter()
            .flat_map(|scene| &scene.shots)
            .collect::<Vec<_>>();
        if actual.len() != expected.len() {
            return Err(ShootingScriptValidationError {
                path: "scenes".into(),
                message: "拍摄剧本镜头数量必须与导演时间线一致".into(),
            });
        }
        for (shot, director) in actual.into_iter().zip(expected) {
            if shot.shot_id != director.segment_id
                || shot.visual_shot_id.as_deref() != Some(director.segment_id.as_str())
                || shot.planned_start_ms != Some(director.start_ms)
                || shot.planned_end_ms != Some(director.end_ms)
                || shot.visual_constraints.as_ref() != Some(&director)
            {
                return Err(ShootingScriptValidationError {
                    path: format!("shots[{}].visual_constraints", shot.shot_id),
                    message: "拍摄剧本不能改变已确认的镜头边界、图片、场景和变化描述".into(),
                });
            }
            if (shot.estimated_natural_seconds
                - (director.end_ms - director.start_ms) as f64 / 1000.0)
                .abs()
                > 0.001
            {
                return Err(ShootingScriptValidationError {
                    path: format!("shots[{}].estimated_natural_seconds", shot.shot_id),
                    message: "镜头时长必须与导演编排一致".into(),
                });
            }
        }
        Ok(())
    }
    pub fn validate(&self) -> Result<(), ShootingScriptValidationError> {
        script::validate_script(self)
    }
}
