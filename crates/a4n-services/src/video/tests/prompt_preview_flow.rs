use super::*;
use std::collections::HashMap;

const SEGMENT_PLAN_JSON: &str = r#"{
  "segments": [
    {
      "segment_number": 1,
      "analysis_shot_id": 1,
      "segment_index_in_shot": 1,
      "shot_segment_count": 1,
      "covered_beat_ids": [1, 2],
      "title": "入殿",
      "scene_summary": "小宦官推门入殿，发现异常",
      "blocking_summary": "小宦官从殿门外进入殿内，门是固定空间锚点，人物始终在画面中央偏左",
      "spatial_anchors": [{"anchor": "殿门", "usage": "约束入殿起点和移动方向"}],
      "emotion_action_goals": [{"character": "小宦官", "emotion": "紧张", "visible_action": "放慢脚步并收紧肩膀"}],
      "video_intent": {"hook": "推门入殿后察觉异常", "audience_effect": "让观众感到紧张和不安", "rhythm": "缓慢进入后短暂停顿", "visual_priority": "小宦官推门、入殿、发现异常的动作"},
      "emotional_arc": {"start": "谨慎", "turn": "察觉殿内异常", "end": "紧张压住恐惧", "function": "场景建立"},
      "material_coverage": {"required_entities": ["小宦官", "殿门", "殿内空间"], "missing_or_weak_references": ["小宦官", "殿门", "殿内空间"], "risk_note": "需要用文字明确人物服饰、殿门和殿内空间"},
      "source_span_ids": ["ch1:s1:f1"],
      "duration_secs": 4,
      "aspect_ratio": "16:9",
      "reference_image_ids": []
    },
    {
      "segment_number": 2,
      "analysis_shot_id": 2,
      "segment_index_in_shot": 1,
      "shot_segment_count": 1,
      "covered_beat_ids": [3, 4],
      "title": "回望",
      "scene_summary": "小宦官回望暗处，压住恐惧",
      "blocking_summary": "小宦官从殿内中景转向暗廊，暗廊位于画面右后方，人物不要跳到暗廊另一侧",
      "spatial_anchors": [{"anchor": "暗廊入口", "usage": "约束回望方向和终点视线"}],
      "emotion_action_goals": [{"character": "小宦官", "emotion": "恐惧", "visible_action": "停步、回头、轻微后缩"}],
      "video_intent": {"hook": "小宦官回望暗处压住恐惧", "audience_effect": "制造悬疑和压迫", "rhythm": "停顿后缓慢回望", "visual_priority": "回头、后缩和暗廊方向"},
      "emotional_arc": {"start": "紧张", "turn": "听见或感到暗处异动", "end": "恐惧但克制", "function": "悬念保留"},
      "material_coverage": {"required_entities": ["小宦官", "暗廊入口"], "missing_or_weak_references": ["小宦官", "暗廊入口"], "risk_note": "需要用文字锁定暗廊在画面右后方"},
      "source_span_ids": ["ch1:s1:f1"],
      "duration_secs": 4,
      "aspect_ratio": "16:9",
      "reference_image_ids": []
    }
  ]
}"#;

