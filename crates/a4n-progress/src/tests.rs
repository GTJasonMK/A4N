use super::*;

const FRAME_MILESTONES: &[FieldMilestone] = &[
    FieldMilestone::new("title", "标题", 1),
    FieldMilestone::new("scene_summary", "剧情状态", 1),
    FieldMilestone::new("frame_prompt", "画面提示词", 1),
    FieldMilestone::new("blocking_state", "调度状态", 1),
    FieldMilestone::new("lighting_state", "光影状态", 1),
    FieldMilestone::new("negative_prompt", "负向提示词", 1),
];

fn frame_spec(expected_items: Option<usize>) -> StructuredJsonProgressSpec {
    StructuredJsonProgressSpec {
        root_array_key: "frames",
        item_label: "边界帧",
        expected_items,
        milestones: FRAME_MILESTONES,
    }
}

#[test]
fn tracks_split_json_fields_inside_root_items() {
    let mut tracker =
        StructuredJsonProgressTracker::new(frame_spec(Some(1)), ProgressSpan::new(20, 80));
    let mut events = Vec::new();
    for chunk in [
        r#"```json
{"frames":[{"#,
        r#""frame_number":1,"title":"开场","#,
        r#""scene_summary":"{不是对象}","#,
        r#""frame_prompt":"带有 \"引号\" 的提示","#,
        r#""blocking_state":{"characters":[]},"#,
        r#""lighting_state":{"time":"夜"},"#,
        r#""negative_prompt":"避免模糊"}]}"#,
    ] {
        events.extend(tracker.consume(chunk));
    }

    let messages = events
        .iter()
        .map(|event| event.message.as_str())
        .collect::<Vec<_>>();
    assert!(
        messages
            .iter()
            .any(|message| message.contains("标题已完成"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("剧情状态已完成"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("画面提示词已完成"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("调度状态已完成"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("光影状态已完成"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("负向提示词已完成"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("边界帧已完成"))
    );
    assert!(
        events
            .windows(2)
            .all(|pair| pair[0].current <= pair[1].current)
    );
}

#[test]
fn ignores_matching_keys_inside_nested_objects() {
    let mut tracker =
        StructuredJsonProgressTracker::new(frame_spec(Some(1)), ProgressSpan::new(0, 100));
    let events = tracker.consume(
            r#"{"frames":[{"title":"外层","blocking_state":{"title":"内层"},"negative_prompt":"避免模糊"}]}"#,
        );
    let title_events = events
        .iter()
        .filter(|event| event.message.contains("标题已完成"))
        .count();
    assert_eq!(title_events, 1);
}

#[test]
fn omits_moving_denominator_when_expected_item_count_is_unknown() {
    let mut tracker =
        StructuredJsonProgressTracker::new(frame_spec(None), ProgressSpan::new(20, 80));
    let events = tracker.consume(
        r#"{"frames":[{"title":"一","scene_summary":"一"},{"title":"二","scene_summary":"二"}]}"#,
    );
    let messages = events
        .iter()
        .map(|event| event.message.as_str())
        .collect::<Vec<_>>();

    assert!(
        messages
            .iter()
            .any(|message| message.contains("第 1 个边界帧：标题已完成"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("第 2 个边界帧：标题已完成"))
    );
    assert!(
        messages
            .iter()
            .all(|message| !message.contains("/2") && !message.contains("/3"))
    );
    assert!(
        events
            .windows(2)
            .all(|pair| pair[0].current <= pair[1].current)
    );
}

#[test]
fn tracks_root_object_fields_when_root_array_key_is_empty() {
    const ROOT_FIELDS: &[FieldMilestone] = &[
        FieldMilestone::new("metadata_additions", "元数据补充", 1),
        FieldMilestone::new("summaries", "摘要", 1),
    ];
    let mut tracker = StructuredJsonProgressTracker::new(
        StructuredJsonProgressSpec {
            root_array_key: "",
            item_label: "章节摘要阶段",
            expected_items: None,
            milestones: ROOT_FIELDS,
        },
        ProgressSpan::new(20, 40),
    );
    let events = tracker.consume(
            r#"{"metadata_additions":{"tags":["转折"],"tone":"紧张"},"summaries":{"one_line":"发现线索"}}"#,
        );
    let messages = events
        .iter()
        .map(|event| event.message.as_str())
        .collect::<Vec<_>>();

    assert!(
        messages
            .iter()
            .any(|message| message.contains("元数据补充已完成"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("摘要已完成"))
    );
    assert!(
        messages
            .iter()
            .any(|message| message.contains("章节摘要阶段已完成"))
    );
}
