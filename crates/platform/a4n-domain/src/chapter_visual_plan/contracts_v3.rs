use std::collections::BTreeSet;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use super::{ChapterVisualPlanSchemaVersion, ChapterVisualPlanValidationError};

pub const CHAPTER_VISUAL_PLAN_V3: &str = "chapter_visual_plan_v3";
pub const DIRECTOR_SEGMENT_PROMPT: &str = "director_segment_prompt";
pub const VISUAL_PLAN_MAX_TIME_MS: u64 = 9_007_199_254_740_991;

/// Points own time and images; segments reference points, never duplicate timing.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ChapterVisualPlanV3 {
    pub schema_version: ChapterVisualPlanSchemaVersion,
    pub frame: VisualFrameV3,
    pub points: Vec<VisualPointV3>,
    pub segments: Vec<VisualSegmentV3>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VisualFrameV3 {
    pub width: u32,
    pub height: u32,
}

impl VisualFrameV3 {
    pub fn aspect_ratio(&self) -> Result<String, String> {
        if self.width == 0 || self.height == 0 {
            return Err("画幅宽高必须大于 0".into());
        }
        let (mut a, mut b) = (self.width, self.height);
        while b != 0 {
            let remainder = a % b;
            a = b;
            b = remainder;
        }
        Ok(format!("{}:{}", self.width / a, self.height / a))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VisualPointV3 {
    pub id: String,
    #[schemars(range(max = 9_007_199_254_740_991_u64))]
    pub at_ms: u64,
    pub label: String,
    pub image_id: Option<i64>,
    pub description: String,
    pub reference_image_ids: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VisualSegmentV3 {
    pub id: String,
    pub title: String,
    pub start_point_id: String,
    pub end_point_id: String,
    pub prompt: String,
    pub scene_binding: Option<SceneBindingV3>,
    /// A boundary usually belongs to the NEXT shot. Never infer this from scene equality.
    pub use_end_point_image: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SceneBindingV3 {
    Scene {
        scene_profile_id: i64,
    },
    Transition {
        from_scene_profile_id: i64,
        to_scene_profile_id: i64,
    },
}

impl SceneBindingV3 {
    pub fn entry_scene_id(&self) -> i64 {
        match self {
            Self::Scene { scene_profile_id } => *scene_profile_id,
            Self::Transition {
                from_scene_profile_id,
                ..
            } => *from_scene_profile_id,
        }
    }

    pub fn exit_scene_id(&self) -> i64 {
        match self {
            Self::Scene { scene_profile_id } => *scene_profile_id,
            Self::Transition {
                to_scene_profile_id,
                ..
            } => *to_scene_profile_id,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
#[schemars(rename = "VisualPlanEditOperation")]
pub enum VisualPlanEditOperationV3 {
    /// One validated transaction and one undo step. Nested batches are rejected.
    Batch {
        operations: Vec<VisualPlanEditOperationV3>,
    },
    InsertPoint {
        point: VisualPointV3,
    },
    UpdatePoint {
        point: VisualPointV3,
    },
    MovePoint {
        point_id: String,
        at_ms: u64,
    },
    RemovePoint {
        point_id: String,
    },
    Remove {
        point_ids: Vec<String>,
        segment_ids: Vec<String>,
    },
    RemovePoints {
        point_ids: Vec<String>,
    },
    UpdateSegment {
        segment: VisualSegmentV3,
    },
    SplitSegment {
        segment_id: String,
        point_id: String,
        right_segment_id: String,
        left_prompt: String,
        right_prompt: String,
        left_scene_binding: Option<SceneBindingV3>,
        right_scene_binding: Option<SceneBindingV3>,
    },
    MergeSegments {
        left_segment_id: String,
        right_segment_id: String,
        prompt: String,
        scene_binding: Option<SceneBindingV3>,
    },
    MoveSegmentBoundary {
        left_segment_id: String,
        point_id: String,
    },
    ScaleTimeline {
        duration_ms: u64,
    },
    SetFrame {
        frame: VisualFrameV3,
    },
}

/// Immutable, ordered generation constraints projected from a confirmed segment.
#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VisualSegmentProjectionV3 {
    pub frame: VisualFrameV3,
    pub segment_id: String,
    pub title: String,
    pub start_ms: u64,
    pub end_ms: u64,
    pub prompt: String,
    pub scene_binding: SceneBindingV3,
    /// Includes the start point and internal points, with segment-relative times.
    pub keyframes: Vec<VisualPointV3>,
    /// Present only when explicitly associated; not the next shot's default image.
    pub end_frame: Option<VisualPointV3>,
}

impl VisualSegmentProjectionV3 {
    /// 镜头内容身份：绝对起点与标题只决定位置和展示，可按新方案确定性改写，不使下游过时。
    pub fn same_content(&self, other: &Self) -> bool {
        self.content_view() == other.content_view()
    }

    pub fn content_view(&self) -> Self {
        let mut view = self.clone();
        view.end_ms = self.end_ms.saturating_sub(self.start_ms);
        view.start_ms = 0;
        view.title.clear();
        for point in view.keyframes.iter_mut().chain(view.end_frame.iter_mut()) {
            point.label.clear();
        }
        view
    }

    pub fn remap_resources(
        &mut self,
        mut image: impl FnMut(i64) -> Result<i64, String>,
        mut scene: impl FnMut(i64) -> Result<i64, String>,
    ) -> Result<(), String> {
        let mut next = self.clone();
        for point in next.keyframes.iter_mut().chain(next.end_frame.iter_mut()) {
            remap_point_images(point, &mut image)?;
        }
        remap_scene(&mut next.scene_binding, &mut scene)?;
        *self = next;
        Ok(())
    }
    pub fn image_ids(&self) -> Vec<i64> {
        let mut seen = BTreeSet::new();
        self.keyframes
            .iter()
            .chain(self.end_frame.iter())
            .filter_map(|point| point.image_id)
            .filter(|id| seen.insert(*id))
            .collect()
    }

    pub fn video_duration_secs(&self) -> Result<i32, String> {
        let duration = self
            .end_ms
            .checked_sub(self.start_ms)
            .ok_or("镜头时间范围无效")?;
        if duration == 0 || duration % 1000 != 0 {
            return Err(format!(
                "镜头 {} 的时长为 {duration} 毫秒；当前视频执行接口只接受整秒时长，请先调整导演编排，不会自动舍入",
                self.segment_id
            ));
        }
        i32::try_from(duration / 1000).map_err(|_| "镜头时长超出视频执行范围".into())
    }
}

impl ChapterVisualPlanV3 {
    pub fn starter() -> Self {
        let point = |id: &str, label: &str, at_ms| VisualPointV3 {
            id: id.into(),
            at_ms,
            label: label.into(),
            image_id: None,
            description: String::new(),
            reference_image_ids: Vec::new(),
        };
        Self {
            schema_version: ChapterVisualPlanSchemaVersion::V3,
            frame: VisualFrameV3 {
                width: 1920,
                height: 1080,
            },
            points: vec![point("start", "开始", 0), point("end", "结束", 10_000)],
            segments: vec![VisualSegmentV3 {
                id: "shot-1".into(),
                title: "镜头 1".into(),
                start_point_id: "start".into(),
                end_point_id: "end".into(),
                prompt: String::new(),
                scene_binding: None,
                use_end_point_image: false,
            }],
        }
    }

    pub fn duration_ms(&self) -> u64 {
        self.points.last().map_or(0, |point| point.at_ms)
    }

    pub fn point(&self, id: &str) -> Option<&VisualPointV3> {
        self.points.iter().find(|point| point.id == id)
    }

    pub fn referenced_image_ids(&self) -> BTreeSet<i64> {
        self.points
            .iter()
            .flat_map(|point| {
                point
                    .image_id
                    .into_iter()
                    .chain(point.reference_image_ids.iter().copied())
            })
            .collect()
    }

    pub fn referenced_scene_ids(&self) -> BTreeSet<i64> {
        self.segments
            .iter()
            .filter_map(|segment| segment.scene_binding.as_ref())
            .flat_map(|scene| [scene.entry_scene_id(), scene.exit_scene_id()])
            .collect()
    }

    pub fn remap_resources(
        &mut self,
        mut image: impl FnMut(i64) -> Result<i64, String>,
        mut scene: impl FnMut(i64) -> Result<i64, String>,
    ) -> Result<(), String> {
        let mut next = self.clone();
        for point in &mut next.points {
            remap_point_images(point, &mut image)?;
        }
        for binding in next
            .segments
            .iter_mut()
            .filter_map(|segment| segment.scene_binding.as_mut())
        {
            remap_scene(binding, &mut scene)?;
        }
        next.validate_draft().map_err(|error| error.to_string())?;
        *self = next;
        Ok(())
    }

    pub fn project(
        &self,
    ) -> Result<Vec<VisualSegmentProjectionV3>, ChapterVisualPlanValidationError> {
        self.validate_confirmed()?;
        self.segments
            .iter()
            .enumerate()
            .map(|(index, segment)| {
                let start = self
                    .point(&segment.start_point_id)
                    .expect("validated start point")
                    .at_ms;
                let end = self
                    .point(&segment.end_point_id)
                    .expect("validated end point");
                let relative = |point: &VisualPointV3| {
                    let mut point = point.clone();
                    point.at_ms -= start;
                    point
                };
                Ok(VisualSegmentProjectionV3 {
                    frame: self.frame.clone(),
                    segment_id: segment.id.clone(),
                    title: if segment.title.trim().is_empty() {
                        format!("镜头 {}", index + 1)
                    } else {
                        segment.title.clone()
                    },
                    start_ms: start,
                    end_ms: end.at_ms,
                    prompt: segment.prompt.clone(),
                    scene_binding: segment
                        .scene_binding
                        .clone()
                        .expect("validated scene binding"),
                    keyframes: self
                        .points
                        .iter()
                        .filter(|point| point.at_ms >= start && point.at_ms < end.at_ms)
                        .map(relative)
                        .collect(),
                    end_frame: segment.use_end_point_image.then(|| relative(end)),
                })
            })
            .collect()
    }
}

fn remap_point_images(
    point: &mut VisualPointV3,
    image: &mut impl FnMut(i64) -> Result<i64, String>,
) -> Result<(), String> {
    point.image_id = point.image_id.map(&mut *image).transpose()?;
    for id in &mut point.reference_image_ids {
        *id = image(*id)?;
    }
    Ok(())
}

fn remap_scene(
    binding: &mut SceneBindingV3,
    scene: &mut impl FnMut(i64) -> Result<i64, String>,
) -> Result<(), String> {
    match binding {
        SceneBindingV3::Scene { scene_profile_id } => *scene_profile_id = scene(*scene_profile_id)?,
        SceneBindingV3::Transition {
            from_scene_profile_id,
            to_scene_profile_id,
        } => {
            *from_scene_profile_id = scene(*from_scene_profile_id)?;
            *to_scene_profile_id = scene(*to_scene_profile_id)?;
        }
    }
    Ok(())
}
