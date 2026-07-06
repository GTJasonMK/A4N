use super::super::*;
use serde_json::{Value, json};

#[test]
fn sync_write_steps_keep_deletion_marks_on_chapter_timeline() {
    let mut attribute_events = vec![
        AttributeEventDraft {
            chapter_number: 3,
            sequence: 0,
            stage: 0,
            category: "explicit".to_string(),
            key: "护身玉佩".to_string(),
            operation: "modify".to_string(),
            old_value: json!("旧玉佩"),
            new_value: json!("重新系在腰间"),
            change_description: "重新使用护身玉佩".to_string(),
            cause: "第3章重新使用".to_string(),
            evidence: "他重新将玉佩系在腰间".to_string(),
            from_classification: false,
        },
        AttributeEventDraft {
            chapter_number: 2,
            sequence: 0,
            stage: 1,
            category: "implicit".to_string(),
            key: "谨慎".to_string(),
            operation: "add".to_string(),
            old_value: Value::Null,
            new_value: json!("遇事先观察"),
            change_description: "从行为推断新增谨慎".to_string(),
            cause: "行为推断".to_string(),
            evidence: "他先看向四周".to_string(),
            from_classification: true,
        },
    ];
    attribute_events.sort_by(|left, right| {
        left.chapter_number
            .cmp(&right.chapter_number)
            .then_with(|| left.stage.cmp(&right.stage))
            .then_with(|| left.sequence.cmp(&right.sequence))
    });
    let behaviors = vec![json!({
        "chapter_number": 2,
        "behavior_type": "action",
        "behavior_description": "收起破损玉佩",
        "original_text": "他将玉佩碎片收进袖中",
        "behavior_tags": ["谨慎"],
        "classification_results": {},
        "related_characters": [],
        "impact_analysis": ""
    })];
    let deletion_candidates = vec![json!({
        "chapter_number": 2,
        "category": "explicit",
        "key": "护身玉佩",
        "reason": "疑似损毁",
        "evidence": "玉佩裂开一道纹路"
    })];

    let steps = sync_write_steps(
        &[2, 3],
        &attribute_events,
        &behaviors,
        &deletion_candidates,
        1,
        3,
    )
    .expect("write steps should build");
    let simplified: Vec<_> = steps
        .iter()
        .map(|step| (step.chapter_number, step.kind, step.index))
        .collect();

    assert_eq!(
        simplified,
        vec![
            (2, SyncWriteStepKind::AttributeEvent, 0),
            (2, SyncWriteStepKind::Behavior, 0),
            (2, SyncWriteStepKind::DeletionCandidate, 0),
            (3, SyncWriteStepKind::AttributeEvent, 1),
            (3, SyncWriteStepKind::ImplicitReview, 0),
        ]
    );
}
