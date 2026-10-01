mod operations;

use super::validation_v3::invalid;
use super::{ChapterVisualPlanV3, ChapterVisualPlanValidationError, VisualPlanEditOperationV3};

impl ChapterVisualPlanV3 {
    /// Apply atomically, including validation: invalid gestures never partially change a draft.
    pub fn edited(
        &self,
        operation: &VisualPlanEditOperationV3,
    ) -> Result<Self, ChapterVisualPlanValidationError> {
        self.validate_draft()?;
        let mut next = self.clone();
        match operation {
            VisualPlanEditOperationV3::Batch { operations } => {
                if operations.is_empty() || operations.len() > 100 {
                    return Err(invalid("operations", "批次必须包含 1 到 100 个操作"));
                }
                for operation in operations {
                    if matches!(operation, VisualPlanEditOperationV3::Batch { .. }) {
                        return Err(invalid("operations", "不允许嵌套编辑批次"));
                    }
                    next = next.edited(operation)?;
                }
            }
            operation => self.apply_single_edit(&mut next, operation)?,
        }
        next.validate_draft()?;
        Ok(next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chapter_visual_plan::{SceneBindingV3, VisualPointV3};

    fn fixture() -> ChapterVisualPlanV3 {
        let mut plan = ChapterVisualPlanV3::starter();
        plan.points[1].at_ms = 12_000;
        plan.points[1].image_id = Some(99);
        plan.segments[0].prompt = "沿街跟拍".into();
        plan.segments[0].scene_binding = Some(SceneBindingV3::Scene {
            scene_profile_id: 1,
        });
        for time in [2_000, 4_000, 6_000, 8_000, 10_000] {
            plan = plan
                .edited(&VisualPlanEditOperationV3::InsertPoint {
                    point: VisualPointV3 {
                        id: format!("point-{time}"),
                        at_ms: time,
                        label: time.to_string(),
                        image_id: Some(time as i64),
                        description: String::new(),
                        reference_image_ids: vec![],
                    },
                })
                .unwrap();
        }
        plan
    }

    #[test]
    fn points_do_not_split_shots_and_endpoint_is_opt_in() {
        let mut plan = fixture();
        assert_eq!(plan.segments.len(), 1);
        let shots = plan.project().unwrap();
        assert_eq!(shots[0].keyframes.len(), 6);
        assert!(shots[0].end_frame.is_none());
        assert_eq!(shots[0].keyframes.last().unwrap().at_ms, 10_000);
        plan.segments[0].use_end_point_image = true;
        assert_eq!(
            plan.project().unwrap()[0]
                .end_frame
                .as_ref()
                .unwrap()
                .image_id,
            Some(99)
        );
    }

    #[test]
    fn batch_remove_points_is_atomic_and_keeps_structural_points() {
        let plan = fixture();
        let removed = plan
            .edited(&VisualPlanEditOperationV3::RemovePoints {
                point_ids: vec![
                    "point-2000".into(),
                    "point-4000".into(),
                    "point-6000".into(),
                ],
            })
            .unwrap();

        assert_eq!(
            removed
                .points
                .iter()
                .map(|point| point.id.as_str())
                .collect::<Vec<_>>(),
            vec!["start", "point-8000", "point-10000", "end"]
        );
        assert!(removed.validate_draft().is_ok());
        assert!(
            plan.edited(&VisualPlanEditOperationV3::RemovePoints {
                point_ids: vec!["point-2000".into(), "point-2000".into()],
            })
            .is_err()
        );
        assert_eq!(plan.points.len(), 7);
    }

    #[test]
    fn batch_remove_can_empty_a_draft_but_not_a_confirmed_plan() {
        let plan = fixture();
        let removed = plan
            .edited(&VisualPlanEditOperationV3::Remove {
                point_ids: plan.points[1..plan.points.len() - 1]
                    .iter()
                    .map(|point| point.id.clone())
                    .collect(),
                segment_ids: plan
                    .segments
                    .iter()
                    .map(|segment| segment.id.clone())
                    .collect(),
            })
            .unwrap();

        assert_eq!(removed.points.len(), 2);
        assert!(removed.segments.is_empty());
        assert!(removed.validate_draft().is_ok());
        assert!(removed.validate_confirmed().is_err());
    }

    #[test]
    fn scaling_preserves_identity_and_is_atomic_when_points_collide() {
        let plan = fixture();
        let scaled = plan
            .edited(&VisualPlanEditOperationV3::ScaleTimeline { duration_ms: 6_000 })
            .unwrap();
        assert_eq!(
            scaled
                .points
                .iter()
                .map(|point| point.at_ms)
                .collect::<Vec<_>>(),
            vec![0, 1000, 2000, 3000, 4000, 5000, 6000]
        );
        assert_eq!(plan.segments, scaled.segments);
        assert_eq!(plan.points[2].image_id, scaled.points[2].image_id);
        assert!(
            plan.edited(&VisualPlanEditOperationV3::ScaleTimeline { duration_ms: 2 })
                .is_err()
        );
        assert_eq!(plan.duration_ms(), 12_000);
        for id in ["start", "end"] {
            assert!(
                plan.edited(&VisualPlanEditOperationV3::RemovePoint {
                    point_id: id.into()
                })
                .is_err()
            );
            assert!(
                plan.edited(&VisualPlanEditOperationV3::MovePoint {
                    point_id: id.into(),
                    at_ms: 100
                })
                .is_err()
            );
        }
    }

    #[test]
    fn split_merge_boundary_and_scene_transitions_keep_one_timeline() {
        let plan = fixture();
        let mut split = plan
            .edited(&VisualPlanEditOperationV3::SplitSegment {
                segment_id: "shot-1".into(),
                point_id: "point-6000".into(),
                right_segment_id: "shot-2".into(),
                left_prompt: "跟拍".into(),
                right_prompt: "走入室内".into(),
                left_scene_binding: Some(SceneBindingV3::Scene {
                    scene_profile_id: 1,
                }),
                right_scene_binding: Some(SceneBindingV3::Transition {
                    from_scene_profile_id: 1,
                    to_scene_profile_id: 2,
                }),
            })
            .unwrap();
        assert_eq!(split.project().unwrap()[1].keyframes[0].at_ms, 0);
        assert!(
            split
                .edited(&VisualPlanEditOperationV3::RemovePoint {
                    point_id: "point-6000".into()
                })
                .is_err()
        );
        split = split
            .edited(&VisualPlanEditOperationV3::MoveSegmentBoundary {
                left_segment_id: "shot-1".into(),
                point_id: "point-4000".into(),
            })
            .unwrap();
        assert_eq!(split.segments[1].start_point_id, "point-4000");
        split.segments[1].scene_binding = Some(SceneBindingV3::Transition {
            from_scene_profile_id: 3,
            to_scene_profile_id: 2,
        });
        assert!(split.validate_draft().is_ok());
        assert!(split.validate_confirmed().is_err());
        let merged = split
            .edited(&VisualPlanEditOperationV3::MergeSegments {
                left_segment_id: "shot-1".into(),
                right_segment_id: "shot-2".into(),
                prompt: "跟随人物进入室内".into(),
                scene_binding: Some(SceneBindingV3::Transition {
                    from_scene_profile_id: 1,
                    to_scene_profile_id: 2,
                }),
            })
            .unwrap();
        assert_eq!(merged.points, plan.points);
        assert_eq!(merged.segments.len(), 1);
        assert!(merged.validate_confirmed().is_ok());
    }

    #[test]
    fn draft_completeness_and_resource_remapping_are_explicit() {
        let mut plan = fixture();
        plan.points[0].reference_image_ids = vec![21, 22];
        assert_eq!(
            plan.referenced_scene_ids().into_iter().collect::<Vec<_>>(),
            vec![1]
        );
        let original = plan.clone();
        assert!(
            plan.remap_resources(|_| Err("missing image".into()), Ok)
                .is_err()
        );
        assert_eq!(plan, original);
        plan.remap_resources(|id| Ok(id + 100), |id| Ok(id + 10))
            .unwrap();
        assert_eq!(plan.points[0].reference_image_ids, vec![121, 122]);
        assert_eq!(
            plan.referenced_scene_ids().into_iter().collect::<Vec<_>>(),
            vec![11]
        );
        let starter = ChapterVisualPlanV3::starter();
        assert!(starter.validate_draft().is_ok());
        assert!(starter.validate_confirmed().is_err());
        plan.points
            .iter_mut()
            .for_each(|point| point.image_id = None);
        assert!(plan.validate_confirmed().is_ok());
        plan.segments[0].use_end_point_image = true;
        assert!(plan.validate_confirmed().is_err());
    }
    #[test]
    fn batches_are_atomic_and_cannot_nest() {
        let plan = ChapterVisualPlanV3::starter();
        let operation = VisualPlanEditOperationV3::Batch {
            operations: vec![
                VisualPlanEditOperationV3::SetFrame {
                    frame: super::super::VisualFrameV3 {
                        width: 1080,
                        height: 1920,
                    },
                },
                VisualPlanEditOperationV3::MovePoint {
                    point_id: "missing".into(),
                    at_ms: 1000,
                },
            ],
        };
        assert!(plan.edited(&operation).is_err());
        assert_eq!(plan, ChapterVisualPlanV3::starter());
        assert!(
            plan.edited(&VisualPlanEditOperationV3::Batch {
                operations: vec![operation]
            })
            .is_err()
        );
    }
}
