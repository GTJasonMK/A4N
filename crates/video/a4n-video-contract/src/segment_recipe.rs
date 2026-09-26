use std::collections::{HashMap, HashSet};

use serde_json::Value;

use super::{
    VIDEO_SEGMENT_EXECUTION_RECIPE_V1, VideoSegmentExecutionRecipeV1, VideoSegmentGuidanceModeV1,
    VideoSegmentPlanV2, VideoSegmentWorkflowModeV1, video_segment_content_fingerprint,
};

pub const VIDEO_SEGMENT_RECIPES_METADATA_KEY: &str = "segment_recipes";

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
        .map(|segment| {
            let images = segment
                .director_constraints
                .as_ref()
                .map_or_else(Vec::new, |director| director.image_ids());
            VideoSegmentExecutionRecipeV1 {
                schema_version: VIDEO_SEGMENT_EXECUTION_RECIPE_V1.to_string(),
                segment_number: segment.segment_number,
                workflow_mode: if segment.director_constraints.is_some() {
                    VideoSegmentWorkflowModeV1::FirstFrameChain
                } else {
                    VideoSegmentWorkflowModeV1::ChatVideo
                },
                style: style.to_string(),
                prompt_output_language: prompt_output_language.to_string(),
                duration_secs: segment.duration_secs,
                duration_secs_options: vec![segment.duration_secs],
                aspect_ratio: segment.aspect_ratio.clone(),
                reference_image_ids: images.clone(),
                video_library_image_ids: Vec::new(),
                guidance_mode: if !images.is_empty() {
                    VideoSegmentGuidanceModeV1::Keyframes
                } else {
                    VideoSegmentGuidanceModeV1::None
                },
                guidance_reference_image_ids: images,
                continuity_source_video_id: None,
                reference_media: None,
                adopted_video_id: None,
            }
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
    let previous_by_anchor = previous_segments
        .iter()
        .zip(previous_recipes)
        .enumerate()
        .fold(
            HashMap::with_capacity(previous_segments.len().min(previous_recipes.len())),
            |mut by_anchor, (index, (segment, _))| {
                by_anchor
                    .entry(video_segment_recipe_anchor_key(segment))
                    .or_insert(index);
                by_anchor
            },
        );
    for (next, recipe) in next_segments.iter().zip(&mut recipes) {
        let Some(previous_index) = previous_by_anchor
            .get(&video_segment_recipe_anchor_key(next))
            .copied()
        else {
            continue;
        };
        let previous_segment = &previous_segments[previous_index];
        // A changed director plan supplies a fresh recipe; old bindings cannot override it.
        if previous_segment.director_constraints != next.director_constraints {
            continue;
        }
        let previous = &previous_recipes[previous_index];
        *recipe = previous.clone();
        recipe.segment_number = next.segment_number;
        recipe.style = style.trim().to_string();
        recipe.prompt_output_language = prompt_output_language.to_string();
        recipe.duration_secs = next.duration_secs;
        recipe.duration_secs_options = vec![next.duration_secs];
        recipe.aspect_ratio.clone_from(&next.aspect_ratio);
        if matches!(
            recipe.workflow_mode,
            VideoSegmentWorkflowModeV1::ReferenceMedia | VideoSegmentWorkflowModeV1::ChatVideo
        ) {
            recipe.guidance_mode = VideoSegmentGuidanceModeV1::None;
        }
        if video_segment_content_fingerprint(previous_segment)?
            != video_segment_content_fingerprint(next)?
            || previous.generation_fingerprint()? != recipe.generation_fingerprint()?
        {
            recipe.adopted_video_id = None;
        }
    }
    validate_video_segment_recipes_for_plan(&recipes, next_segments, style)?;
    Ok(recipes)
}

fn video_segment_recipe_anchor_key(
    segment: &VideoSegmentPlanV2,
) -> (String, i32, i32, Vec<String>, Vec<String>) {
    (
        segment.source_shot_id.clone(),
        segment.segment_index_in_shot,
        segment.shot_segment_count,
        segment.covered_beat_ids.clone(),
        segment.source_span_ids.clone(),
    )
}

pub fn video_segments_share_recipe_anchor(
    left: &VideoSegmentPlanV2,
    right: &VideoSegmentPlanV2,
) -> bool {
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
    let mut seen = HashSet::new();
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
    let segments = crate::video_segment_plan_from_metadata(metadata)?;
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
    let mut recipes_by_number = HashMap::with_capacity(recipes.len());
    for recipe in recipes {
        if recipes_by_number
            .insert(recipe.segment_number, recipe)
            .is_some()
        {
            return Err(format!(
                "片段执行方案 segment_number {} 重复",
                recipe.segment_number
            ));
        }
    }
    for segment in segments {
        let recipe = recipes_by_number
            .get(&segment.segment_number)
            .copied()
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

pub fn resolve_video_segment_number(
    metadata: Option<&Value>,
    source_shot_id: &str,
    segment_index_in_shot: i32,
) -> Result<i32, String> {
    let source_shot_id = source_shot_id.trim();
    if source_shot_id.is_empty() || segment_index_in_shot <= 0 {
        return Err("稳定片段目标必须提供镜头 ID 和正整数镜头内序号".to_string());
    }
    let metadata = metadata.ok_or_else(|| "片段规划缺少 metadata_json".to_string())?;
    ensure_video_segment_source_current(Some(metadata), source_shot_id)?;
    let segments = crate::video_segment_plan_from_metadata(metadata)?;
    segments
        .into_iter()
        .find(|segment| {
            segment.source_shot_id == source_shot_id
                && segment.segment_index_in_shot == segment_index_in_shot
        })
        .map(|segment| segment.segment_number)
        .ok_or_else(|| {
            format!("当前片段规划不存在镜头 {source_shot_id} 的第 {segment_index_in_shot} 个片段")
        })
}

pub fn ensure_video_segment_source_current(
    metadata: Option<&Value>,
    shot_id: &str,
) -> Result<(), String> {
    if metadata
        .and_then(|value| value.get("stale_source_shot_ids"))
        .and_then(Value::as_array)
        .is_some_and(|ids| ids.iter().any(|id| id.as_str() == Some(shot_id)))
    {
        return Err(format!(
            "镜头 {shot_id} 的来源已更新，请先局部更新该镜头的片段规划"
        ));
    }
    Ok(())
}

pub fn update_video_segment_recipe_metadata(
    metadata: Option<&Value>,
    plan_style: &str,
    mut recipe: VideoSegmentExecutionRecipeV1,
) -> Result<(Value, Vec<VideoSegmentExecutionRecipeV1>), String> {
    let metadata = metadata.ok_or_else(|| "片段规划缺少 metadata_json".to_string())?;
    let segments = crate::video_segment_plan_from_metadata(metadata)?;
    let mut recipes = video_segment_recipes_from_metadata(Some(metadata))?;
    validate_video_segment_recipes_for_plan(&recipes, &segments, plan_style)?;
    let index = recipes
        .iter()
        .position(|existing| existing.segment_number == recipe.segment_number)
        .ok_or_else(|| format!("当前片段规划不存在片段 {}", recipe.segment_number))?;
    let existing = recipes[index].clone();
    if recipe.adopted_video_id.is_none() {
        recipe.adopted_video_id = existing.adopted_video_id;
    }
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

    fn segment(segment_number: i32) -> VideoSegmentPlanV2 {
        serde_json::from_value(serde_json::json!({
            "schema_version": "video_segment_plan_v2",
            "segment_number": segment_number,
            "source_shot_id": "shot-1",
            "segment_index_in_shot": 1,
            "shot_segment_count": 1,
            "covered_beat_ids": [],
            "title": format!("片段 {segment_number}"),
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
        }))
        .unwrap()
    }

    #[test]
    fn stored_plan_rejects_duplicate_targets_and_incomplete_shot_groups() {
        let first = segment(1);
        let duplicate = segment(2);
        assert!(
            crate::video_segment_plan_from_metadata(
                &serde_json::json!({"segments": [first, duplicate]})
            )
            .is_err()
        );
        let mut incomplete = segment(1);
        incomplete.shot_segment_count = 2;
        assert!(
            crate::video_segment_plan_from_metadata(&serde_json::json!({"segments": [incomplete]}))
                .is_err()
        );
    }

    #[test]
    fn recipe_upsert_is_scoped_to_current_plan_segment() {
        assert_eq!(
            update_video_segment_recipe_metadata(None, "cinematic", recipe(1)).unwrap_err(),
            "片段规划缺少 metadata_json",
        );
        let segment_one = segment(1);
        let mut segment_two = segment(2);
        segment_two.source_shot_id = "shot-2".into();
        let metadata = serde_json::json!({
            "segments": [segment_one, segment_two],
            "segment_recipes": [recipe(1), recipe(2)]
        });
        assert_eq!(
            resolve_video_segment_number(Some(&metadata), "shot-1", 1).unwrap(),
            1
        );
        assert!(
            resolve_video_segment_number(Some(&metadata), "missing", 1)
                .unwrap_err()
                .contains("不存在")
        );
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
            Some(9)
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
    fn display_number_does_not_invalidate_segment_outputs() {
        let previous_segment = segment(1);
        let mut renumbered_segment = previous_segment.clone();
        renumbered_segment.segment_number = 2;

        let mut previous_recipe = recipe(1);
        previous_recipe.adopted_video_id = Some(9);
        let carried = carry_forward_video_segment_recipes(
            std::slice::from_ref(&previous_segment),
            std::slice::from_ref(&previous_recipe),
            std::slice::from_ref(&renumbered_segment),
            "cinematic",
            "zh_cn",
        )
        .unwrap();

        assert_eq!(
            previous_recipe.generation_fingerprint().unwrap(),
            carried[0].generation_fingerprint().unwrap()
        );
        assert_eq!(
            video_segment_content_fingerprint(&previous_segment).unwrap(),
            video_segment_content_fingerprint(&renumbered_segment).unwrap()
        );
        assert_eq!(carried[0].adopted_video_id, Some(9));

        renumbered_segment.title = "内容已经变化".to_string();
        let changed = carry_forward_video_segment_recipes(
            &[previous_segment],
            &[previous_recipe],
            &[renumbered_segment],
            "cinematic",
            "zh_cn",
        )
        .unwrap();
        assert_eq!(changed[0].adopted_video_id, None);
    }

    #[test]
    fn later_first_frame_segment_allows_explicit_system_frame() {
        let mut recipe = recipe(2);
        recipe.workflow_mode = VideoSegmentWorkflowModeV1::FirstFrameChain;
        recipe.guidance_mode = VideoSegmentGuidanceModeV1::SystemFrames;
        assert!(recipe.validate_generation_ready().is_ok());

        let target_segment = segment(2);
        let carried = carry_forward_video_segment_recipes(
            std::slice::from_ref(&target_segment),
            std::slice::from_ref(&recipe),
            std::slice::from_ref(&target_segment),
            "cinematic",
            "zh_cn",
        )
        .unwrap();
        assert_eq!(
            carried[0].guidance_mode,
            VideoSegmentGuidanceModeV1::SystemFrames
        );

        recipe.guidance_mode = VideoSegmentGuidanceModeV1::Continuity;
        recipe.continuity_source_video_id = Some(7);
        recipe.adopted_video_id = Some(9);
        assert!(recipe.validate_generation_ready().is_ok());
        let carried = carry_forward_video_segment_recipes(
            std::slice::from_ref(&target_segment),
            std::slice::from_ref(&recipe),
            std::slice::from_ref(&target_segment),
            "cinematic",
            "zh_cn",
        )
        .unwrap();
        assert_eq!(
            carried[0], recipe,
            "unchanged shots retain explicit continuity and adopted videos"
        );
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
    fn director_image_bindings_can_be_overridden_in_execution_recipe() {
        use a4n_domain::chapter_visual_plan::{ChapterVisualPlanV3, SceneBindingV3};
        let mut plan = ChapterVisualPlanV3::starter();
        plan.points[1].at_ms = 6000;
        plan.points[0].image_id = Some(11);
        plan.segments[0].prompt = "推近门口".into();
        plan.segments[0].scene_binding = Some(SceneBindingV3::Scene {
            scene_profile_id: 1,
        });
        let mut segment = segment(1);
        segment.director_constraints = Some(plan.project().unwrap().remove(0));
        let mut recipe = default_video_segment_recipes(&[segment.clone()], "cinematic", "zh_cn")
            .unwrap()
            .remove(0);
        assert!(recipe.validate_director_generation(&segment).is_ok());
        let mut changed = segment.clone();
        changed.director_constraints.as_mut().unwrap().keyframes[0].image_id = Some(12);
        let carried = carry_forward_video_segment_recipes(
            &[segment.clone()],
            &[recipe.clone()],
            &[changed],
            "cinematic",
            "zh_cn",
        )
        .unwrap();
        assert_eq!(carried[0].provider_reference_image_ids(), vec![12]);
        recipe.workflow_mode = VideoSegmentWorkflowModeV1::BoundaryFrames;
        recipe.guidance_mode = VideoSegmentGuidanceModeV1::None;
        assert!(recipe.validate_for_plan(&segment, "cinematic").is_ok());
        assert!(
            recipe
                .validate_director_generation(&segment)
                .unwrap_err()
                .contains("显式关联目标尾帧")
        );
        recipe.workflow_mode = VideoSegmentWorkflowModeV1::ChatVideo;
        recipe.reference_image_ids.clear();
        recipe.guidance_reference_image_ids.clear();
        assert!(recipe.validate_for_plan(&segment, "cinematic").is_ok());
        assert!(recipe.validate_director_generation(&segment).is_ok());

        recipe.workflow_mode = VideoSegmentWorkflowModeV1::FirstFrameChain;
        assert!(recipe.validate_director_generation(&segment).is_ok());
        recipe.guidance_mode = VideoSegmentGuidanceModeV1::Continuity;
        recipe.continuity_source_video_id = Some(7);
        assert!(recipe.validate_director_generation(&segment).is_ok());
        let director = segment.director_constraints.as_mut().unwrap();
        assert_eq!(director.video_duration_secs(), Ok(6));
        director.end_ms = 6500;
        assert!(
            director
                .video_duration_secs()
                .unwrap_err()
                .contains("不会自动舍入")
        );
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
