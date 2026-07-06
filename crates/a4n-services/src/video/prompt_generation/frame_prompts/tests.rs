use super::*;
use crate::project::{
    CreateDerivedProjectParams, DerivedChapterScope, PROJECT_KIND_VIDEO, PROJECT_TYPE_NOVEL_LONG,
    ProjectService,
};
use a4n_core::event::bridge::BroadcastEventBus;
use a4n_core::models::chapter::chapter;
use sea_orm::{ActiveModelTrait, Set};
use std::sync::Arc;

fn source_span(id: &str, paragraph_index: usize) -> SourceSpan {
    SourceSpan {
        id: id.to_string(),
        chapter_id: 1,
        chapter_number: 1,
        paragraph_index,
        sentence_index: 1,
        fragment_index: 1,
        text: id.to_string(),
        char_start: 0,
        char_end: id.chars().count(),
    }
}

#[test]
fn source_span_windows_preserve_paragraph_boundaries() {
    let spans = (0..70)
        .map(|index| source_span(&format!("p1-{index}"), 1))
        .chain((0..40).map(|index| source_span(&format!("p2-{index}"), 2)))
        .chain((0..10).map(|index| source_span(&format!("p3-{index}"), 3)))
        .collect::<Vec<_>>();

    let windows = source_span_windows(&spans);

    assert_eq!(windows.len(), 4);
    assert_eq!(windows[0].len(), 48);
    assert!(windows[0].iter().all(|span| span.paragraph_index == 1));
    assert_eq!(windows[1].len(), 22);
    assert!(windows[1].iter().all(|span| span.paragraph_index == 1));
    assert_eq!(windows[2].len(), 40);
    assert_eq!(windows[2][0].paragraph_index, 2);
    assert_eq!(windows[3].len(), 10);
    assert_eq!(windows[3][0].paragraph_index, 3);
}

#[test]
fn source_span_window_content_uses_only_current_window_text() {
    let mut first = source_span("ch1:s1:f1", 1);
    first.text = "当前窗口动作".to_string();
    let mut second = source_span("ch1:s2:f1", 1);
    second.text = "窗口外动作".to_string();
    let content = source_span_window_content(&[first]);

    assert!(content.contains("当前窗口动作"));
    assert!(!content.contains("窗口外动作"));
}

fn frame(frame_number: i32, reference_image_ids: Vec<i64>) -> VideoFramePrompt {
    VideoFramePrompt {
        frame_number,
        title: format!("边界帧 {frame_number}"),
        scene_summary: format!("画面状态 {frame_number}"),
        storyboard_panels: Vec::new(),
        frame_prompt: format!("提示词 {frame_number}"),
        blocking_state: json!({}),
        lighting_state: json!({}),
        negative_prompt: "低质量".to_string(),
        duration_secs: 5,
        aspect_ratio: "16:9".to_string(),
        reference_image_ids,
        segment_numbers: vec![frame_number],
    }
}

fn segment(segment_number: i32) -> VideoSegmentPlan {
    VideoSegmentPlan {
        segment_number,
        analysis_shot_id: segment_number,
        segment_index_in_shot: 1,
        shot_segment_count: 1,
        covered_beat_ids: vec![segment_number],
        title: format!("片段 {segment_number}"),
        scene_summary: format!("片段摘要 {segment_number}"),
        blocking_summary: "站位".to_string(),
        spatial_anchors: json!({}),
        emotion_action_goals: json!([]),
        video_intent: json!({}),
        emotional_arc: json!({}),
        performance_plan: json!({
            "movement_path": "站位",
            "body_performance": "保持当前姿态",
            "gaze_and_hands": "按片段摘要处理视线和手部",
            "pause_or_reaction": "保持可续接停顿",
            "camera_relationship": "镜头服务主体动作",
            "time_blocks": [],
            "action_script": []
        }),
        material_coverage: json!({}),
        source_span_ids: vec![format!("s{segment_number}")],
        duration_secs: 5,
        aspect_ratio: "16:9".to_string(),
        reference_image_ids: vec![3, 9],
    }
}

