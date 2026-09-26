use serde_json::{Value, json};

use a4n_error::AppError;

use super::{
    VideoPlanningPromptContext, render_strict_json_user_prompt, rule_text, rule_value,
    video_builder_rules, video_prompt_task,
};

pub(in crate::video) async fn build_segment_plan_for_shot_user(
    context: VideoPlanningPromptContext<'_>,
    current_shot: &Value,
) -> Result<String, AppError> {
    let rules = video_builder_rules(context.prompt_renderer, context.project_id).await?;
    let task = video_prompt_task(
        &rules,
        "chapter_video_segment_plan",
        "task_instruction_chapter_video_segment_plan",
    )?;
    let user_instruction = rule_text(&rules, "user_instruction_chapter_video_segment_plan")?;
    let directed = current_shot
        .get("director_constraints")
        .filter(|value| !value.is_null());
    let duration_secs = if directed.is_some() {
        current_shot
            .get("segment_duration_secs")
            .and_then(Value::as_i64)
            .ok_or_else(|| AppError::InvalidArgument("导演镜头缺少计划时长".into()))?
    } else {
        i64::from(context.duration_secs)
    };
    let mut payload = json!({
        "task": task,
        "chapter": {
            "summary": context.chapter_summary.map(str::trim).filter(|s| !s.is_empty()),
        },
        "current_shot": current_shot,
        "constraints": {
            "style": context.style,
            "selected_duration_secs": duration_secs,
            "aspect_ratio": context.aspect_ratio,
            "style_preservation_rule": rule_value(&rules, "style_preservation_rule")?,
            "time_budget_rule": rule_value(&rules, "time_budget_rule")?,
            "safe_visual_rewrite_rule": rule_value(&rules, "safe_visual_rewrite_rule")?,
            "source_span_rule": rule_value(&rules, "segment_plan_source_span_rule")?,
            "shooting_script_projection_rule": rule_value(&rules, "segment_plan_shooting_script_projection_rule")?,
            "beat_mapping_rule": rule_value(&rules, "segment_plan_beat_mapping_rule")?,
            "shot_mapping_rule": rule_value(&rules, "segment_plan_shot_mapping_rule")?,
            "duration_selection_rule": rule_value(&rules, "segment_plan_duration_selection_rule")?
        },
        "output_contract": {
            "format": "json_object",
            "prompt_name": "chapter_video_segment_plan",
            "required_root_field": "segments",
            "required_segment_fields": [
                "source_shot_id",
                "covered_beat_ids",
                "title",
                "scene_summary",
                "blocking_summary",
                "spatial_anchors",
                "emotion_action_goals",
                "video_intent",
                "emotional_arc",
                "performance_plan",
                "material_coverage"
            ],
            "derived_segment_fields": [
                "segment_number",
                "segment_index_in_shot",
                "shot_segment_count",
                "source_span_ids",
                "duration_secs",
                "aspect_ratio"
            ],
            "segment_count_rule": "segments 数量必须至少等于 current_shot.minimum_readable_segment_count；优先按 current_shot.recommended_segment_count 规划，但当最低可读数量更大时必须采用更大的数量。当最终 segments 数量大于 current_shot.action_beats 数量时，必须把耗时较长的同一动作节拍按连续时间阶段拆到相邻 segments；这些相邻 segments 可以重复引用同一 beat_id，这是动作节拍通常只能覆盖一次的唯一例外。除此之外，同一 beat_id 只能由一个 segment 覆盖；每个单独 segment 的 covered_beat_ids 内仍不得重复 ID。不得为了减少调用把必需动作节拍压进观众无法读清的单个片段。",
            "segment_number_rule": "只为 current_shot 生成 1 个或多个连续 segments；source_shot_id 必须等于 current_shot.source_shot_id；covered_beat_ids 必须使用 current_shot.action_beats[].beat_id 的稳定字符串 ID。不要输出 segment_number、segment_index_in_shot、shot_segment_count、source_span_ids、duration_secs、aspect_ratio 或 reference_image_ids；后端会按数组顺序、拍摄剧本投影、用户选择时长和用户画幅推导结构字段。片段规划只做素材需求审计，不绑定具体参考图。",
            "performance_action_script_rule": "当 current_shot.participants 非空时，emotion_action_goals 和 performance_plan.action_script 都必须非空，character 必须来自 participants；action_script 每个对象必须同时包含 character、body、movement、hands、gaze、timing 六个非空字符串字段，禁止空字符串、null、对象、数组或省略字段。hands 没有明显动作时也要写“手未接触道具”或“双手保持稳定”等可见静止状态。仅当 participants 为空的纯环境镜头时，这两个数组必须都返回 []，不要把场景、地点、天气或光影伪装成人物；环境动态写入 performance_plan.time_blocks。"
        }
    });
    if let Some(shot) = payload["current_shot"].as_object_mut() {
        shot.remove("source_span_ids");
        if let Some(beats) = shot.get_mut("action_beats").and_then(Value::as_array_mut) {
            for beat in beats {
                if let Some(beat) = beat.as_object_mut() {
                    beat.remove("source_span_ids");
                }
            }
        }
    }
    if directed.is_some() {
        payload["output_contract"]["segment_count_rule"] = json!(
            "当前镜头由导演编排固定，只能输出一个 segment；不能拆分、合并或改变时长。完整保留 director_constraints 中的 prompt、场景关系与关键帧约束。"
        );
        payload["output_contract"]["segment_number_rule"] = json!(
            "只为 current_shot 输出 1 个 segment，source_shot_id 原样保留，covered_beat_ids 按顺序覆盖本镜头必需节拍且不得重复；不要输出 derived_segment_fields 或 reference_image_ids。"
        );
        let constraints = payload["constraints"]
            .as_object_mut()
            .expect("constraints object");
        constraints.remove("time_budget_rule");
        constraints.remove("duration_selection_rule");
        let director: a4n_domain::chapter_visual_plan::VisualSegmentProjectionV3 =
            serde_json::from_value(directed.cloned().expect("directed constraint"))?;
        payload["constraints"]["aspect_ratio"] = json!(
            director
                .frame
                .aspect_ratio()
                .map_err(AppError::InvalidArgument)?
        );
    }
    render_strict_json_user_prompt(
        context.prompt_renderer,
        context.project_id,
        &user_instruction,
        &payload,
    )
    .await
}
