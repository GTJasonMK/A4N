use crate::manga::{MangaPipelineDbError, MangaPromptSyncDraft, validate_manga_prompt_identity};
use std::collections::HashSet;

pub(super) fn validate_manga_sync_drafts(
    drafts: &[MangaPromptSyncDraft],
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
        validate_manga_prompt_identity(
            &draft.prompt_type,
            draft.page_number,
            draft.panel_number,
            draft.layout.as_ref(),
        )?;
        let identity = (
            draft.prompt_type.clone(),
            draft.page_number,
            draft.panel_number,
        );
        if !identities.insert(identity) {
            return Err(MangaPipelineDbError::InvalidArgument(
                "漫画同步草稿包含重复 prompt_type/page_number/panel_number".to_string(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::manga::MANGA_PAGE_PROMPT_TYPE;
    use serde_json::json;

    fn page_draft(project_id: i64, chapter_id: i64) -> MangaPromptSyncDraft {
        MangaPromptSyncDraft {
            chapter_id,
            project_id,
            page_number: 1,
            panel_number: 0,
            prompt_type: MANGA_PAGE_PROMPT_TYPE.to_string(),
            image_prompt: "镜头".to_string(),
            dialogue_text: None,
            characters_in_frame: None,
            layout: Some(json!({
                "schema_version": "manga_page_generation_v2",
                "shooting_script_id": 71,
                "shooting_script_revision": 2
            })),
            camera_angle: None,
            dialogue_position: None,
            status: "pending".to_string(),
        }
    }

    #[test]
    fn manga_sync_validation_checks_scope_before_prompt_identity() {
        let mut draft = page_draft(11, 12);
        draft.prompt_type = "legacy".to_string();

        let error = validate_manga_sync_drafts(&[draft], 21, 22).unwrap_err();

        assert!(matches!(
            error,
            MangaPipelineDbError::InvalidArgument(message)
                if message
                    == "漫画同步草稿的 project_id/chapter_id 必须与同步范围一致"
        ));
    }

    #[test]
    fn manga_sync_validation_accepts_valid_identity_and_rejects_duplicates_or_legacy_types() {
        let draft = page_draft(11, 12);
        assert!(validate_manga_sync_drafts(std::slice::from_ref(&draft), 11, 12).is_ok());

        let error = validate_manga_sync_drafts(&[draft.clone(), draft], 11, 12).unwrap_err();
        assert!(matches!(
            error,
            MangaPipelineDbError::InvalidArgument(message)
                if message
                    == "漫画同步草稿包含重复 prompt_type/page_number/panel_number"
        ));

        let mut legacy = page_draft(11, 12);
        legacy.prompt_type = "page_planning".to_string();
        let error = validate_manga_sync_drafts(&[legacy], 11, 12).unwrap_err();
        assert!(matches!(
            error,
            MangaPipelineDbError::InvalidArgument(message)
                if message == "不支持的漫画 prompt_type: page_planning"
        ));
    }
}