#[test]
fn video_units_use_selected_duration_for_content_and_transition_units() {
    let mut first = segment(1);
    first.duration_secs = 4;
    let mut second = segment(2);
    second.duration_secs = 4;

    let units = video_units_for_segment_plan(&[first, second], 8);

    assert_eq!(units.len(), 3);
    assert!(
        units.iter().all(|unit| unit.duration_secs == 8),
        "all generated video units must use the user-selected duration"
    );
}

#[test]
fn video_unit_segments_retime_performance_blocks_to_selected_duration() {
    let mut source = segment(1);
    source.duration_secs = 6;
    source.performance_plan["time_blocks"] = json!([
        {
            "start_sec": 0,
            "end_sec": 3,
            "camera": "前半段镜头",
            "subject_action": "前半段动作",
            "continuity_target": "前半段连续性"
        },
        {
            "start_sec": 3,
            "end_sec": 6,
            "camera": "后半段镜头",
            "subject_action": "后半段动作",
            "continuity_target": "后半段连续性"
        }
    ]);
    let units = video_units_for_segment_plan(&[source], 8);

    let segments = video_prompt_unit_segments_with_reference_ids(
        PromptWorkflowMode::FirstFrameChain,
        &units,
        &HashMap::new(),
        &HashMap::new(),
        &[],
    )
    .expect("video unit segments should retime system-owned time blocks");

    assert_eq!(segments[0].duration_secs, 8);
    assert_eq!(
        segments[0].performance_plan["time_blocks"][0]["start_sec"],
        json!(0.0)
    );
    assert_eq!(
        segments[0].performance_plan["time_blocks"][0]["end_sec"],
        json!(4.0)
    );
    assert_eq!(
        segments[0].performance_plan["time_blocks"][1]["start_sec"],
        json!(4.0)
    );
    assert_eq!(
        segments[0].performance_plan["time_blocks"][1]["end_sec"],
        json!(8.0)
    );
}

fn segment_plan_row(id: i64, source_hash: Option<&str>) -> chapter_video_prompt::Model {
    let now = chrono::Utc::now().naive_utc();
    let segment = segment(1);
    let metadata_json = Some(match source_hash {
        Some(hash) => json!({
            "contract_version": FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
            "segments": [segment.clone()],
            "source": {
                "content_sha256": hash
            }
        }),
        None => json!({
            "contract_version": FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
            "segments": [segment.clone()]
        }),
    });

    chapter_video_prompt::Model {
        id,
        project_id: 2,
        chapter_id: 1,
        style: Some("测试风格".to_string()),
        prompt_type: "frame_segment_plan".to_string(),
        workflow_mode: PromptWorkflowMode::BoundaryFrames.as_str().to_string(),
        shot_number: 0,
        title: Some("视频片段规划".to_string()),
        scene_summary: None,
        visual_prompt: json!({ "segments": [segment] }).to_string(),
        motion_prompt: None,
        negative_prompt: None,
        duration_secs: 5,
        aspect_ratio: Some("16:9".to_string()),
        reference_image_ids_json: Some(json!([])),
        status: "completed".to_string(),
        generated_video_id: None,
        metadata_json,
        created_at: now,
        updated_at: now,
    }
}

fn boundary_frame(
    frame_number: i32,
    reference_image_ids: Vec<i64>,
    segment_numbers: Vec<i32>,
) -> VideoFramePrompt {
    let mut frame = frame(frame_number, reference_image_ids);
    frame.segment_numbers = segment_numbers;
    frame
}

#[test]
fn rejects_frame_prompt_inputs_with_mismatched_non_duration_fields() {
    let mut old_prompt = boundary_frame(1, vec![59, 77], vec![1]);
    old_prompt.duration_secs = 15;
    old_prompt.aspect_ratio = "9:16".to_string();
    let mut current_segment = segment(1);
    current_segment.duration_secs = 5;
    current_segment.aspect_ratio = "16:9".to_string();

    let err = validate_frame_prompts_for_video_prompt_inputs(
        vec![old_prompt],
        &[current_segment],
        &HashSet::from([59]),
        "首帧提示词",
    )
    .expect_err("frame prompt input with mismatched saved fields must fail");

    assert!(
        err.to_string()
            .contains("aspect_ratio 必须等于片段 1 的画幅 16:9")
    );
}

