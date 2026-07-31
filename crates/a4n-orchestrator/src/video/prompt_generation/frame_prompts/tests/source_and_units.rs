use super::*;

#[test]
fn video_units_only_add_action_bridge_for_preserved_action() {
    let mut first = segment(1);
    first.transition_to_next = Some(a4n_core::video_execution::VideoSegmentTransitionV1 {
        transition_type: "match_cut".to_string(),
        editing_mode: a4n_core::shooting_script::ShootingEditingModeV1::Continuity,
        preserved_action: Some("角色推门动作连续完成".to_string()),
        preserved_sound: None,
        required_cut_point: None,
    });
    let second = segment(2);

    let units = video_units_for_segment_plan(&[first, second], 5).expect("video units");

    assert_eq!(units.len(), 3);
    assert!(
        units.iter().all(|unit| unit.duration_secs == 5),
        "all generated video units must use the user-selected duration"
    );
    assert!(units[1].is_action_bridge());
    assert_eq!(
        units[1].prompt_segment.video_intent.hook,
        "角色推门动作连续完成"
    );
    assert_ne!(
        units[1].prompt_segment.performance_plan, units[0].prompt_segment.performance_plan,
        "action bridge must not copy the previous content performance plan"
    );
}

#[test]
fn target_segment_selects_content_unit_even_when_a_bridge_precedes_it() {
    let mut first = segment(1);
    first.transition_to_next = Some(a4n_core::video_execution::VideoSegmentTransitionV1 {
        transition_type: "match_cut".to_string(),
        editing_mode: a4n_core::shooting_script::ShootingEditingModeV1::Continuity,
        preserved_action: Some("角色推门动作连续完成".to_string()),
        preserved_sound: None,
        required_cut_point: None,
    });
    let plan = vec![first, segment(2)];

    let unit = video_unit_for_target_segment(&plan, 5, 2).expect("target content unit");

    assert_eq!(unit.unit_number, 2);
    assert_eq!(unit.prompt_segment.segment_number, 2);
    assert_eq!(unit.source_segment_numbers(), [2]);
    assert!(!unit.is_action_bridge());
    assert_eq!(unit.storyboard_number, Some(2));
}

#[test]
fn sound_only_or_cut_metadata_does_not_create_action_bridge() {
    let mut first = segment(1);
    first.transition_to_next = Some(a4n_core::video_execution::VideoSegmentTransitionV1 {
        transition_type: "hard_cut".to_string(),
        editing_mode: a4n_core::shooting_script::ShootingEditingModeV1::Continuity,
        preserved_action: None,
        preserved_sound: Some("门铃声延续".to_string()),
        required_cut_point: Some("铃声响起时切换".to_string()),
    });
    let units = video_units_for_segment_plan(&[first, segment(2)], 5).expect("video units");
    assert_eq!(units.len(), 2);
    assert!(units.iter().all(|unit| !unit.is_action_bridge()));
    assert_eq!(expected_boundary_frame_count(&[segment(1), segment(2)]), 3);
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
fn generated_boundary_frame_prefix_stops_before_mismatched_contract_fields() {
    let segment_plan = vec![segment(1), segment(2)];
    let rows = vec![
        row_for_boundary_frame(169, boundary_frame(1, vec![59], vec![1])),
        row_for_boundary_frame(178, boundary_frame(2, vec![88, 80], vec![2])),
    ];
    let image_id_by_frame = HashMap::from([(1, 88), (2, 93)]);
    let allowed = HashSet::from([59, 80, 88, 93]);

    let prefix =
        boundary_frame_prompt_rows_prefix(rows, &segment_plan, &allowed, &image_id_by_frame)
            .expect("reusable prefix validation should stop before invalid saved contract fields");

    assert_eq!(prefix.iter().map(|row| row.id).collect::<Vec<_>>(), [169]);
}