fn preview_options() -> ChapterVideoPromptOptions {
    ChapterVideoPromptOptions {
        style: "电影感写实".to_string(),
        prompt_output_language: None,
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

async fn store_segment_plan(
    service: &VideoService,
    project_id: i64,
    chapter: &chapter::Model,
    _workflow_mode: PromptWorkflowMode,
) {
    let prompt_type = "frame_segment_plan";
    let title = "视频片段规划";
    let contract_version = FRAME_SEGMENT_PLAN_CONTRACT_VERSION;
    let source = source_context(chapter);
    let source_metadata = super::super::prompt_context::video_source_metadata(&source);
    let segment_value: Value = serde_json::from_str(SEGMENT_PLAN_JSON).unwrap();
    let draft = new_prompt_draft(NewChapterVideoPromptInput {
        project_id,
        chapter_id: chapter.id,
        workflow_mode: PromptWorkflowMode::BoundaryFrames,
        style: Some("电影感写实"),
        prompt_type,
        shot_number: 0,
        title: Some(title),
        scene_summary: None,
        visual_prompt: SEGMENT_PLAN_JSON,
        motion_prompt: None,
        negative_prompt: None,
        duration_secs: 4,
        aspect_ratio: Some("16:9"),
        reference_image_ids_json: Some(json!([])),
        metadata_json: Some(json!({
            "contract_version": contract_version,
            "raw": SEGMENT_PLAN_JSON,
            "segments": segment_value["segments"].clone(),
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
    service: &VideoService,
    project_id: i64,
    chapter: &chapter::Model,
    frame_number: i32,
    segment_numbers: Vec<i32>,
) -> i64 {
    let source = source_context(chapter);
    let source_metadata = super::super::prompt_context::video_source_metadata(&source);
    let title = format!("边界帧 #{frame_number}");
    let scene_summary = format!("边界帧 #{frame_number} 的稳定画面");
    let frame_prompt = "【参考图约束】无参考图。【画面主体】小宦官站在殿门旁，保持安全静态姿态。【空间与道具】殿门在画面左侧，暗廊在右后方。【构图光线风格】平视中景，冷色弱光。【一致性与避免项】保持人物、服装、殿门和暗廊关系。"
        .to_string();
    let negative_prompt = "低质量、模糊、人体结构扭曲、空间关系错误、光影不连续".to_string();
    let frame = json!({
        "frame_number": frame_number,
        "title": title.clone(),
        "scene_summary": scene_summary.clone(),
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
    let raw = json!({ "frames": [frame.clone()] }).to_string();
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
        metadata_json: Some(json!({
            "contract_version": BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION,
            "raw": raw,
            "boundary_frame": frame,
            "source_prompt_type": "frame_segment_plan",
            "source": source_metadata
        })),
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
    assert!(user_prompt.contains(&format!(
        "\"required_frame_number\": {expected_frame_number}"
    )));
    assert!(user_prompt.contains("只输出 frames 数组中的 1 条"));
    assert!(!user_prompt.contains("frame_number_rule"));
    assert!(!user_prompt.contains("N+1 条"));
}

#[tokio::test]
async fn boundary_previews_ignore_existing_running_video_prompt() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let service = VideoService::new(Arc::clone(&db));
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    store_matching_video_prompt(
        db.as_ref(),
        project.id,
        &chapter,
        PromptWorkflowMode::BoundaryFrames,
        "running",
    )
    .await;

    let frame_preview = service
        .preview_chapter_frame_prompt_requests(
            project.id,
            chapter.id,
            source_context(&chapter),
            &engine,
            &HashMap::new(),
            preview_options(),
        )
        .await
        .unwrap();
    assert_displayable_steps_have_requests(&frame_preview);

    let video_preview = service
        .preview_chapter_video_prompt_requests(
            project.id,
            chapter.id,
            source_context(&chapter),
            &engine,
            &HashMap::new(),
            preview_options(),
        )
        .await
        .unwrap();
    assert_displayable_steps_have_requests(&video_preview);
}

#[tokio::test]
async fn boundary_frame_prompt_preview_with_segment_uses_incremental_single_frame_request() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let service = VideoService::new(db);
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    store_segment_plan(
        &service,
        project.id,
        &chapter,
        PromptWorkflowMode::BoundaryFrames,
    )
    .await;

    let preview = service
        .preview_chapter_frame_prompt_requests(
            project.id,
            chapter.id,
            source_context(&chapter),
            &engine,
            &HashMap::new(),
            preview_options(),
        )
        .await
        .unwrap();

    assert_eq!(step_states(&preview), vec!["reused", "ready"]);
    assert_displayable_steps_have_requests(&preview);
    assert_incremental_boundary_frame_request(preview_step(&preview, "frame_prompts"), 1);
}

#[tokio::test]
async fn boundary_frame_prompt_preview_after_first_frame_targets_second_frame_with_previous_context()
 {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let service = VideoService::new(db);
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    store_segment_plan(
        &service,
        project.id,
        &chapter,
        PromptWorkflowMode::BoundaryFrames,
    )
    .await;
    store_boundary_frame_prompt(&service, project.id, &chapter, 1, vec![1]).await;

    let preview = service
        .preview_chapter_frame_prompt_requests(
            project.id,
            chapter.id,
            source_context(&chapter),
            &engine,
            &HashMap::new(),
            preview_options(),
        )
        .await
        .unwrap();

    let frame_step = preview_step(&preview, "frame_prompts");
    assert_incremental_boundary_frame_request(frame_step, 2);
    let user_prompt = frame_step.user_prompt.as_deref().unwrap_or_default();
    assert!(user_prompt.contains("previous_frame"));
    assert!(user_prompt.contains("边界帧 #1"));
    assert!(user_prompt.contains("\"required_segment_numbers\""));
}

#[tokio::test]
async fn boundary_frame_prompt_preview_includes_previous_frame_image_when_available() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let service = VideoService::new(Arc::clone(&db));
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    store_segment_plan(
        &service,
        project.id,
        &chapter,
        PromptWorkflowMode::BoundaryFrames,
    )
    .await;
    let prompt_id = store_boundary_frame_prompt(&service, project.id, &chapter, 1, vec![1]).await;
    store_boundary_frame_image(db.as_ref(), project.id, chapter.id, prompt_id).await;

    let preview = service
        .preview_chapter_frame_prompt_requests(
            project.id,
            chapter.id,
            source_context(&chapter),
            &engine,
            &HashMap::new(),
            preview_options(),
        )
        .await
        .unwrap();

    let frame_step = preview_step(&preview, "frame_prompts");
    assert_incremental_boundary_frame_request(frame_step, 2);
    let user_prompt = frame_step.user_prompt.as_deref().unwrap_or_default();
    assert!(user_prompt.contains("previous_frame_image"));
    assert!(user_prompt.contains("boundary_frame_continuity"));
    assert!(user_prompt.contains("已生成的上一张边界帧画面"));
}

#[tokio::test]
async fn first_frame_previews_ignore_existing_running_video_prompt() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let service = VideoService::new(Arc::clone(&db));
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    store_matching_video_prompt(
        db.as_ref(),
        project.id,
        &chapter,
        PromptWorkflowMode::FirstFrameChain,
        "running",
    )
    .await;

    let frame_preview = service
        .preview_chapter_first_frame_prompt_requests(
            project.id,
            chapter.id,
            source_context(&chapter),
            &engine,
            &HashMap::new(),
            preview_options(),
        )
        .await
        .unwrap();
    assert_displayable_steps_have_requests(&frame_preview);

    let video_preview = service
        .preview_chapter_first_frame_video_prompt_requests(
            project.id,
            chapter.id,
            source_context(&chapter),
            &engine,
            &HashMap::new(),
            preview_options(),
        )
        .await
        .unwrap();
    assert_displayable_steps_have_requests(&video_preview);
}

#[tokio::test]
async fn boundary_video_prompt_preview_without_predecessors_returns_placeholder_chain() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let service = VideoService::new(db);
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();

    let preview = service
        .preview_chapter_video_prompt_requests(
            project.id,
            chapter.id,
            source_context(&chapter),
            &engine,
            &HashMap::new(),
            preview_options(),
        )
        .await
        .unwrap();

    assert_eq!(
        step_states(&preview),
        vec![
            "ready",
            "ready",
            "ready",
            "preview_placeholder",
            "preview_placeholder",
            "preview_placeholder"
        ]
    );
    assert_displayable_steps_have_requests(&preview);
    let video_step = preview_step(&preview, "video_prompt");
    assert!(
        video_step
            .user_prompt
            .as_deref()
            .unwrap_or_default()
            .contains("chapter_video_frame_prompts")
    );
    assert!(
        video_step
            .user_prompt
            .as_deref()
            .unwrap_or_default()
            .contains("preview_placeholder")
    );
}

#[tokio::test]
async fn boundary_video_prompt_preview_with_segment_but_missing_frames_shows_frame_request() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let service = VideoService::new(db);
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    store_segment_plan(
        &service,
        project.id,
        &chapter,
        PromptWorkflowMode::BoundaryFrames,
    )
    .await;

    let preview = service
        .preview_chapter_video_prompt_requests(
            project.id,
            chapter.id,
            source_context(&chapter),
            &engine,
            &HashMap::new(),
            preview_options(),
        )
        .await
        .unwrap();

    assert_eq!(
        step_states(&preview),
        vec![
            "reused",
            "ready",
            "preview_placeholder",
            "preview_placeholder"
        ]
    );
    assert_displayable_steps_have_requests(&preview);
    assert_eq!(preview_step(&preview, "frame_prompts").state, "ready");
    assert_incremental_boundary_frame_request(preview_step(&preview, "frame_prompts"), 1);
    assert_eq!(
        preview_step(&preview, "video_prompt").state,
        "preview_placeholder"
    );
}

#[tokio::test]
async fn chat_video_prompt_preview_without_predecessors_uses_chat_prompt_chain() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let service = VideoService::new(db);
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();

    let preview = service
        .preview_chapter_chat_video_prompt_requests(
            project.id,
            chapter.id,
            source_context(&chapter),
            &engine,
            &HashMap::new(),
            preview_options(),
        )
        .await
        .unwrap();

    assert_eq!(preview.workflow, "chapter_chat_video_prompt_generation");
    assert_eq!(
        preview
            .steps
            .iter()
            .map(|step| step.prompt_name.as_str())
            .collect::<Vec<_>>(),
        vec![
            "chapter_video_beat_extraction",
            "chapter_video_shot_analysis",
            "chapter_video_segment_plan",
            "chapter_video_frame_prompts",
            "chapter_video_chat_prompt",
            "chapter_video_chat_prompt_review"
        ]
    );
    assert_eq!(
        step_states(&preview),
        vec![
            "ready",
            "ready",
            "ready",
            "preview_placeholder",
            "preview_placeholder",
            "preview_placeholder"
        ]
    );
    assert_displayable_steps_have_requests(&preview);
}

#[tokio::test]
async fn chat_video_prompt_preview_ignores_boundary_segment_plan_when_available() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let service = VideoService::new(db);
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    store_segment_plan(
        &service,
        project.id,
        &chapter,
        PromptWorkflowMode::BoundaryFrames,
    )
    .await;

    let preview = service
        .preview_chapter_chat_video_prompt_requests(
            project.id,
            chapter.id,
            source_context(&chapter),
            &engine,
            &HashMap::new(),
            preview_options(),
        )
        .await
        .unwrap();

    assert_eq!(preview.workflow, "chapter_chat_video_prompt_generation");
    assert_eq!(
        step_states(&preview),
        vec![
            "reused",
            "ready",
            "preview_placeholder",
            "preview_placeholder"
        ]
    );
    assert_eq!(
        preview_step(&preview, "video_prompt").prompt_name,
        "chapter_video_chat_prompt"
    );
    assert_displayable_steps_have_requests(&preview);
}

#[tokio::test]
async fn first_frame_video_prompt_preview_without_predecessors_returns_placeholder_chain() {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let service = VideoService::new(db);
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();

    let preview = service
        .preview_chapter_first_frame_video_prompt_requests(
            project.id,
            chapter.id,
            source_context(&chapter),
            &engine,
            &HashMap::new(),
            preview_options(),
        )
        .await
        .unwrap();

    assert_eq!(
        step_states(&preview),
        vec![
            "ready",
            "ready",
            "ready",
            "preview_placeholder",
            "preview_placeholder",
            "preview_placeholder"
        ]
    );
    assert_displayable_steps_have_requests(&preview);
    let first_frame_step = preview_step(&preview, "first_frame_prompts");
    assert_eq!(
        first_frame_step.prompt_name,
        "chapter_video_first_frame_prompts"
    );
    let video_step = preview_step(&preview, "video_prompt");
    assert_eq!(
        video_step.prompt_name,
        "chapter_video_first_frame_video_prompt"
    );
    assert!(
        video_step
            .user_prompt
            .as_deref()
            .unwrap_or_default()
            .contains("chapter_video_first_frame_prompts")
    );
    assert!(
        video_step
            .user_prompt
            .as_deref()
            .unwrap_or_default()
            .contains("preview_placeholder")
    );
}

#[tokio::test]
async fn first_frame_video_prompt_preview_with_segment_but_missing_initial_frame_shows_frame_request()
 {
    let db = Arc::new(setup_db().await);
    let project = insert_video_project(db.as_ref(), "视频项目").await;
    let chapter = insert_video_chapter(db.as_ref(), project.id, 1).await;
    let service = VideoService::new(db);
    let engine = TeraPromptEngine::from_directory(&prompts_dir()).unwrap();
    store_segment_plan(
        &service,
        project.id,
        &chapter,
        PromptWorkflowMode::FirstFrameChain,
    )
    .await;

    let preview = service
        .preview_chapter_first_frame_video_prompt_requests(
            project.id,
            chapter.id,
            source_context(&chapter),
            &engine,
            &HashMap::new(),
            preview_options(),
        )
        .await
        .unwrap();

    assert_eq!(
        step_states(&preview),
        vec![
            "reused",
            "ready",
            "preview_placeholder",
            "preview_placeholder"
        ]
    );
    assert_displayable_steps_have_requests(&preview);
    assert_eq!(preview_step(&preview, "first_frame_prompts").state, "ready");
    assert_eq!(
        preview_step(&preview, "video_prompt").state,
        "preview_placeholder"
    );
}
