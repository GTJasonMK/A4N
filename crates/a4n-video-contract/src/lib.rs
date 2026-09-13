//! Pure video execution contracts shared by Host, database, and workers.
//!
//! This crate deliberately contains no ORM, provider, filesystem, or runtime
//! state. Implementations can evolve behind these serialized contracts.

mod contracts;
mod segment_recipe;

pub use contracts::*;
pub use segment_recipe::*;

use a4n_domain::{
    VideoAspectRatio,
    video_workflow::{
        WORKFLOW_MODE_BOUNDARY_FRAMES, WORKFLOW_MODE_CHAT_VIDEO, WORKFLOW_MODE_FIRST_FRAME_CHAIN,
        WORKFLOW_MODE_REFERENCE_MEDIA, WORKFLOW_MODE_STORYBOARD,
    },
};
use serde::Serialize;
use sha2::{Digest, Sha256};

pub const VIDEO_SEGMENT_EXECUTION_RECIPE_V1: &str = "video_segment_execution_recipe_v1";

#[derive(
    Debug, Clone, Copy, serde::Serialize, serde::Deserialize, schemars::JsonSchema, PartialEq, Eq,
)]
#[serde(rename_all = "snake_case")]
pub enum VideoSegmentWorkflowModeV1 {
    BoundaryFrames,
    FirstFrameChain,
    Storyboard,
    ReferenceMedia,
    ChatVideo,
}

impl VideoSegmentWorkflowModeV1 {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::BoundaryFrames => WORKFLOW_MODE_BOUNDARY_FRAMES,
            Self::FirstFrameChain => WORKFLOW_MODE_FIRST_FRAME_CHAIN,
            Self::Storyboard => WORKFLOW_MODE_STORYBOARD,
            Self::ReferenceMedia => WORKFLOW_MODE_REFERENCE_MEDIA,
            Self::ChatVideo => WORKFLOW_MODE_CHAT_VIDEO,
        }
    }
}

#[derive(
    Debug, Clone, Copy, serde::Serialize, serde::Deserialize, schemars::JsonSchema, PartialEq, Eq,
)]
#[serde(rename_all = "snake_case")]
pub enum VideoSegmentGuidanceModeV1 {
    SystemFrames,
    Keyframes,
    Continuity,
    None,
}

impl VideoSegmentGuidanceModeV1 {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SystemFrames => "system_frames",
            Self::Keyframes => "keyframes",
            Self::Continuity => "continuity",
            Self::None => "none",
        }
    }
}

#[derive(
    Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, PartialEq, Eq,
)]
#[serde(deny_unknown_fields)]
pub struct VideoReferenceMediaBindingV1 {
    pub url: String,
    pub usage: String,
    pub upload_sha256: Option<String>,
    pub upload_file_name: Option<String>,
    pub upload_mime_type: Option<String>,
}

#[derive(
    Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, PartialEq, Eq,
)]
#[serde(deny_unknown_fields)]
pub struct VideoReferenceMediaRecipeV1 {
    pub prompt: String,
    pub image_rows: Vec<VideoReferenceMediaBindingV1>,
    pub video_rows: Vec<VideoReferenceMediaBindingV1>,
    pub audio_rows: Vec<VideoReferenceMediaBindingV1>,
}

#[derive(
    Debug, Clone, serde::Serialize, serde::Deserialize, schemars::JsonSchema, PartialEq, Eq,
)]
#[serde(deny_unknown_fields)]
pub struct VideoSegmentExecutionRecipeV1 {
    pub schema_version: String,
    pub segment_number: i32,
    pub workflow_mode: VideoSegmentWorkflowModeV1,
    pub style: String,
    pub prompt_output_language: String,
    pub duration_secs: i32,
    pub duration_secs_options: Vec<i32>,
    pub aspect_ratio: String,
    pub reference_image_ids: Vec<i64>,
    pub video_library_image_ids: Vec<i64>,
    pub guidance_mode: VideoSegmentGuidanceModeV1,
    pub guidance_reference_image_ids: Vec<i64>,
    pub continuity_source_video_id: Option<i64>,
    pub reference_media: Option<VideoReferenceMediaRecipeV1>,
    pub adopted_video_id: Option<i64>,
}