#[test]
fn allows_frame_prompt_input_duration_to_differ_from_segment_plan() {
    let mut old_prompt = boundary_frame(1, vec![59], vec![1]);
    old_prompt.duration_secs = 15;
    old_prompt.aspect_ratio = "16:9".to_string();
    let mut current_segment = segment(1);
    current_segment.duration_secs = 5;
    current_segment.aspect_ratio = "16:9".to_string();

    validate_frame_prompts_for_video_prompt_inputs(
        vec![old_prompt],
        &[current_segment],
        &HashSet::from([59]),
        "首帧提示词",
    )
    .expect("frame prompt duration is planning context and should not block reuse");
}

#[test]
fn segment_plan_reuse_requires_saved_source_hash() {
    let row = segment_plan_row(11, None);
    let source_metadata = json!({ "content_sha256": "current-source" });
    let source_spans = vec![source_span("s1", 1)];

    let err = match reusable_segment_plan_from_row(ReusableSegmentPlanFromRowInput {
        row,
        source_metadata: &source_metadata,
        source_spans: &source_spans,
        style: "测试风格",
        aspect_ratio: "16:9",
        contract_version: FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
        context: "视频片段规划",
        draft_workflow_mode: PromptWorkflowMode::BoundaryFrames,
        draft_prompt_type: "frame_segment_plan",
        draft_title: "视频片段规划",
    }) {
        Ok(_) => panic!("segment plan without source hash must not be silently reused"),
        Err(err) => err,
    };

    assert!(
        err.to_string()
            .contains("metadata_json 缺少 source.content_sha256")
    );
}

#[test]
fn segment_plan_reuse_requires_metadata_json() {
    let mut row = segment_plan_row(13, Some("current-source"));
    row.metadata_json = None;
    let source_metadata = json!({ "content_sha256": "current-source" });
    let source_spans = vec![source_span("s1", 1)];

    let err = match reusable_segment_plan_from_row(ReusableSegmentPlanFromRowInput {
        row,
        source_metadata: &source_metadata,
        source_spans: &source_spans,
        style: "测试风格",
        aspect_ratio: "16:9",
        contract_version: FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
        context: "视频片段规划",
        draft_workflow_mode: PromptWorkflowMode::BoundaryFrames,
        draft_prompt_type: "frame_segment_plan",
        draft_title: "视频片段规划",
    }) {
        Ok(_) => panic!("segment plan without metadata_json must not be silently reused"),
        Err(err) => err,
    };

    assert!(err.to_string().contains("缺少 metadata_json"));
}

#[test]
fn segment_plan_reuse_rejects_source_mismatch() {
    let row = segment_plan_row(12, Some("old-source"));
    let source_metadata = json!({ "content_sha256": "current-source" });
    let source_spans = vec![source_span("s1", 1)];

    let err = match reusable_segment_plan_from_row(ReusableSegmentPlanFromRowInput {
        row,
        source_metadata: &source_metadata,
        source_spans: &source_spans,
        style: "测试风格",
        aspect_ratio: "16:9",
        contract_version: FRAME_SEGMENT_PLAN_CONTRACT_VERSION,
        context: "视频片段规划",
        draft_workflow_mode: PromptWorkflowMode::BoundaryFrames,
        draft_prompt_type: "frame_segment_plan",
        draft_title: "视频片段规划",
    }) {
        Ok(_) => panic!("segment plan from another source must not be reused"),
        Err(err) => err,
    };

    assert!(err.to_string().contains("来源内容与当前请求不一致"));
}

fn row_for_boundary_frame(id: i64, frame: VideoFramePrompt) -> chapter_video_prompt::Model {
    let now = chrono::Utc::now().naive_utc();
    chapter_video_prompt::Model {
        id,
        project_id: 2,
        chapter_id: 1,
        style: Some("测试风格".to_string()),
        prompt_type: "frame_prompt".to_string(),
        workflow_mode: PromptWorkflowMode::BoundaryFrames.as_str().to_string(),
        shot_number: frame.frame_number,
        title: Some(frame.title.clone()),
        scene_summary: Some(frame.scene_summary.clone()),
        visual_prompt: frame.frame_prompt.clone(),
        motion_prompt: None,
        negative_prompt: Some(frame.negative_prompt.clone()),
        duration_secs: frame.duration_secs,
        aspect_ratio: Some(frame.aspect_ratio.clone()),
        reference_image_ids_json: Some(json!(frame.reference_image_ids.clone())),
        status: "pending".to_string(),
        generated_video_id: None,
        metadata_json: Some(json!({
            "boundary_frame": frame
        })),
        created_at: now,
        updated_at: now,
    }
}

