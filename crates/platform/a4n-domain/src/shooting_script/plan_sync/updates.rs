use std::collections::HashSet;

use super::super::{
    ChapterShootingScriptV2, ShootingScriptValidationError, director_projection::DirectorScriptPlan,
};

impl ChapterShootingScriptV2 {
    /// 只采用模型返回的指定镜头，保留其他镜头和已编辑的场景信息。
    pub fn with_updated_shots(
        &self,
        plan: &DirectorScriptPlan,
        generated_scenes: Vec<super::super::ShootingSceneV2>,
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
                Some(old) => old.shots.extend(scene.shots),
                None => result.scenes.push(scene),
            }
        }
        let source = self
            .visual_plan_source
            .clone()
            .ok_or_else(|| invalid("visual_plan_source", "局部更新需要已绑定导演编排的拍摄剧本"))?;
        result.rebased_onto(plan, source)
    }
}

fn invalid(path: impl Into<String>, message: impl Into<String>) -> ShootingScriptValidationError {
    ShootingScriptValidationError {
        path: path.into(),
        message: message.into(),
    }
}
