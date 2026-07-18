use std::collections::HashMap;

fn stored_segment_plan_fixture(source_span_id: &str) -> Value {
    json!({
        "segments": [
            {
                "schema_version": "video_segment_plan_v2",
                "segment_number": 1,
                "source_shot_id": "shot-1",
                "segment_index_in_shot": 1,
                "shot_segment_count": 1,
                "covered_beat_ids": ["beat-1", "beat-2"],
                "title": "入殿",
                "scene_summary": "小宦官推门入殿，发现异常",
                "blocking_summary": "小宦官从殿门外进入殿内，门是固定空间锚点，人物始终在画面中央偏左",
                "spatial_anchors": [{"anchor": "殿门", "usage": "约束入殿起点和移动方向"}],
                "emotion_action_goals": [{"character": "小宦官", "emotion": "紧张", "visible_action": "放慢脚步并收紧肩膀"}],
                "video_intent": {"hook": "推门入殿后察觉异常", "audience_effect": "让观众感到紧张和不安", "rhythm": "缓慢进入后短暂停顿", "visual_priority": "小宦官推门、入殿、发现异常的动作"},
                "emotional_arc": {"start": "谨慎", "turn": "察觉殿内异常", "end": "紧张压住恐惧", "function": "场景建立"},
                "performance_plan": test_segment_performance_plan(
                    4,
                    "从殿门外中央偏左推门进入殿内，在暗廊入口前停住。",
                    "小宦官推门入殿，放慢脚步后停住观察。",
                    "先看殿门内侧，再落向右后方暗廊",
                    "平视中景缓慢推进，保持殿门和暗廊同框。",
                ),
                "material_coverage": {"required_entities": ["小宦官", "殿门", "殿内空间"], "missing_or_weak_references": ["小宦官", "殿门", "殿内空间"], "risk_note": "需要用文字明确人物服饰、殿门和殿内空间"},
                "transition_to_next": null,
                "source_span_ids": [source_span_id],
                "duration_secs": 4,
                "aspect_ratio": "16:9",
                "reference_image_ids": []
            },
            {
                "schema_version": "video_segment_plan_v2",
                "segment_number": 2,
                "source_shot_id": "shot-2",
                "segment_index_in_shot": 1,
                "shot_segment_count": 1,
                "covered_beat_ids": ["beat-3", "beat-4"],
                "title": "回望",
                "scene_summary": "小宦官回望暗处，压住恐惧",
                "blocking_summary": "小宦官从殿内中景转向暗廊，暗廊位于画面右后方，人物不要跳到暗廊另一侧",
                "spatial_anchors": [{"anchor": "暗廊入口", "usage": "约束回望方向和终点视线"}],
                "emotion_action_goals": [{"character": "小宦官", "emotion": "恐惧", "visible_action": "停步、回头、轻微后缩"}],
                "video_intent": {"hook": "小宦官回望暗处压住恐惧", "audience_effect": "制造悬疑和压迫", "rhythm": "停顿后缓慢回望", "visual_priority": "回头、后缩和暗廊方向"},
                "emotional_arc": {"start": "紧张", "turn": "听见或感到暗处异动", "end": "恐惧但克制", "function": "悬念保留"},
                "performance_plan": test_segment_performance_plan(
                    4,
                    "从殿内中央偏左转向右后方暗廊，轻微后缩后停在入口前。",
                    "小宦官停步回头看向暗廊，轻微后缩并压住恐惧。",
                    "从前方缓慢移向右后方暗廊入口",
                    "平视中景轻微跟随回头，保持人物不越过暗廊入口。",
                ),
                "material_coverage": {"required_entities": ["小宦官", "暗廊入口"], "missing_or_weak_references": ["小宦官", "暗廊入口"], "risk_note": "需要用文字锁定暗廊在画面右后方"},
                "transition_to_next": null,
                "source_span_ids": [source_span_id],
                "duration_secs": 4,
                "aspect_ratio": "16:9",
                "reference_image_ids": []
            }
        ]
    })
}

