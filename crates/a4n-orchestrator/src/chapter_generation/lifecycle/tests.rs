use std::collections::HashMap;

use serde_json::json;

use super::{
    StartChapterGenerationWorkflowInput, workflow::chapter_generation_workflow_fingerprint,
};
use crate::chapter_generation::ChapterGenerationWordCountRange;

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