#[test]
fn image_lookup_context_ignores_generation_asset_fingerprint() {
    let source_metadata = json!({
        "content_sha256": "same-source"
    });
    let mut row = row_for_boundary_frame(45, frame(1, vec![3]));
    row.metadata_json = Some(json!({
        "contract_version": BOUNDARY_FRAME_PROMPT_CONTRACT_VERSION,
        "boundary_frame": frame(1, vec![3]),
        "source_prompt_type": "frame_segment_plan",
        "source": {
            "content_sha256": "same-source"
        },
        "generation_asset_fingerprint": {
            "schema_version": 1,
            "reference_image_ids": [3],
            "video_library_image_ids": [],
            "available_assets_sha256": "old-assets"
        },
        "prompt_output_language": "zh_cn"
    }));

    assert!(frame_row_matches_image_lookup_context(
        &row,
        &FrameImageLookupContext {
            workflow_mode: PromptWorkflowMode::BoundaryFrames,
            prompt_type: "frame_prompt",
            source_metadata: &source_metadata,
            style: "测试风格",
            aspect_ratio: "16:9",
            source_prompt_type: Some("frame_segment_plan"),
            prompt_output_language: PromptOutputLanguage::ZhCn,
        },
    ));
}

#[test]
fn video_prompt_reference_ids_are_system_derived_from_generation_and_frames() {
    let segments = vec![segment(1), segment(2)];
    let frame_by_number = HashMap::from([
        (1, frame(1, vec![11])),
        (2, frame(2, vec![12])),
        (3, frame(3, vec![13, 11])),
    ]);
    let frame_image_ids_by_number = HashMap::from([(1, 101), (2, 102), (3, 103)]);

    let enriched_segments = video_prompt_segments_with_reference_ids(
        PromptWorkflowMode::BoundaryFrames,
        &segments,
        &frame_by_number,
        &frame_image_ids_by_number,
        &[99, 12],
    );

    assert_eq!(segments[0].reference_image_ids, vec![3, 9]);
    assert_eq!(
        enriched_segments[0].reference_image_ids,
        vec![101, 102, 11, 12, 99]
    );
    assert_eq!(
        enriched_segments[1].reference_image_ids,
        vec![102, 103, 12, 13, 11, 99]
    );
}

#[test]
fn frame_prompt_style_template_uses_prompt_output_language_sections() {
    let prompt = "[Reference Constraints]None.\n[Main Subject]The protagonist sits on the warehouse floor.\n[Space And Props]Broken bricks and shallow puddles stay fixed.\n[Composition Lighting Style]Cold side backlight.\n[Continuity And Avoidances]Keep character identity and warehouse anchors consistent.";
    let with_style = frame_prompt_with_style_template(
        prompt,
        "KyoAni inspired anime style",
        PromptOutputLanguage::En,
    );

    assert!(with_style.contains("[Composition Lighting Style]"));
    assert!(with_style.contains("Verbatim visual style template: KyoAni inspired anime style"));
    assert!(with_style.contains("[Continuity And Avoidances]"));
    assert!(!with_style.contains("【构图光线风格】"));
}

#[test]
fn boundary_frame_validation_accepts_safe_rewrite_prompt() {
    let mut frame = boundary_frame(1, Vec::new(), vec![1]);
    frame.frame_prompt =
        "【参考图约束】无。【画面主体】林寻坐在仓库地面，手停在安全距离外，表情压抑决绝。【空间与道具】危险铁片落在地面远处，碎砖和墙角固定不变。【构图光线风格】冷色中景，侧光从左前方进入。【一致性与避免项】保持人物、服装、仓库锚点和光源方向，避免血腥喷溅。"
            .to_string();

    validate_single_boundary_frame_prompt(&frame, &[segment(1)], &HashSet::new(), None)
        .expect("safe rewrite prompt should pass");
}