fn preview_options() -> ChapterVideoPromptOptions {
    ChapterVideoPromptOptions {
        style: "电影感写实".to_string(),
        prompt_output_language: PromptOutputLanguage::ZhCn,
        prompt_batch_size: 1,
        duration_secs: 4,
        duration_secs_options: vec![4],
        aspect_ratio: "16:9".to_string(),
        reference_image_ids: Vec::new(),
        video_library_image_ids: Vec::new(),
        duration_capability: None,
        regenerate_existing: false,
        workflow_run_id: None,
    }
}

fn source_context(chapter: &chapter::Model) -> ChapterVideoSourceContext<'_> {
    ChapterVideoSourceContext {
        content: &chapter.content,
        summary: chapter.summary.as_deref(),
        analysis: chapter.analysis_data.as_ref(),
    }
}

async fn fixture_video_execution_identity(
    service: &OrchestratorContext,
    project_id: i64,
    chapter: &chapter::Model,
    segment_value: &Value,
) -> Value {
    let resolved = service
        .require_current_chapter_shooting_script(
            crate::shooting_script::ChapterShootingScriptSourceInput {
                project_id,
                source_chapter_id: chapter.id,
            },
        )
        .await
        .expect("preview fixture requires current shooting script");
    let record = &resolved.shooting_script.record;
    let projection = crate::video::prompt_generation::video_projection_from_shooting_script(
        &resolved.shooting_script.script,
        a4n_core::video_execution::VideoShootingScriptIdentityV1 {
            record_id: record.id,
            revision: record.revision,
            source_revision_key: record.source_revision_key.clone(),
            source_content_sha256: record.source_content_sha256.clone(),
            script_sha256: record.script_sha256.clone(),
        },
        4,
    )
    .expect("preview fixture shooting script projection");
    let segment_plan: Vec<crate::video::prompt_json::VideoSegmentPlan> =
        serde_json::from_value(segment_value["segments"].clone())
            .expect("preview fixture segment plan");
    crate::video::prompt_generation::video_execution_chain_identity(
        &projection,
        &segment_plan,
        "电影感写实",
        4,
        "16:9",
    )
    .expect("preview fixture execution identity")
}

async fn store_segment_plan(
    service: &OrchestratorContext,
    project_id: i64,
    chapter: &chapter::Model,
    _workflow_mode: PromptWorkflowMode,
) {
    let prompt_type = "frame_segment_plan";
    let title = "视频片段规划";
    let contract_version = FRAME_SEGMENT_PLAN_CONTRACT_VERSION;
    let source = source_context(chapter);
    let source_metadata = super::super::prompt_context::video_source_metadata(&source);
    let source_span_id = a4n_core::source_spans::source_spans_for_content(
        chapter.id,
        chapter.chapter_number,
        &chapter.content,
    )
    .into_iter()
    .next()
    .expect("preview test chapter should have a source span")
    .id;
    let segment_value = stored_segment_plan_fixture(&source_span_id);
    let execution_identity =
        fixture_video_execution_identity(service, project_id, chapter, &segment_value).await;
    let segment_plan_json = segment_value.to_string();
    let draft = new_prompt_draft(NewChapterVideoPromptInput {
        project_id,
        chapter_id: chapter.id,
        workflow_mode: PromptWorkflowMode::BoundaryFrames,
        style: Some("电影感写实"),
        prompt_type,
        shot_number: 0,
        title: Some(title),
        scene_summary: None,
        visual_prompt: &segment_plan_json,
        motion_prompt: None,
        negative_prompt: None,
        duration_secs: 4,
        aspect_ratio: Some("16:9"),
        reference_image_ids_json: Some(json!([])),
        metadata_json: Some(json!({
            "contract_version": contract_version,
            "raw": segment_plan_json,
            "segments": segment_value["segments"].clone(),
            "video_execution_identity": execution_identity,
            "available_assets": [],
            "source": source_metadata
        })),
    });
    service
        .sync_chapter_prompts(chapter.id, vec![draft])
        .await
        .unwrap();
}

