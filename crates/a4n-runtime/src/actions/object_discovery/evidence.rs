use super::types::ParsedObjectProfileDraft;
use crate::actions::app_error_message;
use crate::state::AppState;
use a4n_orchestrator::asset::ObjectProfileUpsertRecordResult;
use a4n_orchestrator::asset::normalize_object_name;
use a4n_orchestrator::text_anchor::EVIDENCE_ROLE_FIRST_SEEN;
use a4n_orchestrator::text_anchor::{TextAnchorEvidenceInput, TextAnchorProfileKind};
use std::collections::{BTreeSet, HashMap};

pub(super) async fn bind_object_profile_evidence(
    state: &AppState,
    project_id: i64,
    drafts: &[ParsedObjectProfileDraft],
    result: &ObjectProfileUpsertRecordResult,
) -> Result<(), String> {
    let profiles = result
        .created
        .iter()
        .chain(result.updated.iter())
        .map(|profile| (profile.normalized_name.clone(), profile.id))
        .collect::<HashMap<_, _>>();
    for draft in drafts {
        let normalized = normalize_object_name(&draft.draft.name).map_err(|err| err.to_string())?;
        let profile_id = profiles
            .get(&normalized)
            .copied()
            .ok_or_else(|| format!("物体档案写入结果缺少对象: {}", draft.draft.name))?;
        let mut refs = Vec::new();
        let mut seen = BTreeSet::new();
        for anchor_id in &draft.evidence_anchor_ids {
            if seen.insert((*anchor_id, EVIDENCE_ROLE_FIRST_SEEN)) {
                refs.push(TextAnchorEvidenceInput {
                    text_anchor_id: *anchor_id,
                    evidence_role: EVIDENCE_ROLE_FIRST_SEEN.to_string(),
                });
            }
        }
        if refs.is_empty() {
            return Err("参数无效：profile_evidence 不能为空".to_string());
        }
        state
            .orchestrator
            .merge_profile_evidence_records(
                project_id,
                None,
                TextAnchorProfileKind::Object,
                profile_id,
                refs,
            )
            .await
            .map_err(app_error_message)?;
    }
    Ok(())
}