#[test]
fn boundary_frame_validation_leaves_unsafe_wording_to_prompt_optimization() {
    let mut frame = boundary_frame(1, Vec::new(), vec![1]);
    frame.frame_prompt =
        "【参考图约束】无。【画面主体】林寻把铁片刺入眼睛的执行瞬间，表情痛苦。【空间与道具】铁片贴近面部。【构图光线风格】冷色近景。【一致性与避免项】保持人物一致。"
            .to_string();

    validate_single_boundary_frame_prompt(&frame, &[segment(1)], &HashSet::new(), None)
        .expect("unsafe wording is left to prompt optimization instead of hard validation");
}

#[test]
fn boundary_frame_validation_rejects_internal_reference_id_text() {
    let mut frame = boundary_frame(1, vec![3], vec![1]);
    frame.frame_prompt =
        "【参考图约束】参考图 #3 约束林寻外观和仓库空间。【画面主体】林寻坐在地面，表情警觉。【空间与道具】水洼、碎砖和货架保持固定。【构图光线风格】冷色中景，侧逆光从右后方进入。【一致性与避免项】保持人物服装、空间锚点和光源方向。"
            .to_string();
    let err =
        validate_single_boundary_frame_prompt(&frame, &[segment(1)], &HashSet::from([3]), None)
            .expect_err("internal reference image id text should fail visibly");

    assert!(err.to_string().contains("内部图片 ID"));
}

#[test]
fn boundary_frame_validation_requires_previous_image_as_first_reference() {
    let segment_plan = vec![segment(1), segment(2)];
    let mut frame = boundary_frame(2, vec![3, 13], vec![1]);
    frame.frame_prompt =
        "【参考图约束】参考图 1 约束人物外观，参考图 2 约束上一帧连续性。【画面主体】林寻站在仓库门边，表情警觉。【空间与道具】水洼、碎砖和货架保持固定。【构图光线风格】冷色中景，侧逆光从右后方进入。【一致性与避免项】保持人物服装、空间锚点和光源方向。"
            .to_string();

    let err = validate_single_boundary_frame_prompt(
        &frame,
        &segment_plan,
        &HashSet::from([3, 13]),
        Some(13),
    )
    .expect_err("previous frame image must be reference image 1");

    assert!(err.to_string().contains("reference_image_ids[0]"));
}

#[test]
fn trims_reusable_frame_prefix_when_previous_image_is_missing_from_prompt_refs() {
    let frames = vec![frame(1, vec![3, 9]), frame(2, vec![3, 9])];
    let image_id_by_frame_number = HashMap::from([(1, 13)]);

    let keep_len =
        reusable_frame_prefix_len_with_previous_images(&frames, &image_id_by_frame_number);

    assert_eq!(keep_len, 1);
}

#[test]
fn appends_previous_boundary_frame_as_regular_available_asset() {
    let available_assets = json!([
        {"image_id": 3, "display_order": 1, "display_label": "参考图 1"},
        {"image_id": 9, "display_order": 2, "display_label": "参考图 2"}
    ]);
    let previous_frame = frame(1, vec![3, 9]);

    let expanded = available_assets_with_previous_boundary_frame(
        &available_assets,
        Some(13),
        Some(&previous_frame),
    );

    let items = expanded
        .as_array()
        .expect("expanded assets should be array");
    assert_eq!(items.len(), 3);
    assert_eq!(items[2].get("image_id"), Some(&json!(13)));
    assert_eq!(items[2].get("display_label"), Some(&json!("参考图 3")));
    assert_eq!(
        previous_boundary_frame_asset_context(&expanded, Some(13))
            .and_then(|value| value.get("slot_label").cloned()),
        Some(json!("参考图 1"))
    );
    assert!(
        previous_boundary_frame_asset_context(&expanded, Some(13))
            .and_then(|value| value.get("image_id").cloned())
            .is_none()
    );
}

#[test]
fn frame_prompt_available_assets_hide_prompt_visible_asset_labels() {
    let available_assets = json!([
        {"image_id": 3, "display_order": 1, "display_label": "参考图 1", "label": "角色主图"},
        {"image_id": 9, "display_order": 2, "display_label": "参考图 2", "label": "场景主图"}
    ]);

    let prompt_assets = frame_prompt_available_assets_context(&available_assets)
        .expect("available assets should be sanitized");
    let items = prompt_assets
        .as_array()
        .expect("prompt assets should exist");
    let item = items.first().expect("prompt asset should exist");

    assert!(item.get("display_label").is_none());
    assert!(
        items
            .get(1)
            .and_then(|item| item.get("display_label"))
            .is_none()
    );
    assert!(item.get("candidate_label").is_none());
    assert!(item.get("source_display_label").is_none());
}

