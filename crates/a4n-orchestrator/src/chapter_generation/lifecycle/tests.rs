use std::collections::HashMap;

use serde_json::json;

use super::{
    StartChapterGenerationWorkflowInput, chapter_stream_task_id,
    workflow::chapter_generation_workflow_fingerprint,
};
use crate::chapter_generation::ChapterGenerationWordCountRange;

#[test]
fn chapter_stream_task_id_generates_id_when_absent() {
    let task_id = chapter_stream_task_id(7, 12, None).expect("generated task id");

    assert!(task_id.starts_with("chapter-7-12-"));
}

#[test]
fn chapter_stream_task_id_accepts_trimmed_safe_id() {
    let task_id = chapter_stream_task_id(7, 12, Some("  chapter:7_12.alpha-1  ".to_string()))
        .expect("provided task id");

    assert_eq!(task_id, "chapter:7_12.alpha-1");
}

#[test]
fn chapter_stream_task_id_rejects_invalid_provided_id() {
    assert!(chapter_stream_task_id(7, 12, Some("   ".to_string())).is_err());
    assert_eq!(
        chapter_stream_task_id(7, 12, Some("章节-7-12".to_string())),
        Err("task_id 只能包含 ASCII 字母、数字、-、_、:、.".to_string())
    );
    assert!(chapter_stream_task_id(7, 12, Some("a".repeat(161))).is_err());
}

#[test]
fn chapter_generation_workflow_fingerprint_preserves_generation_contract() {
    let variables = HashMap::from([("chapter_number".to_string(), json!(12))]);
    let fingerprint =
        chapter_generation_workflow_fingerprint(&StartChapterGenerationWorkflowInput {
            project_id: 7,
            chapter_number: 12,
            task_id: "chapter:7:12",
            chapter_title: "归途",
            outline_text: "章纲",
            context_before: Some("前文"),
            word_count_range: ChapterGenerationWordCountRange {
                min: 4_500,
                max: 6_000,
            },
            use_rag: true,
            variables: &variables,
        });

    assert_eq!(fingerprint["workflow"], "chapter_generation");
    assert_eq!(fingerprint["project_id"], 7);
    assert_eq!(fingerprint["chapter_number"], 12);
    assert_eq!(fingerprint["chapter_title"], "归途");
    assert_eq!(fingerprint["word_count_range"]["min"], 4_500);
    assert_eq!(fingerprint["word_count_range"]["max"], 6_000);
    assert_eq!(fingerprint["use_rag"], true);
    assert_eq!(fingerprint["variables"]["chapter_number"], 12);
}