async fn store_boundary_frame_prompt(
    service: &OrchestratorContext,
    project_id: i64,
    chapter: &chapter::Model,
    frame_number: i32,
    segment_numbers: Vec<i32>,
) -> i64 {
    let source = source_context(chapter);
    let source_metadata = super::super::prompt_context::video_source_metadata(&source);
    let source_span_id = a4n_core::source_spans::source_spans_for_content(
        chapter.id,
        chapter.chapter_number,
        &chapter.content,
    )
    .into_iter()
    .next()
    .expect("preview test chapter should have a source span")
    .id;
    let segment_value = stored_segment_plan_fixture(&source_span_id);
    let execution_identity =
        fixture_video_execution_identity(service, project_id, chapter, &segment_value).await;
    let title = format!("边界帧 #{frame_number}");
    let scene_summary = format!("边界帧 #{frame_number} 的稳定画面");
    let frame_prompt = "【参考图约束】无参考图。【画面主体】小宦官站在殿门旁，保持安全静态姿态。【空间与道具】殿门在画面左侧，暗廊在右后方。【构图光线风格】平视中景，冷色弱光。【一致性与避免项】保持人物、服装、殿门和暗廊关系。"
        .to_string();
    let negative_prompt = "低质量、模糊、人体结构扭曲、空间关系错误、光影不连续".to_string();
    let frame = json!({
        "frame_number": frame_number,
        "title": title.clone(),
        "scene_summary": scene_summary.clone(),
        "storyboard_panels": [],
        "frame_prompt": frame_prompt.clone(),
        "blocking_state": {
            "characters": [{
                "name": "小宦官",
                "position": "殿门旁",
                "facing": "朝向暗廊",
                "pose": "站立"
            }],
            "spatial_anchors": ["殿门", "暗廊入口"],
            "do_not_change": ["殿门在左侧，暗廊在右后方"]
        },
        "lighting_state": {
            "time_of_day": "夜晚",
            "key_light": "殿门外冷光",
            "direction": "镜头左前方",
            "shadow": "右后方阴影较深",
            "intent": "压低安全感，延续悬疑"
        },
        "negative_prompt": negative_prompt.clone(),
        "duration_secs": 4,
        "aspect_ratio": "16:9",
        "reference_image_ids": [],
        "segment_numbers": segment_numbers
    });
    let generation_asset_fingerprint =
        crate::video::prompt_records::video_generation_asset_fingerprint(&[], &[], &json!([]));
    let metadata = a4n_core::prompt_language::with_prompt_output_language_metadata(
        json!({
            "contract_version": BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION,
            "boundary_frame": frame.clone(),
            "video_frame_boundary": {
                "source_segment_numbers": frame["segment_numbers"].clone(),
                "shared_between_units": frame["segment_numbers"]
                    .as_array()
                    .is_some_and(|numbers| numbers.len() > 1)
            },
            "reference_slots": [],
            "blocking_state": frame["blocking_state"].clone(),
            "lighting_state": frame["lighting_state"].clone(),
            "available_assets": [],
            "generation_asset_fingerprint": generation_asset_fingerprint,
            "video_execution_identity": execution_identity,
            "source_prompt_type": "frame_segment_plan",
            "source": source_metadata
        }),
        PromptOutputLanguage::ZhCn,
    );
    let draft = new_prompt_draft(NewChapterVideoPromptInput {
        project_id,
        chapter_id: chapter.id,
        workflow_mode: PromptWorkflowMode::BoundaryFrames,
        style: Some("电影感写实"),
        prompt_type: "frame_prompt",
        shot_number: frame_number,
        title: Some(title.as_str()),
        scene_summary: Some(scene_summary.as_str()),
        visual_prompt: frame_prompt.as_str(),
        motion_prompt: None,
        negative_prompt: Some(negative_prompt.as_str()),
        duration_secs: 4,
        aspect_ratio: Some("16:9"),
        reference_image_ids_json: Some(json!([])),
        metadata_json: Some(metadata),
    });
    let rows = service
        .sync_chapter_prompts(chapter.id, vec![draft])
        .await
        .unwrap();
    rows.into_iter()
        .find(|row| row.prompt_type == "frame_prompt" && row.shot_number == frame_number)
        .expect("stored boundary frame prompt")
        .id
}