#[test]
fn frame_prompt_available_assets_do_not_expose_internal_image_ids() {
    let available_assets = json!([
        {
            "image_id": 59,
            "display_order": 1,
            "display_label": "参考图 1",
            "asset_id": "manual_library_asset_1",
            "source_id": "manual_library_asset_1",
            "candidate_label": "manual_library_asset_1",
            "source_display_label": "参考图 1",
            "project_id": 9,
            "profile_id": 7,
            "role": "chapter_material",
            "label": "manual_library_asset_1",
            "description": "林清辞角色参考 manual_library_asset_1",
            "prompt_excerpt": "清晨教室窗边的角色主图",
            "selection_reason": "manual_library",
            "profile_kind": "asset_id 59",
            "is_primary": true,
            "is_favorite": false
        }
    ]);

    let prompt_assets = frame_prompt_available_assets_context(&available_assets)
        .expect("available assets should be sanitized");
    let item = prompt_assets
        .as_array()
        .and_then(|items| items.first())
        .expect("prompt asset should exist");
    let serialized = serde_json::to_string(item).expect("asset context should serialize");

    assert!(item.get("image_id").is_none());
    assert!(item.get("display_label").is_none());
    assert!(item.get("label").is_none());
    assert!(item.get("profile_kind").is_none());
    assert_eq!(item.get("description"), Some(&json!("林清辞角色参考")));
    assert_eq!(
        item.get("prompt_excerpt"),
        Some(&json!("清晨教室窗边的角色主图"))
    );
    assert!(item.get("display_order").is_none());
    assert!(item.get("project_id").is_none());
    assert!(item.get("profile_id").is_none());
    assert!(item.get("asset_id").is_none());
    assert!(item.get("source_id").is_none());
    assert!(item.get("candidate_label").is_none());
    assert!(item.get("source_display_label").is_none());
    assert!(!serialized.contains("manual_library_asset_1"));
    assert!(!serialized.contains("候选素材"));
    assert!(!serialized.contains("asset_id"));
}

#[test]
fn reference_slots_do_not_repeat_internal_image_ids() {
    let slots = reference_slots_for_frame(2, Some("previous_boundary_frame"));
    let first = slots
        .as_array()
        .and_then(|items| items.first())
        .expect("reference slot should exist");

    assert_eq!(first.get("slot_label"), Some(&json!("参考图 1")));
    assert!(first.get("image_id").is_none());
}

#[test]
fn validates_reused_boundary_frames_with_previous_image_allowed() {
    let segment_plan = vec![segment(1), segment(2)];
    let frames = vec![
        boundary_frame(1, vec![3, 9], vec![1]),
        boundary_frame(2, vec![13, 3, 9], vec![1]),
        boundary_frame(3, vec![3, 9], vec![2]),
        boundary_frame(4, vec![3, 9], vec![2]),
    ];
    let base_allowed = HashSet::from([3, 9]);
    let mut allowed_with_previous_image = base_allowed.clone();
    allowed_with_previous_image.insert(13);

    let err = validate_frame_prompts(&frames, &segment_plan, &base_allowed)
        .expect_err("previous boundary frame image must be explicitly allowed");
    assert!(err.to_string().contains("图片 ID：13"));
    validate_frame_prompts(&frames, &segment_plan, &allowed_with_previous_image)
        .expect("previous boundary frame image should be accepted as an available reference");
}

#[test]
fn reusable_boundary_frame_prefix_stops_before_stale_reference_image() {
    let segment_plan = vec![segment(1), segment(2)];
    let rows = vec![
        row_for_boundary_frame(45, boundary_frame(1, vec![3, 9], vec![1])),
        row_for_boundary_frame(50, boundary_frame(2, vec![3, 9, 13], vec![1, 2])),
    ];
    let allowed = HashSet::from([3, 9]);

    let prefix =
        boundary_frame_prompt_rows_prefix(rows, &segment_plan, &allowed, &HashMap::new()).unwrap();

    assert_eq!(prefix.len(), 1);
    assert_eq!(prefix[0].id, 45);
}

