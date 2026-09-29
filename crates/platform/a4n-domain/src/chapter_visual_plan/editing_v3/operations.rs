use super::super::validation_v3::invalid;
use super::super::{
    ChapterVisualPlanV3, ChapterVisualPlanValidationError, VISUAL_PLAN_MAX_TIME_MS,
    VisualPlanEditOperationV3,
};

impl ChapterVisualPlanV3 {
    pub(super) fn apply_single_edit(
        &self,
        next: &mut Self,
        operation: &VisualPlanEditOperationV3,
    ) -> Result<(), ChapterVisualPlanValidationError> {
        match operation {
            VisualPlanEditOperationV3::Batch { .. } => {
                return Err(invalid("operations", "不允许嵌套编辑批次"));
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
        Ok(())
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
