use crate::manga::{MangaPipelineDbError, validate_manga_page_prompt_layout};
use a4n_domain::manga::MangaPagePromptSyncDraft;
use std::collections::HashSet;

pub(super) fn validate_manga_sync_drafts(
    drafts: &[MangaPagePromptSyncDraft],
    project_id: i64,
    chapter_id: i64,
) -> Result<(), MangaPipelineDbError> {
    let mut identities = HashSet::with_capacity(drafts.len());
    for draft in drafts {
        if draft.project_id != project_id || draft.chapter_id != chapter_id {
            return Err(MangaPipelineDbError::InvalidArgument(
                "漫画同步草稿的 project_id/chapter_id 必须与同步范围一致".to_string(),
            ));
        }
        validate_manga_page_prompt_layout(draft.page_number, draft.layout.as_ref())?;
        if !identities.insert(draft.page_number) {
            return Err(MangaPipelineDbError::InvalidArgument(
                "漫画同步草稿包含重复 page_number".to_string(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn page_draft(project_id: i64, chapter_id: i64) -> MangaPagePromptSyncDraft {
        MangaPagePromptSyncDraft {
            chapter_id,
            project_id,
            page_number: 1,
            image_prompt: "镜头".to_string(),
            dialogue_text: None,
            characters_in_frame: None,
            layout: Some(json!({
                "schema_version": "manga_page_generation_v2",
                "page_plan": {
                    "page_number": 1,
                    "beat_refs": [{"scene_index": 1, "shot_index": 1, "beat_index": 1}],
                    "role": "setup",
                    "pacing": "medium",
                    "importance": "normal",
                    "reading_flow": "top_to_bottom_left_to_right",
                    "page_goal": "推进场景",
                    "visual_thesis": "角色对峙",
                    "entry_state": "平静",
                    "exit_state": "紧张",
                    "dominant_panel_number": 1,
                    "panels": [{
                        "panel_number": 1,
                        "beat_refs": [{"scene_index": 1, "shot_index": 1, "beat_index": 1}],
                        "function": "dialogue",
                        "size": "dominant",
                        "shot_scale": "medium",
                        "camera_angle": "eye_level",
                        "participants": ["hero"],
                        "composition": "单人构图",
                        "observable_action": "抬头",
                        "reaction": null,
                        "spatial_relationship": "中央",
                        "continuity_from_previous": null,
                        "text_layers": []
                    }]
                },
                "drawing_prompt": {
                    "schema_version": "manga_page_drawing_prompt_v2",
                    "page_number": 1,
                    "image_prompt": "整页"
                },
                "reference_bindings": [],
                "prompt_fingerprint": {"hash": "sha256"},
                "source": {
                    "root_project_id": 11,
                    "source_chapter_id": 12,
                    "source_snapshot_id": null,
                    "source_snapshot_chapter_id": null,
                    "source_revision_key": "live:12:content-sha256",
                    "content_sha256": "content-sha256",
                    "shooting_script_id": 71,
                    "shooting_script_revision": 2,
                    "shooting_script_sha256": "script-sha256"
                }
            })),
            status: "pending".to_string(),
        }
    }

    #[test]
    fn manga_sync_validation_checks_scope_before_prompt_identity() {
        let draft = page_draft(11, 12);

        let error = validate_manga_sync_drafts(&[draft], 21, 22).unwrap_err();

        assert!(matches!(
            error,
            MangaPipelineDbError::InvalidArgument(message)
                if message
                    == "漫画同步草稿的 project_id/chapter_id 必须与同步范围一致"
        ));
    }

    #[test]
    fn manga_sync_validation_accepts_valid_identity_and_rejects_duplicates() {
        let draft = page_draft(11, 12);
        assert!(validate_manga_sync_drafts(std::slice::from_ref(&draft), 11, 12).is_ok());

        let error = validate_manga_sync_drafts(&[draft.clone(), draft], 11, 12).unwrap_err();
        assert!(matches!(
            error,
            MangaPipelineDbError::InvalidArgument(message)
                if message
                    == "漫画同步草稿包含重复 page_number"
        ));
    }
}