#[test]
fn generated_boundary_frame_prefix_stops_before_mismatched_contract_fields() {
    let segment_plan = vec![segment(1), segment(2)];
    let rows = vec![
        row_for_boundary_frame(169, boundary_frame(1, vec![59], vec![1])),
        row_for_boundary_frame(178, boundary_frame(2, vec![88, 80], vec![1, 2])),
    ];
    let image_id_by_frame = HashMap::from([(1, 88), (2, 93)]);
    let allowed = HashSet::from([59, 80, 88, 93]);

    let prefix =
        boundary_frame_prompt_rows_prefix(rows, &segment_plan, &allowed, &image_id_by_frame)
            .expect("reusable prefix validation should stop before invalid saved contract fields");

    assert_eq!(prefix.iter().map(|row| row.id).collect::<Vec<_>>(), [169]);
}

#[tokio::test]
async fn boundary_frame_image_lookup_accepts_root_project_images_for_video_project() {
    let db = Arc::new(a4n_migration::test_support::connect_test_database().await);
    let project_service = ProjectService::new(db.clone(), Arc::new(BroadcastEventBus::new(16)));
    let root = project_service
        .create(1, "根文本", "", PROJECT_TYPE_NOVEL_LONG)
        .await
        .unwrap();
    let now = chrono::Utc::now().naive_utc();
    let chapter = chapter::ActiveModel {
        project_id: Set(root.id),
        outline_id: Set(None),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("章节正文".to_string()),
        word_count: Set(4),
        status: Set("draft".to_string()),
        summary: Set(None),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap();
    let video_project = project_service
        .create_derived_project(
            1,
            CreateDerivedProjectParams {
                root_project_id: root.id,
                derived_kind: PROJECT_KIND_VIDEO.to_string(),
                title: "视频项目".to_string(),
                description: String::new(),
                chapter_scope: DerivedChapterScope {
                    start_chapter: 1,
                    end_chapter: 1,
                },
                creation_options_json: json!({}),
            },
        )
        .await
        .unwrap();
    let prompt = chapter_video_prompt::ActiveModel {
        project_id: Set(video_project.id),
        chapter_id: Set(chapter.id),
        style: Set(Some("测试风格".to_string())),
        prompt_type: Set("frame_prompt".to_string()),
        workflow_mode: Set(PromptWorkflowMode::ChatVideo.as_str().to_string()),
        shot_number: Set(1),
        title: Set(Some("边界帧 1".to_string())),
        scene_summary: Set(Some("首帧".to_string())),
        visual_prompt: Set("首帧提示词".to_string()),
        motion_prompt: Set(None),
        negative_prompt: Set(Some("低质量".to_string())),
        duration_secs: Set(5),
        aspect_ratio: Set(Some("16:9".to_string())),
        reference_image_ids_json: Set(Some(json!([]))),
        status: Set("completed".to_string()),
        metadata_json: Set(Some(json!({
            "boundary_frame": frame(1, vec![])
        }))),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap();
    let image = generated_image::ActiveModel {
        project_id: Set(root.id),
        image_type: Set("video_boundary_frame".to_string()),
        file_path: Set("frames/1.png".to_string()),
        prompt: Set("首帧提示词".to_string()),
        model: Set("image-model".to_string()),
        width: Set(1280),
        height: Set(720),
        file_size: Set(None),
        ai_score: Set(None),
        is_favorite: Set(false),
        chapter_id: Set(Some(chapter.id)),
        panel_id: Set(None),
        metadata_json: Set(Some(json!({
            "generation_context": {
                "source": "video_boundary_frame_prompt",
                "prompt_id": prompt.id
            }
        }))),
        created_at: Set(now),
        ..Default::default()
    }
    .insert(db.as_ref())
    .await
    .unwrap();
    let video_service = VideoService::new(db);

    let image_id_by_frame =
        boundary_frame_image_ids_by_number(&video_service, video_project.id, chapter.id, &[prompt])
            .await
            .unwrap();

    assert_eq!(image_id_by_frame.get(&1), Some(&image.id));
}
