//! 拍摄剧本领域不变量校验。

use thiserror::Error;

use super::contracts::ChapterShootingScriptV1;

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

impl ChapterShootingScriptV1 {
    pub fn validate_director_plan(
        &self,
        plan: &crate::chapter_visual_plan::ChapterVisualPlanV3,
    ) -> Result<(), ShootingScriptValidationError> {
        let mut expected = plan
            .project()
            .map_err(|error| ShootingScriptValidationError {
                path: error.path,
                message: error.message,
            })?
            .into_iter();
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
            if shot.visual_shot_id.as_deref() != Some(director.segment_id.as_str())
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
