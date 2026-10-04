use a4n_db::project::family::import::normalize_profile_evidence_role;
use std::collections::HashSet;

use super::super::super::ProjectFamilyPackage;

pub(in super::super) fn validate_profile_evidence_manifest(
    package: &ProjectFamilyPackage,
) -> Result<(), String> {
    let text_anchor_ids = package
        .text_anchors
        .iter()
        .map(|anchor| anchor.id)
        .collect::<HashSet<_>>();
    let character_profile_ids = package
        .text_core
        .character_profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<HashSet<_>>();
    let object_profile_ids = package
        .text_core
        .object_profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<HashSet<_>>();
    let scene_profile_ids = package
        .text_core
        .scene_profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<HashSet<_>>();
    let mut seen = HashSet::new();
    for evidence in &package.profile_evidence {
        if !text_anchor_ids.contains(&evidence.text_anchor_id) {
            return Err(format!(
                "项目族导入包档案证据引用了不存在的文本锚点: profile_evidence_id={}, text_anchor_id={}",
                evidence.id, evidence.text_anchor_id
            ));
        }
        validate_profile_evidence_profile_id(
            evidence,
            &character_profile_ids,
            &object_profile_ids,
            &scene_profile_ids,
        )?;
        let evidence_role = normalize_profile_evidence_role(&evidence.evidence_role)
            .map_err(|error| error.to_string())?;
        if !seen.insert((
            evidence.profile_kind.clone(),
            evidence.profile_id,
            evidence.text_anchor_id,
            evidence_role.to_string(),
        )) {
            return Err(format!(
                "项目族导入包档案证据重复: profile_kind={}, profile_id={}, text_anchor_id={}, evidence_role={}",
                evidence.profile_kind, evidence.profile_id, evidence.text_anchor_id, evidence_role
            ));
        }
    }
    Ok(())
}

fn validate_profile_evidence_profile_id(
    evidence: &a4n_domain::project::ProjectFamilyProfileEvidenceRecord,
    character_profile_ids: &HashSet<i64>,
    object_profile_ids: &HashSet<i64>,
    scene_profile_ids: &HashSet<i64>,
) -> Result<(), String> {
    let exists = match evidence.profile_kind.as_str() {
        "character" => character_profile_ids.contains(&evidence.profile_id),
        "object" => object_profile_ids.contains(&evidence.profile_id),
        "scene" => scene_profile_ids.contains(&evidence.profile_id),
        other => {
            return Err(format!(
                "项目族导入包档案证据 profile_kind 无效: profile_evidence_id={}, profile_kind={other}",
                evidence.id
            ));
        }
    };
    if !exists {
        return Err(format!(
            "项目族导入包档案证据引用了不存在的档案: profile_evidence_id={}, profile_kind={}, profile_id={}",
            evidence.id, evidence.profile_kind, evidence.profile_id
        ));
    }
    Ok(())
}
