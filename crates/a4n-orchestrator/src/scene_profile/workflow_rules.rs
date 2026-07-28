//! 场景档案同步工作流指纹纯规则（无 DB）。

use a4n_core::models::chapter::chapter;
use a4n_core::source_spans::source_text_hash as source_content_sha256;
use serde_json::{Value, json};

/// 场景档案同步 exclusive workflow business key。
pub fn scene_profile_sync_business_key(project_id: i64, chapter_number: Option<i32>) -> String {
    match chapter_number {
        Some(chapter_number) => {
            format!("scene_profile_sync:{project_id}:chapter:{chapter_number}")
        }
        None => format!("scene_profile_sync:{project_id}:all"),
    }
}

/// 场景档案同步工作流输入指纹。
pub fn scene_profile_sync_workflow_fingerprint(
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn business_key_covers_chapter_and_all_scope() {
        assert_eq!(
            scene_profile_sync_business_key(7, Some(3)),
            "scene_profile_sync:7:chapter:3"
        );
        assert_eq!(
            scene_profile_sync_business_key(7, None),
            "scene_profile_sync:7:all"
        );
    }

    #[test]
    fn workflow_fingerprint_includes_project_and_scope() {
        let value = scene_profile_sync_workflow_fingerprint(9, Some(2), &[]);
        assert_eq!(value["workflow"], "scene_profile_sync");
        assert_eq!(value["project_id"], 9);
        assert_eq!(value["chapter_number"], 2);
        assert!(value["chapters"].as_array().unwrap().is_empty());
    }
}
