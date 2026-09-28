use super::validation_v3::invalid;
use super::{
    ChapterVisualPlanV3, ChapterVisualPlanValidationError, VISUAL_PLAN_MAX_TIME_MS,
    VisualPlanEditOperationV3,
};

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
            VisualPlanEditOperationV3::InsertPoint { point } => {
                if point.at_ms == 0 || point.at_ms >= self.duration_ms() {
                    return Err(invalid("point.at_ms", "新点必须在章节两端之间"));
                }
                let index = next
                    .points
                    .partition_point(|existing| existing.at_ms < point.at_ms);
                next.points.insert(index, point.clone());
            }
            VisualPlanEditOperationV3::UpdatePoint { point } => {
                let index = self.point_index(&point.id)?;
                if point.at_ms != self.points[index].at_ms {
                    return Err(invalid("point.at_ms", "修改时间请使用移动点或整体时长伸缩"));
                }
                next.points[index] = point.clone();
            }
            VisualPlanEditOperationV3::MovePoint { point_id, at_ms } => {
                let index = self.internal_point_index(point_id)?;
                next.points[index].at_ms = *at_ms;
            }
            VisualPlanEditOperationV3::RemovePoint { point_id } => {
                let index = self.internal_point_index(point_id)?;
                if self.segments.iter().any(|segment| {
                    &segment.start_point_id == point_id || &segment.end_point_id == point_id
                }) {
                    return Err(invalid(
                        "point_id",
                        "镜头边界不能直接删除，请先合并相邻镜头",
                    ));
                }
                next.points.remove(index);
            }
            VisualPlanEditOperationV3::Remove {
                point_ids,
                segment_ids,
            } => {
                if point_ids.is_empty() && segment_ids.is_empty() {
                    return Err(invalid("remove", "批量删除至少需要一个图片点或镜头段"));
                }
                let mut segment_indices = Vec::with_capacity(segment_ids.len());
                for segment_id in segment_ids {
                    let index = self.segment_index(segment_id)?;
                    if segment_indices.contains(&index) {
                        return Err(invalid("segment_ids", "批量删除不能包含重复的镜头段"));
                    }
                    segment_indices.push(index);
                }
                segment_indices.sort_unstable_by(|left, right| right.cmp(left));
                for index in segment_indices {
                    next.segments.remove(index);
                }

                let mut point_indices = Vec::with_capacity(point_ids.len());
                for point_id in point_ids {
                    let index = self.internal_point_index(point_id)?;
                    if point_indices.contains(&index) {
                        return Err(invalid("point_ids", "批量删除不能包含重复的图片点"));
                    }
                    if next.segments.iter().any(|segment| {
                        &segment.start_point_id == point_id || &segment.end_point_id == point_id
                    }) {
                        return Err(invalid(
                            "point_ids",
                            "镜头仍引用该点，不能删除；请同时删除镜头或先合并",
                        ));
                    }
                    point_indices.push(index);
                }
                point_indices.sort_unstable_by(|left, right| right.cmp(left));
                for index in point_indices {
                    next.points.remove(index);
                }
            }
            VisualPlanEditOperationV3::RemovePoints { point_ids } => {
                if point_ids.is_empty() {
                    return Err(invalid("point_ids", "批量删除至少需要一个图片点"));
                }
                let mut indices = Vec::with_capacity(point_ids.len());
                for point_id in point_ids {
                    let index = self.internal_point_index(point_id)?;
                    if indices.contains(&index) {
                        return Err(invalid("point_ids", "批量删除不能包含重复的图片点"));
                    }
                    if self.segments.iter().any(|segment| {
                        &segment.start_point_id == point_id || &segment.end_point_id == point_id
                    }) {
                        return Err(invalid(
                            "point_ids",
                            "镜头边界不能直接批量删除，请先合并相邻镜头",
                        ));
                    }
                    indices.push(index);
                }
                indices.sort_unstable_by(|left, right| right.cmp(left));
                for index in indices {
                    next.points.remove(index);
                }
            }
            VisualPlanEditOperationV3::UpdateSegment { segment } => {
                let index = self.segment_index(&segment.id)?;
                if segment.start_point_id != self.segments[index].start_point_id
                    || segment.end_point_id != self.segments[index].end_point_id
                {
                    return Err(invalid("segment", "调整边界请使用镜头边界操作"));
                }
                next.segments[index] = segment.clone();
            }
            VisualPlanEditOperationV3::SplitSegment {
                segment_id,
                point_id,
                right_segment_id,
                left_prompt,
                right_prompt,
                left_scene_binding,
                right_scene_binding,
            } => {
                let index = self.segment_index(segment_id)?;
                let point = &self.points[self.point_index(point_id)?];
                let original = &self.segments[index];
                let start = self
                    .point(&original.start_point_id)
                    .expect("validated start");
                let end = self.point(&original.end_point_id).expect("validated end");
                if point.at_ms <= start.at_ms || point.at_ms >= end.at_ms {
                    return Err(invalid("point_id", "分镜点必须位于当前镜头内部"));
                }
                let mut right = original.clone();
                right.id = right_segment_id.clone();
                right.title = format!("{} · 后段", original.title);
                right.start_point_id = point_id.clone();
                right.prompt = right_prompt.clone();
                right.scene_binding = right_scene_binding.clone();
                let left = &mut next.segments[index];
                left.end_point_id = point_id.clone();
                left.prompt = left_prompt.clone();
                left.scene_binding = left_scene_binding.clone();
                left.use_end_point_image = false;
                next.segments.insert(index + 1, right);
            }
            VisualPlanEditOperationV3::MergeSegments {
                left_segment_id,
                right_segment_id,
                prompt,
                scene_binding,
            } => {
                let index = self.segment_index(left_segment_id)?;
                let right = self
                    .segments
                    .get(index + 1)
                    .filter(|segment| &segment.id == right_segment_id)
                    .ok_or_else(|| invalid("right_segment_id", "只能合并按时间相邻的镜头段"))?;
                let left = &mut next.segments[index];
                left.end_point_id = right.end_point_id.clone();
                left.use_end_point_image = right.use_end_point_image;
                left.prompt = prompt.clone();
                left.scene_binding = scene_binding.clone();
                next.segments.remove(index + 1);
            }
            VisualPlanEditOperationV3::MoveSegmentBoundary {
                left_segment_id,
                point_id,
            } => {
                let index = self.segment_index(left_segment_id)?;
                let right = self
                    .segments
                    .get(index + 1)
                    .ok_or_else(|| invalid("left_segment_id", "章节结束边界请使用整体时长伸缩"))?;
                let point = &self.points[self.point_index(point_id)?];
                let start = self
                    .point(&self.segments[index].start_point_id)
                    .expect("validated start");
                let end = self.point(&right.end_point_id).expect("validated end");
                if point.at_ms <= start.at_ms || point.at_ms >= end.at_ms {
                    return Err(invalid("point_id", "边界必须处于相邻两个镜头的范围内部"));
                }
                if next.segments[index].end_point_id != *point_id {
                    next.segments[index].end_point_id = point_id.clone();
                    next.segments[index].use_end_point_image = false;
                    next.segments[index + 1].start_point_id = point_id.clone();
                }
            }
            VisualPlanEditOperationV3::ScaleTimeline { duration_ms } => {
                if *duration_ms == 0 || *duration_ms > VISUAL_PLAN_MAX_TIME_MS {
                    return Err(invalid("duration_ms", "总时长必须为正的安全整数毫秒"));
                }
                let old = self.duration_ms() as u128;
                for point in &mut next.points {
                    point.at_ms =
                        ((point.at_ms as u128 * *duration_ms as u128 + old / 2) / old) as u64;
                }
            }
            VisualPlanEditOperationV3::SetFrame { frame } => next.frame = frame.clone(),
        }
        next.validate_draft()?;
        Ok(next)
    }

    fn point_index(&self, id: &str) -> Result<usize, ChapterVisualPlanValidationError> {
        self.points
            .iter()
            .position(|point| point.id == id)
            .ok_or_else(|| invalid("point_id", "图片点不存在"))
    }

    fn internal_point_index(&self, id: &str) -> Result<usize, ChapterVisualPlanValidationError> {
        let index = self.point_index(id)?;
        if index == 0 || index + 1 == self.points.len() {
            return Err(invalid("point_id", "章节两端固定；修改总时长请整体伸缩"));
        }
        Ok(index)
    }

    fn segment_index(&self, id: &str) -> Result<usize, ChapterVisualPlanValidationError> {
        self.segments
            .iter()
            .position(|segment| segment.id == id)
            .ok_or_else(|| invalid("segment_id", "镜头段不存在"))
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