impl VideoSegmentExecutionRecipeV1 {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != VIDEO_SEGMENT_EXECUTION_RECIPE_V1 {
            return Err(format!(
                "片段执行方案 schema_version 必须为 {VIDEO_SEGMENT_EXECUTION_RECIPE_V1}"
            ));
        }
        if self.segment_number <= 0 {
            return Err("片段执行方案 segment_number 必须为正整数".to_string());
        }
        if self.style.trim().is_empty() {
            return Err("片段执行方案 style 不能为空".to_string());
        }
        if !matches!(self.prompt_output_language.as_str(), "zh_cn" | "en") {
            return Err("片段执行方案 prompt_output_language 仅支持 zh_cn 或 en".to_string());
        }
        if self.duration_secs <= 0 {
            return Err("片段执行方案 duration_secs 必须为正整数".to_string());
        }
        if self.duration_secs_options.is_empty()
            || !self.duration_secs_options.contains(&self.duration_secs)
            || self.duration_secs_options.iter().any(|value| *value <= 0)
        {
            return Err(
                "片段执行方案 duration_secs_options 必须包含默认时长且全部为正整数".to_string(),
            );
        }
        if self.aspect_ratio.parse::<VideoAspectRatio>().is_err() {
            return Err(format!(
                "片段执行方案 aspect_ratio 无效：{}，必须是 {}",
                self.aspect_ratio,
                VideoAspectRatio::supported_values()
            ));
        }
        validate_positive_unique_ids(&self.reference_image_ids, "reference_image_ids")?;
        validate_positive_unique_ids(&self.video_library_image_ids, "video_library_image_ids")?;
        validate_positive_unique_ids(
            &self.guidance_reference_image_ids,
            "guidance_reference_image_ids",
        )?;
        if self
            .continuity_source_video_id
            .is_some_and(|video_id| video_id <= 0)
        {
            return Err("片段执行方案 continuity_source_video_id 必须为正整数".to_string());
        }
        if self.adopted_video_id.is_some_and(|video_id| video_id <= 0) {
            return Err("片段执行方案 adopted_video_id 必须为正整数".to_string());
        }
        if matches!(
            self.workflow_mode,
            VideoSegmentWorkflowModeV1::ReferenceMedia | VideoSegmentWorkflowModeV1::ChatVideo
        ) && self.guidance_mode != VideoSegmentGuidanceModeV1::None
        {
            return Err("参考媒体或 Chat 方案不能同时启用图片或续接引导".to_string());
        }
        if self.guidance_mode == VideoSegmentGuidanceModeV1::Continuity
            && self.workflow_mode != VideoSegmentWorkflowModeV1::FirstFrameChain
        {
            return Err("续接引导只适用于首帧续接方案".to_string());
        }
        Ok(())
    }

    /// recipe 可以先保存为尚未配齐的草稿；真正生成前再检查必需媒体。
    pub fn validate_generation_ready(&self) -> Result<(), String> {
        self.validate()?;
        match self.guidance_mode {
            VideoSegmentGuidanceModeV1::Keyframes
                if self.guidance_reference_image_ids.is_empty() =>
            {
                Err("关键帧引导至少需要一张参考图".to_string())
            }
            VideoSegmentGuidanceModeV1::Continuity if self.continuity_source_video_id.is_none() => {
                Err("续接引导必须设置来源视频".to_string())
            }
            _ => Ok(()),
        }
    }

    pub fn validate_for_plan(
        &self,
        segment: &VideoSegmentPlanV2,
        plan_style: &str,
    ) -> Result<(), String> {
        self.validate()?;
        if self.segment_number != segment.segment_number {
            return Err(format!(
                "片段执行方案 segment_number {} 与规划片段 {} 不一致",
                self.segment_number, segment.segment_number
            ));
        }
        if self.style.trim() != plan_style.trim() {
            return Err(format!(
                "片段 {} 的风格属于共享片段规划；修改风格后请重新生成片段规划",
                self.segment_number
            ));
        }
        if self.duration_secs != segment.duration_secs
            || self.duration_secs_options.as_slice() != [segment.duration_secs]
        {
            return Err(format!(
                "片段 {} 的时长属于共享片段规划；修改时长后请重新生成片段规划",
                self.segment_number
            ));
        }
        if self.aspect_ratio != segment.aspect_ratio {
            return Err(format!(
                "片段 {} 的画幅属于共享片段规划；修改画幅后请重新生成片段规划",
                self.segment_number
            ));
        }
        if let Some(director) = &segment.director_constraints {
            if director.image_ids().iter().any(|id| {
                !self.reference_image_ids.contains(id)
                    && !self.guidance_reference_image_ids.contains(id)
            }) {
                return Err(format!(
                    "镜头 {} 必须保留导演编排绑定的图片",
                    director.segment_id
                ));
            }
        }
        Ok(())
    }

    /// Check active media at execution time, after the user has configured the recipe.
    pub fn validate_director_generation(&self, segment: &VideoSegmentPlanV2) -> Result<(), String> {
        self.validate_generation_ready()?;
        let Some(director) = &segment.director_constraints else {
            return Ok(());
        };
        let references = self.provider_reference_image_ids();
        if director
            .image_ids()
            .iter()
            .any(|id| !references.contains(id))
        {
            return Err(
                "当前引导方式未使用全部导演图片；请选择 Chat、关键帧或系统帧引导并保留绑定图片"
                    .into(),
            );
        }
        if self.workflow_mode == VideoSegmentWorkflowModeV1::BoundaryFrames
            && director.end_frame.is_none()
        {
            return Err(
                "首尾帧方式需要在导演编排中显式关联目标尾帧，不能把下一镜头的起点自动作为尾帧"
                    .into(),
            );
        }
        if self.guidance_mode == VideoSegmentGuidanceModeV1::Continuity
            && !director.image_ids().is_empty()
        {
            return Err(
                "导演镜头已绑定图片，请使用关键帧引导；视频续接会以另一段视频的尾帧替换当前起点"
                    .into(),
            );
        }
        Ok(())
    }

    /// 只覆盖会影响提示词或媒体生成结果的输入；采用哪个候选视频不属于生成输入。
    pub fn generation_fingerprint(&self) -> Result<String, String> {
        recipe_fingerprint(&self.canonical_generation_recipe()?)
    }

    /// 帧提示词和帧图片不依赖当前激活的视频引导配置，切换引导后仍可复用。
    pub fn frame_generation_fingerprint(&self) -> Result<String, String> {
        let mut recipe = self.canonical_generation_recipe()?;
        recipe.guidance_mode = VideoSegmentGuidanceModeV1::None;
        recipe.guidance_reference_image_ids.clear();
        recipe.continuity_source_video_id = None;
        recipe.reference_media = None;
        recipe_fingerprint(&recipe)
    }

    fn canonical_generation_recipe(&self) -> Result<Self, String> {
        self.validate()?;
        let mut recipe = self.clone();
        recipe.adopted_video_id = None;
        if recipe.workflow_mode != VideoSegmentWorkflowModeV1::ReferenceMedia {
            recipe.reference_media = None;
        }
        match recipe.guidance_mode {
            VideoSegmentGuidanceModeV1::Keyframes => {
                recipe.continuity_source_video_id = None;
            }
            VideoSegmentGuidanceModeV1::Continuity => {
                recipe.guidance_reference_image_ids.clear();
            }
            VideoSegmentGuidanceModeV1::SystemFrames | VideoSegmentGuidanceModeV1::None => {
                recipe.guidance_reference_image_ids.clear();
                recipe.continuity_source_video_id = None;
            }
        }
        Ok(recipe)
    }

    pub fn provider_reference_image_ids(&self) -> Vec<i64> {
        match (self.workflow_mode, self.guidance_mode) {
            (VideoSegmentWorkflowModeV1::ChatVideo, _) => self.reference_image_ids.clone(),
            (_, VideoSegmentGuidanceModeV1::Keyframes) => self.guidance_reference_image_ids.clone(),
            (_, VideoSegmentGuidanceModeV1::SystemFrames) => self.reference_image_ids.clone(),
            (_, VideoSegmentGuidanceModeV1::Continuity) => self.reference_image_ids.clone(),
            (_, VideoSegmentGuidanceModeV1::None) => Vec::new(),
        }
    }
}

