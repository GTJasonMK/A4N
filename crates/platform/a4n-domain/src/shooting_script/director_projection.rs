//! 已确认导演编排到拍摄剧本叙事单元的确定性投影。
//!
//! 生成、变基与校验共用同一分组规则：连续绑定同一场景的镜头归入一个叙事单元，
//! 转场镜头单独成为一个单元；单元身份取首个镜头 ID。

use serde::Serialize;

use crate::chapter_visual_plan::{
    ChapterVisualPlanV3, ChapterVisualPlanValidationError, SceneBindingV3,
    VisualSegmentProjectionV3, VisualUserAdditionKindV1, VisualUserAdditionV1,
};
use crate::source_spans::SourceSpan;

use super::{ShootingDirectingStatementV1, ShootingEditingModeV1};

#[derive(Debug, Clone, Serialize)]
pub struct DirectorScriptPlan {
    pub directing_statement: ShootingDirectingStatementV1,
    pub user_additions: Vec<VisualUserAdditionV1>,
    pub narrative_units: Vec<DirectorScriptUnit>,
}

#[derive(Debug, Clone, Serialize)]
pub struct DirectorScriptUnit {
    pub id: String,
    pub title: String,
    pub sequence_index: u32,
    pub timeline_position: u32,
    pub editing_mode: ShootingEditingModeV1,
    pub intercut_group_id: Option<String>,
    /// Available chapter context, NOT an assertion that every shot adapts every paragraph.
    #[serde(rename = "context_source_span_ids")]
    pub source_span_ids: Vec<String>,
    pub scene_binding: SceneBindingV3,
    pub shots: Vec<VisualSegmentProjectionV3>,
}

impl DirectorScriptPlan {
    pub fn shot(&self, segment_id: &str) -> Option<&VisualSegmentProjectionV3> {
        self.narrative_units
            .iter()
            .flat_map(|unit| &unit.shots)
            .find(|shot| shot.segment_id == segment_id)
    }
}

pub fn segment_addition_id(segment_id: &str) -> String {
    format!("director:{segment_id}")
}

pub fn project_visual_plan_for_shooting_script(
    plan: &ChapterVisualPlanV3,
    source_spans: &[SourceSpan],
) -> Result<DirectorScriptPlan, ChapterVisualPlanValidationError> {
    let shots = plan.project()?;
    let source_ids = source_spans
        .iter()
        .map(|span| span.id.clone())
        .collect::<Vec<_>>();
    let mut units: Vec<DirectorScriptUnit> = Vec::new();
    let mut additions = Vec::with_capacity(shots.len());
    for shot in shots {
        additions.push(VisualUserAdditionV1 {
            id: segment_addition_id(&shot.segment_id),
            kind: VisualUserAdditionKindV1::StoryFact,
            description: shot.prompt.clone(),
            rationale: "用户确认的导演镜头设计".into(),
            introduced_at_ms: Some(shot.start_ms),
        });
        if let Some(last) = units.last_mut()
            && matches!(shot.scene_binding, SceneBindingV3::Scene { .. })
            && last.scene_binding == shot.scene_binding
        {
            last.shots.push(shot);
            continue;
        }
        let index = u32::try_from(units.len() + 1).map_err(|_| ChapterVisualPlanValidationError {
            path: "segments".into(),
            message: "镜头数量超出 u32 范围".into(),
        })?;
        units.push(DirectorScriptUnit {
            id: format!("unit:{}", shot.segment_id),
            title: shot.title.clone(),
            sequence_index: index,
            timeline_position: index,
            editing_mode: ShootingEditingModeV1::Continuity,
            intercut_group_id: None,
            source_span_ids: source_ids.clone(),
            scene_binding: shot.scene_binding.clone(),
            shots: vec![shot],
        });
    }
    Ok(DirectorScriptPlan {
        directing_statement: ShootingDirectingStatementV1 {
            central_dramatic_question: "实现每个已确认镜头段的叙事变化".into(),
            protagonist_throughline: "按时间线顺序展开人物行动".into(),
            audience_experience: "按已确认分镜理解画面、变化与场景衔接".into(),
            viewpoint_strategy: "以起点图片和段内关键帧为视觉依据".into(),
            visual_strategy: plan
                .segments
                .iter()
                .map(|segment| segment.prompt.as_str())
                .collect::<Vec<_>>()
                .join("\n"),
            spatial_strategy: "保持绑定场景和图像中的空间关系，转场遵循显式场景绑定".into(),
            sound_strategy: "在镜头段的计划时长内细化声音设计".into(),
            rhythm_strategy: format!(
                "总时长 {} 毫秒，各镜头边界与关键帧时间均由导演时间线确定",
                plan.duration_ms()
            ),
        },
        user_additions: additions,
        narrative_units: units,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chapter_visual_plan::{VisualPlanEditOperationV3, VisualPointV3};

    #[test]
    fn projection_keeps_segment_identity_and_does_not_require_one_paragraph_per_scene() {
        let mut plan = ChapterVisualPlanV3::starter();
        plan.segments[0].prompt = "开门".into();
        plan.segments[0].scene_binding = Some(SceneBindingV3::Scene {
            scene_profile_id: 1,
        });
        plan = plan
            .edited(&VisualPlanEditOperationV3::InsertPoint {
                point: VisualPointV3 {
                    id: "middle".into(),
                    at_ms: 4000,
                    label: "切点".into(),
                    image_id: Some(9),
                    description: String::new(),
                    reference_image_ids: vec![],
                },
            })
            .unwrap();
        plan = plan
            .edited(&VisualPlanEditOperationV3::SplitSegment {
                segment_id: "shot-1".into(),
                point_id: "middle".into(),
                right_segment_id: "shot-2".into(),
                left_prompt: "开门".into(),
                right_prompt: "进入室内".into(),
                left_scene_binding: Some(SceneBindingV3::Scene {
                    scene_profile_id: 1,
                }),
                right_scene_binding: Some(SceneBindingV3::Scene {
                    scene_profile_id: 2,
                }),
            })
            .unwrap();
        let projected = project_visual_plan_for_shooting_script(&plan, &[]).unwrap();
        assert_eq!(projected.narrative_units.len(), 2);
        let first = &projected.narrative_units[0].shots[0];
        let second = &projected.narrative_units[1].shots[0];
        assert_eq!(
            (first.segment_id.as_str(), first.start_ms, first.end_ms),
            ("shot-1", 0, 4000)
        );
        assert!(first.end_frame.is_none());
        assert_eq!(second.keyframes[0].image_id, Some(9));
        assert!(projected.narrative_units[0].source_span_ids.is_empty());
        assert_eq!(projected.user_additions[0].id, "director:shot-1");
        assert_eq!(projected.shot("shot-2").unwrap().start_ms, 4000);
    }
}
