use super::types::{SceneProfileEvidenceRefs, SceneProfileSyncRequestContext};
use crate::actions::app_error_message;
use crate::actions::text_anchor_contract::anchor_quotes_in_exact_chapter;
use crate::state::AppState;
use a4n_orchestrator::text_anchor::EVIDENCE_ROLE_FIRST_SEEN;
use a4n_orchestrator::text_anchor::{TextAnchorEvidenceInput, TextAnchorProfileKind};

pub(super) fn resolve_scene_text_anchor_texts(
    path: &str,
    anchor_ids: &[i64],
    context: &SceneProfileSyncRequestContext,
) -> Result<Vec<String>, String> {
    anchor_quotes_in_exact_chapter(
        path,
        anchor_ids,
        &context.text_anchors,
        context.chapter.id,
        "当前章节",
    )
}

pub(super) async fn bind_scene_profile_evidence(
    state: &AppState,
    project_id: i64,
    refs: &[SceneProfileEvidenceRefs],
    result: &a4n_orchestrator::scene_profile::SceneSyncResult,
) -> Result<(), String> {
    for item in refs {
        let profile_id = result
            .profile_ref_ids
            .get(&item.scene_ref)
            .copied()
            .ok_or_else(|| format!("场景档案写入结果缺少 scene_ref: {}", item.scene_ref))?;
        let evidence_refs = item
            .evidence_anchor_ids
            .iter()
            .copied()
            .map(|text_anchor_id| TextAnchorEvidenceInput {
                text_anchor_id,
                evidence_role: EVIDENCE_ROLE_FIRST_SEEN.to_string(),
            })
            .collect::<Vec<_>>();
        if evidence_refs.is_empty() {
            return Err("参数无效：profile_evidence 不能为空".to_string());
        }
        state
            .orchestrator
            .merge_profile_evidence_records(
                project_id,
                None,
                TextAnchorProfileKind::Scene,
                profile_id,
                evidence_refs,
            )
            .await
            .map_err(app_error_message)?;
    }
    Ok(())
}
