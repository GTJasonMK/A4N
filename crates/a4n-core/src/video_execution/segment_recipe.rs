use std::collections::{BTreeSet, HashMap};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};

use super::VideoSegmentPlanV2;

pub const VIDEO_SEGMENT_EXECUTION_RECIPE_V1: &str = "video_segment_execution_recipe_v1";
pub const VIDEO_SEGMENT_RECIPES_METADATA_KEY: &str = "segment_recipes";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
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
            Self::BoundaryFrames => "boundary_frames",
            Self::FirstFrameChain => "first_frame_chain",
            Self::Storyboard => "storyboard",
            Self::ReferenceMedia => "reference_media",
            Self::ChatVideo => "chat_video",
        }
    }

    pub fn default_guidance_mode(self, segment_number: i32) -> VideoSegmentGuidanceModeV1 {
        match self {
            Self::BoundaryFrames | Self::Storyboard => VideoSegmentGuidanceModeV1::SystemFrames,
            Self::FirstFrameChain if segment_number > 1 => VideoSegmentGuidanceModeV1::Continuity,
            Self::FirstFrameChain => VideoSegmentGuidanceModeV1::SystemFrames,
            Self::ReferenceMedia | Self::ChatVideo => VideoSegmentGuidanceModeV1::None,
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
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

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VideoReferenceMediaBindingV1 {
    pub url: String,
    pub usage: String,
    pub upload_sha256: Option<String>,
    pub upload_file_name: Option<String>,
    pub upload_mime_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct VideoReferenceMediaRecipeV1 {
    pub prompt: String,
    pub image_rows: Vec<VideoReferenceMediaBindingV1>,
    pub video_rows: Vec<VideoReferenceMediaBindingV1>,
    pub audio_rows: Vec<VideoReferenceMediaBindingV1>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema, PartialEq, Eq)]
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
        if self.aspect_ratio.trim().is_empty() {
            return Err("片段执行方案 aspect_ratio 不能为空".to_string());
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
        if self.workflow_mode == VideoSegmentWorkflowModeV1::FirstFrameChain
            && self.segment_number > 1
            && self.guidance_mode == VideoSegmentGuidanceModeV1::SystemFrames
        {
            return Err(
                "首帧续接的后续片段必须选择续接视频、关键帧或纯文本，不能隐式猜测系统首帧"
                    .to_string(),
            );
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
    let bytes = serde_json::to_vec(recipe)
        .map_err(|error| format!("序列化片段执行方案指纹失败：{error}"))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

pub fn propagate_adopted_video_change(
    recipes: &mut [VideoSegmentExecutionRecipeV1],
    target_segment_number: i32,
    previous_adopted_video_id: Option<i64>,
    next_adopted_video_id: Option<i64>,
) {
    let Some(previous_video_id) = previous_adopted_video_id else {
        return;
    };
    if previous_adopted_video_id == next_adopted_video_id {
        return;
    }

    let mut dependent_indexes = recipes
        .iter()
        .enumerate()
        .filter_map(|(index, recipe)| {
            (recipe.segment_number > target_segment_number)
                .then_some((recipe.segment_number, index))
        })
        .collect::<Vec<_>>();
    dependent_indexes.sort_unstable_by_key(|(segment_number, _)| *segment_number);

    let mut replacements = HashMap::from([(previous_video_id, next_adopted_video_id)]);
    for (_, index) in dependent_indexes {
        let dependent = &mut recipes[index];
        let Some(source_video_id) = dependent.continuity_source_video_id else {
            continue;
        };
        let Some(replacement) = replacements.get(&source_video_id).copied() else {
            continue;
        };
        dependent.continuity_source_video_id = replacement;
        if let Some(invalidated_video_id) = dependent.adopted_video_id.take() {
            replacements.insert(invalidated_video_id, None);
        }
    }
}

pub fn default_video_segment_recipes(
    segments: &[VideoSegmentPlanV2],
    style: &str,
    prompt_output_language: &str,
) -> Result<Vec<VideoSegmentExecutionRecipeV1>, String> {
    let style = style.trim();
    if style.is_empty() {
        return Err("共享片段规划 style 不能为空".to_string());
    }
    let recipes = segments
        .iter()
        .map(|segment| VideoSegmentExecutionRecipeV1 {
            schema_version: VIDEO_SEGMENT_EXECUTION_RECIPE_V1.to_string(),
            segment_number: segment.segment_number,
            workflow_mode: VideoSegmentWorkflowModeV1::ChatVideo,
            style: style.to_string(),
            prompt_output_language: prompt_output_language.to_string(),
            duration_secs: segment.duration_secs,
            duration_secs_options: vec![segment.duration_secs],
            aspect_ratio: segment.aspect_ratio.clone(),
            reference_image_ids: Vec::new(),
            video_library_image_ids: Vec::new(),
            guidance_mode: VideoSegmentWorkflowModeV1::ChatVideo
                .default_guidance_mode(segment.segment_number),
            guidance_reference_image_ids: Vec::new(),
            continuity_source_video_id: None,
            reference_media: None,
            adopted_video_id: None,
        })
        .collect::<Vec<_>>();
    validate_video_segment_recipes_for_plan(&recipes, segments, style)?;
    Ok(recipes)
}

pub fn carry_forward_video_segment_recipes(
    previous_segments: &[VideoSegmentPlanV2],
    previous_recipes: &[VideoSegmentExecutionRecipeV1],
    next_segments: &[VideoSegmentPlanV2],
    style: &str,
    prompt_output_language: &str,
) -> Result<Vec<VideoSegmentExecutionRecipeV1>, String> {
    let mut recipes = default_video_segment_recipes(next_segments, style, prompt_output_language)?;
    for (next, recipe) in next_segments.iter().zip(&mut recipes) {
        let Some((_, previous)) = previous_segments
            .iter()
            .zip(previous_recipes)
            .find(|(segment, _)| same_segment_recipe_anchor(segment, next))
        else {
            continue;
        };
        *recipe = previous.clone();
        recipe.segment_number = next.segment_number;
        recipe.style = style.trim().to_string();
        recipe.prompt_output_language = prompt_output_language.to_string();
        recipe.duration_secs = next.duration_secs;
        recipe.duration_secs_options = vec![next.duration_secs];
        recipe.aspect_ratio.clone_from(&next.aspect_ratio);
        recipe.continuity_source_video_id = None;
        recipe.adopted_video_id = None;
        if matches!(
            recipe.workflow_mode,
            VideoSegmentWorkflowModeV1::ReferenceMedia | VideoSegmentWorkflowModeV1::ChatVideo
        ) {
            recipe.guidance_mode = VideoSegmentGuidanceModeV1::None;
        } else if recipe.workflow_mode == VideoSegmentWorkflowModeV1::FirstFrameChain
            && next.segment_number > 1
            && recipe.guidance_mode == VideoSegmentGuidanceModeV1::SystemFrames
        {
            recipe.guidance_mode = VideoSegmentGuidanceModeV1::Continuity;
        }
    }
    validate_video_segment_recipes_for_plan(&recipes, next_segments, style)?;
    Ok(recipes)
}

fn same_segment_recipe_anchor(left: &VideoSegmentPlanV2, right: &VideoSegmentPlanV2) -> bool {
    left.source_shot_id == right.source_shot_id
        && left.segment_index_in_shot == right.segment_index_in_shot
        && left.shot_segment_count == right.shot_segment_count
        && left.covered_beat_ids == right.covered_beat_ids
        && left.source_span_ids == right.source_span_ids
}

pub fn video_segment_recipes_from_metadata(
    metadata: Option<&Value>,
) -> Result<Vec<VideoSegmentExecutionRecipeV1>, String> {
    let Some(value) = metadata
        .and_then(Value::as_object)
        .and_then(|object| object.get(VIDEO_SEGMENT_RECIPES_METADATA_KEY))
    else {
        return Ok(Vec::new());
    };
    let mut recipes = serde_json::from_value::<Vec<VideoSegmentExecutionRecipeV1>>(value.clone())
        .map_err(|error| format!("片段执行方案 metadata 无效：{error}"))?;
    recipes.sort_by_key(|recipe| recipe.segment_number);
    let mut seen = BTreeSet::new();
    for recipe in &recipes {
        recipe.validate()?;
        if !seen.insert(recipe.segment_number) {
            return Err(format!(
                "片段执行方案 segment_number {} 重复",
                recipe.segment_number
            ));
        }
    }
    Ok(recipes)
}

pub fn video_segment_recipes_for_plan_from_metadata(
    metadata: Option<&Value>,
    plan_style: &str,
) -> Result<Vec<VideoSegmentExecutionRecipeV1>, String> {
    let metadata = metadata.ok_or_else(|| "片段规划 metadata_json 必须为 JSON 对象".to_string())?;
    let object = metadata
        .as_object()
        .ok_or_else(|| "片段规划 metadata_json 必须为 JSON 对象".to_string())?;
    let segments = object
        .get("segments")
        .cloned()
        .ok_or_else(|| "片段规划 metadata_json 缺少 segments".to_string())?;
    let segments = serde_json::from_value::<Vec<VideoSegmentPlanV2>>(segments)
        .map_err(|error| format!("片段规划 segments 无效：{error}"))?;
    let recipes = video_segment_recipes_from_metadata(Some(metadata))?;
    validate_video_segment_recipes_for_plan(&recipes, &segments, plan_style)?;
    Ok(recipes)
}

pub fn validate_video_segment_recipes_for_plan(
    recipes: &[VideoSegmentExecutionRecipeV1],
    segments: &[VideoSegmentPlanV2],
    plan_style: &str,
) -> Result<(), String> {
    if recipes.len() != segments.len() {
        return Err(format!(
            "片段规划必须为每个片段保存且只保存一个执行方案：规划 {} 个片段，实际 {} 个方案",
            segments.len(),
            recipes.len()
        ));
    }
    for segment in segments {
        let recipe = recipes
            .iter()
            .find(|recipe| recipe.segment_number == segment.segment_number)
            .ok_or_else(|| format!("片段 {} 缺少执行方案", segment.segment_number))?;
        recipe.validate_for_plan(segment, plan_style)?;
    }
    Ok(())
}

pub fn video_segment_recipe_from_metadata(
    metadata: Option<&Value>,
    segment_number: i32,
) -> Result<Option<VideoSegmentExecutionRecipeV1>, String> {
    Ok(video_segment_recipes_from_metadata(metadata)?
        .into_iter()
        .find(|recipe| recipe.segment_number == segment_number))
}

pub fn update_video_segment_recipe_metadata(
    metadata: Option<&Value>,
    plan_style: &str,
    recipe: VideoSegmentExecutionRecipeV1,
) -> Result<(Value, Vec<VideoSegmentExecutionRecipeV1>), String> {
    let metadata = metadata.ok_or_else(|| "片段规划缺少 metadata_json".to_string())?;
    let object = metadata
        .as_object()
        .ok_or_else(|| "片段规划 metadata_json 必须为 JSON 对象".to_string())?;
    let segments = serde_json::from_value::<Vec<VideoSegmentPlanV2>>(
        object
            .get("segments")
            .cloned()
            .ok_or_else(|| "片段规划 metadata_json 缺少 segments".to_string())?,
    )
    .map_err(|error| format!("片段规划 segments 无效：{error}"))?;
    let mut recipes = video_segment_recipes_from_metadata(Some(metadata))?;
    validate_video_segment_recipes_for_plan(&recipes, &segments, plan_style)?;
    let index = recipes
        .iter()
        .position(|existing| existing.segment_number == recipe.segment_number)
        .ok_or_else(|| format!("当前片段规划不存在片段 {}", recipe.segment_number))?;
    let existing = recipes[index].clone();
    if let Some(source_video_id) = recipe.continuity_source_video_id
        && !recipes.iter().any(|candidate| {
            candidate.segment_number < recipe.segment_number
                && candidate.adopted_video_id == Some(source_video_id)
        })
    {
        return Err(format!(
            "续接来源视频 #{source_video_id} 必须是当前规划更早片段已采用的视频"
        ));
    }
    let previous_adopted_video_id = existing.adopted_video_id;
    let next_adopted_video_id = recipe.adopted_video_id;
    let target_segment_number = recipe.segment_number;
    recipes[index] = recipe;
    propagate_adopted_video_change(
        &mut recipes,
        target_segment_number,
        previous_adopted_video_id,
        next_adopted_video_id,
    );
    validate_video_segment_recipes_for_plan(&recipes, &segments, plan_style)?;
    let mut updated_metadata = metadata.clone();
    updated_metadata
        .as_object_mut()
        .expect("metadata object was validated above")
        .insert(
            VIDEO_SEGMENT_RECIPES_METADATA_KEY.to_string(),
            serde_json::to_value(&recipes)
                .map_err(|error| format!("序列化片段执行方案失败：{error}"))?,
        );
    Ok((updated_metadata, recipes))
}

fn validate_positive_unique_ids(ids: &[i64], field: &str) -> Result<(), String> {
    let mut seen = BTreeSet::new();
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

#[cfg(test)]
mod tests {
    use super::*;

    fn recipe(segment_number: i32) -> VideoSegmentExecutionRecipeV1 {
        VideoSegmentExecutionRecipeV1 {
            schema_version: VIDEO_SEGMENT_EXECUTION_RECIPE_V1.to_string(),
            segment_number,
            workflow_mode: VideoSegmentWorkflowModeV1::ChatVideo,
            style: "cinematic".to_string(),
            prompt_output_language: "zh_cn".to_string(),
            duration_secs: 6,
            duration_secs_options: vec![6],
            aspect_ratio: "16:9".to_string(),
            reference_image_ids: Vec::new(),
            video_library_image_ids: Vec::new(),
            guidance_mode: VideoSegmentGuidanceModeV1::None,
            guidance_reference_image_ids: Vec::new(),
            continuity_source_video_id: None,
            reference_media: None,
            adopted_video_id: None,
        }
    }

    #[test]
    fn recipe_upsert_is_scoped_to_current_plan_segment() {
        assert_eq!(
            update_video_segment_recipe_metadata(None, "cinematic", recipe(1)).unwrap_err(),
            "片段规划缺少 metadata_json",
        );
        let segment = serde_json::json!({
            "schema_version": "video_segment_plan_v2",
            "segment_number": 1,
            "source_shot_id": "shot-1",
            "segment_index_in_shot": 1,
            "shot_segment_count": 1,
            "covered_beat_ids": [],
            "title": "片段 1",
            "scene_summary": "场景",
            "blocking_summary": "调度",
            "spatial_anchors": [],
            "emotion_action_goals": [],
            "video_intent": {"hook":"h","audience_effect":"a","rhythm":"r","visual_priority":"v"},
            "emotional_arc": {"start":"s","turn":"t","end":"e","function":"f"},
            "performance_plan": {"movement_path":"m","body_performance":"b","gaze_and_hands":"g","pause_or_reaction":"p","camera_relationship":"c","time_blocks":[],"action_script":[]},
            "material_coverage": {"required_entities":[],"missing_or_weak_references":[],"risk_note":"r"},
            "transition_to_next": null,
            "source_span_ids": ["span-1"],
            "duration_secs": 6,
            "aspect_ratio": "16:9",
            "reference_image_ids": []
        });
        let mut segment_two = segment.clone();
        let segment_two_object = segment_two.as_object_mut().unwrap();
        segment_two_object.insert("segment_number".to_string(), Value::from(2));
        segment_two_object.insert("title".to_string(), Value::from("片段 2"));
        let metadata = serde_json::json!({
            "segments": [segment, segment_two],
            "segment_recipes": [recipe(1), recipe(2)]
        });
        let (metadata, _) =
            update_video_segment_recipe_metadata(Some(&metadata), "cinematic", recipe(2)).unwrap();
        assert_eq!(
            video_segment_recipe_from_metadata(Some(&metadata), 2)
                .unwrap()
                .unwrap()
                .workflow_mode,
            VideoSegmentWorkflowModeV1::ChatVideo
        );
        let mut adopted = recipe(2);
        adopted.adopted_video_id = Some(9);
        let (metadata, _) =
            update_video_segment_recipe_metadata(Some(&metadata), "cinematic", adopted).unwrap();
        let (metadata, _) =
            update_video_segment_recipe_metadata(Some(&metadata), "cinematic", recipe(2)).unwrap();
        assert_eq!(
            video_segment_recipe_from_metadata(Some(&metadata), 2)
                .unwrap()
                .unwrap()
                .adopted_video_id,
            None
        );
        assert!(
            update_video_segment_recipe_metadata(Some(&metadata), "cinematic", recipe(3)).is_err()
        );

        let mut wrong_style = recipe(2);
        wrong_style.style = "watercolor".to_string();
        assert!(
            update_video_segment_recipe_metadata(Some(&metadata), "cinematic", wrong_style)
                .unwrap_err()
                .contains("风格属于共享片段规划")
        );
        let mut wrong_duration = recipe(2);
        wrong_duration.duration_secs = 10;
        wrong_duration.duration_secs_options = vec![10];
        assert!(
            update_video_segment_recipe_metadata(Some(&metadata), "cinematic", wrong_duration)
                .unwrap_err()
                .contains("时长属于共享片段规划")
        );
        let mut wrong_aspect_ratio = recipe(2);
        wrong_aspect_ratio.aspect_ratio = "9:16".to_string();
        assert!(
            update_video_segment_recipe_metadata(Some(&metadata), "cinematic", wrong_aspect_ratio,)
                .unwrap_err()
                .contains("画幅属于共享片段规划")
        );
    }

    #[test]
    fn adopted_video_does_not_change_generation_fingerprint() {
        let recipe = recipe(1);
        let mut adopted = recipe.clone();
        adopted.adopted_video_id = Some(9);
        assert_eq!(
            recipe.generation_fingerprint().unwrap(),
            adopted.generation_fingerprint().unwrap()
        );
    }

    #[test]
    fn later_first_frame_segment_rejects_hidden_system_frame_fallback() {
        let mut recipe = recipe(2);
        recipe.workflow_mode = VideoSegmentWorkflowModeV1::FirstFrameChain;
        recipe.guidance_mode = VideoSegmentGuidanceModeV1::SystemFrames;
        assert!(recipe.validate().is_err());
        recipe.guidance_mode = VideoSegmentGuidanceModeV1::Continuity;
        recipe.continuity_source_video_id = Some(7);
        assert!(recipe.validate_generation_ready().is_ok());
    }

    #[test]
    fn continuity_keeps_optional_provider_reference_images() {
        let mut recipe = recipe(2);
        recipe.workflow_mode = VideoSegmentWorkflowModeV1::FirstFrameChain;
        recipe.guidance_mode = VideoSegmentGuidanceModeV1::Continuity;
        recipe.continuity_source_video_id = Some(7);
        recipe.reference_image_ids = vec![11, 12];

        assert_eq!(recipe.provider_reference_image_ids(), vec![11, 12]);
    }

    #[test]
    fn adopted_video_change_invalidates_the_dependent_continuity_chain() {
        let chain_recipe = |segment_number, continuity_source_video_id, adopted_video_id| {
            let mut recipe = recipe(segment_number);
            recipe.workflow_mode = VideoSegmentWorkflowModeV1::FirstFrameChain;
            recipe.guidance_mode = if segment_number == 1 {
                VideoSegmentGuidanceModeV1::None
            } else {
                VideoSegmentGuidanceModeV1::Continuity
            };
            recipe.continuity_source_video_id = continuity_source_video_id;
            recipe.adopted_video_id = adopted_video_id;
            recipe
        };
        let mut recipes = vec![
            chain_recipe(3, Some(22), Some(33)),
            chain_recipe(1, None, Some(11)),
            chain_recipe(4, None, Some(44)),
            chain_recipe(2, Some(11), Some(22)),
        ];

        propagate_adopted_video_change(&mut recipes, 1, Some(11), Some(12));

        let segment = |number| {
            recipes
                .iter()
                .find(|recipe| recipe.segment_number == number)
                .unwrap()
        };
        assert_eq!(segment(2).continuity_source_video_id, Some(12));
        assert_eq!(segment(2).adopted_video_id, None);
        assert_eq!(segment(3).continuity_source_video_id, None);
        assert_eq!(segment(3).adopted_video_id, None);
        assert_eq!(segment(4).adopted_video_id, Some(44));
    }
}
