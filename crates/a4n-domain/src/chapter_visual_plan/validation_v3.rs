use std::collections::{BTreeMap, BTreeSet};

use super::{
    ChapterVisualPlanSchemaVersion, ChapterVisualPlanV3, ChapterVisualPlanValidationError,
    SceneBindingV3, VISUAL_PLAN_MAX_TIME_MS,
};

pub(super) fn invalid(
    path: impl Into<String>,
    message: impl Into<String>,
) -> ChapterVisualPlanValidationError {
    ChapterVisualPlanValidationError {
        path: path.into(),
        message: message.into(),
    }
}

impl ChapterVisualPlanV3 {
    pub fn validate_draft(&self) -> Result<(), ChapterVisualPlanValidationError> {
        self.validate(false)
    }

    pub fn validate_confirmed(&self) -> Result<(), ChapterVisualPlanValidationError> {
        self.validate(true)
    }

    pub fn canonical_json(&self) -> Result<serde_json::Value, ChapterVisualPlanValidationError> {
        self.validate_draft()?;
        serde_json::to_value(self).map_err(|error| invalid("$", error.to_string()))
    }

    fn validate(&self, confirmed: bool) -> Result<(), ChapterVisualPlanValidationError> {
        if self.schema_version != ChapterVisualPlanSchemaVersion::V3 {
            return Err(invalid("schema_version", "必须是 chapter_visual_plan_v3"));
        }
        if self.frame.width == 0
            || self.frame.height == 0
            || self.frame.width > i32::MAX as u32
            || self.frame.height > i32::MAX as u32
        {
            return Err(invalid("frame", "画幅宽高必须为有效正整数"));
        }
        if self.points.len() < 2 || self.points.first().is_none_or(|point| point.at_ms != 0) {
            return Err(invalid("points", "必须包含从 0 开始的两个边界点"));
        }
        let mut ids = BTreeSet::new();
        let mut points = BTreeMap::new();
        for (index, point) in self.points.iter().enumerate() {
            let path = format!("points[{index}]");
            if point.id.trim().is_empty() || !ids.insert(point.id.as_str()) {
                return Err(invalid(format!("{path}.id"), "ID 必须非空且唯一"));
            }
            if point.at_ms > VISUAL_PLAN_MAX_TIME_MS
                || (index > 0 && point.at_ms <= self.points[index - 1].at_ms)
            {
                return Err(invalid(
                    format!("{path}.at_ms"),
                    "时间必须为安全整数，严格递增且不能重合",
                ));
            }
            let references = point
                .reference_image_ids
                .iter()
                .copied()
                .collect::<BTreeSet<_>>();
            if point.image_id.is_some_and(|id| id <= 0)
                || references.iter().any(|id| *id <= 0)
                || references.len() != point.reference_image_ids.len()
            {
                return Err(invalid(
                    format!("{path}.image_id"),
                    "图片 ID 必须为正整数，参考图片不能重复",
                ));
            }
            points.insert(point.id.as_str(), point);
        }
        if self.segments.is_empty() {
            return Err(invalid("segments", "镜头段必须完整覆盖时间线"));
        }
        let mut previous_end = self.points[0].id.as_str();
        for (index, segment) in self.segments.iter().enumerate() {
            let path = format!("segments[{index}]");
            if segment.id.trim().is_empty() || !ids.insert(segment.id.as_str()) {
                return Err(invalid(format!("{path}.id"), "ID 必须非空且唯一"));
            }
            let start = points
                .get(segment.start_point_id.as_str())
                .ok_or_else(|| invalid(format!("{path}.start_point_id"), "起点不存在"))?;
            let end = points
                .get(segment.end_point_id.as_str())
                .ok_or_else(|| invalid(format!("{path}.end_point_id"), "终点不存在"))?;
            if start.at_ms >= end.at_ms || segment.start_point_id != previous_end {
                return Err(invalid(path, "镜头段必须首尾相接、有正时长且不能重叠"));
            }
            previous_end = &segment.end_point_id;
            if let Some(binding) = &segment.scene_binding {
                if binding.entry_scene_id() <= 0 || binding.exit_scene_id() <= 0 {
                    return Err(invalid(
                        format!("{path}.scene_binding"),
                        "场景 ID 必须为正整数",
                    ));
                }
                if confirmed && matches!(binding, SceneBindingV3::Transition { .. }) {
                    let previous = index
                        .checked_sub(1)
                        .and_then(|i| self.segments[i].scene_binding.as_ref());
                    let next = self
                        .segments
                        .get(index + 1)
                        .and_then(|segment| segment.scene_binding.as_ref());
                    if previous
                        .is_some_and(|scene| scene.exit_scene_id() != binding.entry_scene_id())
                        || next
                            .is_some_and(|scene| scene.entry_scene_id() != binding.exit_scene_id())
                    {
                        return Err(invalid(
                            format!("{path}.scene_binding"),
                            "转场的起止场景须与相邻镜头一致",
                        ));
                    }
                }
            } else if confirmed {
                return Err(invalid(
                    format!("{path}.scene_binding"),
                    "确认前需要绑定场景",
                ));
            }
            if confirmed && segment.prompt.trim().is_empty() {
                return Err(invalid(
                    format!("{path}.prompt"),
                    "确认前需要填写镜头变化描述",
                ));
            }
            if confirmed && segment.use_end_point_image && end.image_id.is_none() {
                return Err(invalid(
                    format!("{path}.use_end_point_image"),
                    "显式尾帧关联需要结束点已绑定图片",
                ));
            }
        }
        if previous_end != self.points.last().expect("validated endpoints").id {
            return Err(invalid("segments", "最后一个镜头段必须到达章节终点"));
        }
        Ok(())
    }
}
