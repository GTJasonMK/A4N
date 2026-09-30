use std::collections::BTreeMap;

use crate::chapter_visual_plan::VisualSegmentProjectionV3;

use super::super::{ChapterShootingScriptV2, director_projection::DirectorScriptPlan};

impl ChapterShootingScriptV2 {
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
                fingerprints.insert(
                    shot.shot_id.clone(),
                    crate::generation::canonical_json_sha256(&serde_json::json!({
                        "scene": &shared,
                        "shot": content,
                        "user_additions": additions,
                        "directing_statement": self.directing_statement
                    }))?,
                );
            }
        }
        Ok(fingerprints)
    }
}