fn recipe_fingerprint(recipe: &VideoSegmentExecutionRecipeV1) -> Result<String, String> {
    fingerprint_without_segment_number(recipe, "片段执行方案")
}

pub fn video_segment_content_fingerprint(segment: &VideoSegmentPlanV2) -> Result<String, String> {
    fingerprint_without_segment_number(segment, "视频片段内容")
}

fn fingerprint_without_segment_number<T: Serialize>(
    value: &T,
    context: &str,
) -> Result<String, String> {
    let mut value =
        serde_json::to_value(value).map_err(|error| format!("序列化{context}指纹失败：{error}"))?;
    value
        .as_object_mut()
        .ok_or_else(|| format!("{context}指纹输入必须是 JSON 对象"))?
        .remove("segment_number");
    let bytes =
        serde_json::to_vec(&value).map_err(|error| format!("序列化{context}指纹失败：{error}"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn validate_positive_unique_ids(ids: &[i64], field: &str) -> Result<(), String> {
    let mut seen = std::collections::HashSet::new();
    for id in ids {
        if *id <= 0 {
            return Err(format!("片段执行方案 {field} 只能包含正整数"));
        }
        if !seen.insert(*id) {
            return Err(format!("片段执行方案 {field} 包含重复 ID {id}"));
        }
    }
    Ok(())
}
