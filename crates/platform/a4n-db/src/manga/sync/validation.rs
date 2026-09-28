use crate::manga::{MangaPipelineDbError, validate_manga_page_prompt_layout};
use a4n_domain::manga::MangaPagePromptSyncDraft;
use std::collections::HashSet;

pub(super) async fn validate_adopted_manga_plan(
    db: &sea_orm::DatabaseTransaction,
    project_id: i64,
    chapter_id: i64,
    drafts: &[MangaPagePromptSyncDraft],
) -> Result<(), MangaPipelineDbError> {
    use a4n_db_models::models::project::novel_project;
    use a4n_domain::{
        manga::{MangaPageGenerationLayoutV3, MangaPagePlanV4},
        media_authoring::{AuthoringDocumentKind, AuthoringDocumentStatus},
    };
    use sea_orm::{EntityTrait, QuerySelect};
    let invalid = |message: &str| MangaPipelineDbError::InvalidArgument(message.into());
    let project = novel_project::Entity::find_by_id(project_id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| invalid("漫画项目不存在"))?;
    if project.project_kind != "comic" || project.lifecycle_gate != "active" {
        return Err(invalid("漫画项目不可写"));
    }
    let layouts = drafts
        .iter()
        .map(|draft| {
            serde_json::from_value::<MangaPageGenerationLayoutV3>(
                draft
                    .layout
                    .clone()
                    .ok_or_else(|| invalid("漫画页面缺少布局"))?,
            )
            .map_err(|error| MangaPipelineDbError::InvalidArgument(error.to_string()))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let first = layouts
        .first()
        .ok_or_else(|| invalid("生成漫画页面不能为空"))?;
    let reference = first
        .page_plan_ref
        .as_ref()
        .ok_or_else(|| invalid("生成漫画页面必须记录已采用分页计划"))?;
    let plan = crate::media_authoring::authoring_document(db, project_id, reference.id)
        .await
        .map_err(authoring_error)?;
    if plan.kind != AuthoringDocumentKind::MangaPagePlan
        || plan.status != AuthoringDocumentStatus::Adopted
        || plan.reference() != *reference
        || plan.source_chapter_id != Some(chapter_id)
        || Some(plan.source_snapshot_id) != project.source_snapshot_id
    {
        return Err(invalid("已采用分页计划或原文已变化，请重新准备页面"));
    }
    let ([source], [dependency]) = (
        plan.inputs.sources.as_slice(),
        plan.inputs.dependencies.as_slice(),
    ) else {
        return Err(invalid("分页计划必须对应一个原文章节和改编版本"));
    };
    if !first.source.matches_authoring(source, dependency) {
        return Err(invalid("漫画页面来源与采用的改编版本不一致"));
    }
    crate::media_authoring::validate_adopted_authoring_dependencies(
        db,
        project_id,
        &plan.inputs.dependencies,
    )
    .await
    .map_err(authoring_error)?;
    let document: MangaPagePlanV4 = serde_json::from_value(plan.document)
        .map_err(|error| MangaPipelineDbError::InvalidArgument(error.to_string()))?;
    if document.adaptation_plan_id != dependency.id
        || document.adaptation_plan_revision != dependency.revision
        || document.pages.len() != layouts.len()
        || layouts.iter().any(|layout| {
            layout.page_plan_ref != first.page_plan_ref
                || layout.source != first.source
                || !document.pages.contains(&layout.page_plan)
        })
    {
        return Err(invalid("漫画页面与实际采用的分页计划不一致"));
    }
    Ok(())
}

fn authoring_error(error: crate::media_authoring::AuthoringDocumentError) -> MangaPipelineDbError {
    match error {
        crate::media_authoring::AuthoringDocumentError::Database(error) => {
            MangaPipelineDbError::Database(error)
        }
        error => MangaPipelineDbError::InvalidArgument(error.to_string()),
    }
}

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
                "schema_version": "manga_page_generation_v3",
                "page_plan": {
                    "page_number": 1,
                    "beat_refs": [{"unit_id": "unit-1", "beat_id": "beat-1"}],
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
                        "beat_refs": [{"unit_id": "unit-1", "beat_id": "beat-1"}],
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
                    "adaptation_plan_id": 71,
                    "adaptation_plan_revision": 2,
                    "adaptation_plan_sha256": "plan-sha256"
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
