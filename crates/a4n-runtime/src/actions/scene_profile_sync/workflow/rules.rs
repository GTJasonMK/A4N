use a4n_core::models::chapter::chapter;
use a4n_core::source_spans::source_text_hash as source_content_sha256;
use serde_json::{Value, json};

pub(crate) fn scene_profile_sync_business_key(
    project_id: i64,
    chapter_number: Option<i32>,
) -> String {
    match chapter_number {
        Some(chapter_number) => {
            format!("scene_profile_sync:{project_id}:chapter:{chapter_number}")
        }
        None => format!("scene_profile_sync:{project_id}:all"),
    }
}

pub(crate) fn scene_profile_sync_workflow_fingerprint(
    project_id: i64,
    chapter_number: Option<i32>,
    chapters: &[chapter::Model],
) -> Value {
    json!({
        "workflow": "scene_profile_sync",
        "project_id": project_id,
        "chapter_number": chapter_number,
        "chapters": chapters.iter().map(|chapter| json!({
            "id": chapter.id,
            "chapter_number": chapter.chapter_number,
            "content_sha256": source_content_sha256(&chapter.content),
            "analysis_data": chapter.analysis_data,
        })).collect::<Vec<_>>(),
    })
}