async fn store_boundary_frame_image(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
    prompt_id: i64,
) {
    generated_image::ActiveModel {
        project_id: Set(project_id),
        image_type: Set("video_key_frame".to_string()),
        file_path: Set(format!("/tmp/boundary-frame-{prompt_id}.png")),
        prompt: Set("边界帧图片提示词".to_string()),
        model: Set("test-image-model".to_string()),
        width: Set(1280),
        height: Set(720),
        file_size: Set(None),
        ai_score: Set(None),
        is_favorite: Set(false),
        chapter_id: Set(Some(chapter_id)),
        panel_id: Set(None),
        metadata_json: Set(Some(json!({
            "generation_context": {
                "source": "video_boundary_frame_prompt",
                "prompt_id": prompt_id
            }
        }))),
        created_at: Set(fixed_time(4)),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

async fn store_matching_video_prompt(
    db: &DatabaseConnection,
    project_id: i64,
    chapter: &chapter::Model,
    workflow_mode: PromptWorkflowMode,
    status: &str,
) {
    let contract_version = match workflow_mode {
        PromptWorkflowMode::BoundaryFrames => FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
        PromptWorkflowMode::FirstFrameChain => FIRST_FRAME_CHAIN_CONTRACT_VERSION,
        PromptWorkflowMode::Storyboard => STORYBOARD_VIDEO_PROMPT_CONTRACT_VERSION,
        PromptWorkflowMode::ChatVideo => CHAT_VIDEO_CONTRACT_VERSION,
    };
    let source = source_context(chapter);
    let source_metadata = super::super::prompt_context::video_source_metadata(&source);
    let now = fixed_time(3);
    chapter_video_prompt::ActiveModel {
        project_id: Set(project_id),
        chapter_id: Set(chapter.id),
        style: Set(Some("电影感写实".to_string())),
        prompt_type: Set("video_prompt".to_string()),
        workflow_mode: Set(workflow_mode.as_str().to_string()),
        shot_number: Set(1),
        title: Set(Some("镜头一".to_string())),
        scene_summary: Set(Some("入殿".to_string())),
        visual_prompt: Set("完整视频提示词".to_string()),
        motion_prompt: Set(Some("慢推镜头".to_string())),
        negative_prompt: Set(None),
        duration_secs: Set(4),
        aspect_ratio: Set(Some("16:9".to_string())),
        reference_image_ids_json: Set(Some(json!([]))),
        status: Set(status.to_string()),
        generated_video_id: Set(None),
        metadata_json: Set(Some(json!({
            "contract_version": contract_version,
            "source_prompt_type": "frame_segment_plan",
            "generation_reference_image_ids": [],
            "source": source_metadata
        }))),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

fn step_states(preview: &VideoPromptRequestPreview) -> Vec<&str> {
    preview
        .steps
        .iter()
        .map(|step| step.state.as_str())
        .collect()
}

fn preview_step<'a>(
    preview: &'a VideoPromptRequestPreview,
    key: &str,
) -> &'a VideoPromptRequestPreviewStep {
    preview
        .steps
        .iter()
        .find(|step| step.key == key)
        .unwrap_or_else(|| panic!("missing preview step {key}"))
}

fn assert_displayable_steps_have_requests(preview: &VideoPromptRequestPreview) {
    for step in &preview.steps {
        if step.state == "ready" || step.state == "preview_placeholder" {
            assert!(
                step.request.is_some(),
                "displayable step {} must expose full request",
                step.key
            );
            assert!(
                step.messages.is_some(),
                "displayable step {} must expose chat messages",
                step.key
            );
        }
        assert_ne!(
            step.state, "blocked",
            "preview must not hide step {}",
            step.key
        );
    }
}

fn assert_incremental_boundary_frame_request(
    step: &VideoPromptRequestPreviewStep,
    expected_frame_number: i32,
) {
    let user_prompt = step.user_prompt.as_deref().unwrap_or_default();
    assert!(user_prompt.contains("current_frame"));
    assert!(user_prompt.contains("focused_chapter_video_segment_plan"));
    assert!(!user_prompt.contains("completed_chapter_video_frame_prompts"));
    assert!(user_prompt.contains("required_frame_count"));
    let compact_user_prompt = user_prompt.split_whitespace().collect::<String>();
    assert!(
        compact_user_prompt.contains(&format!(
            "\"required_frame_number\":{expected_frame_number}"
        )),
        "preview request must target boundary frame {expected_frame_number}"
    );
    assert!(user_prompt.contains("只输出 frames 数组中的 1 条"));
    assert!(!user_prompt.contains("frame_number_rule"));
    assert!(!user_prompt.contains("N+1 条"));
}
