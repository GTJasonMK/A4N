use super::validation::ensure_profile_belongs_to_root;
use super::{
    TextAnchorDbResult, TextAnchorEvidenceInput, TextAnchorProfileKind, validate_anchor_ref_records,
};
use a4n_core::models::project::profile_evidence;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, Set};
use std::collections::BTreeSet;

pub async fn merge_profile_evidence_records(
    db: &DatabaseConnection,
    root_project_id: i64,
    source_snapshot_id: Option<i64>,
    profile_kind: TextAnchorProfileKind,
    profile_id: i64,
    refs: Vec<TextAnchorEvidenceInput>,
) -> TextAnchorDbResult<Vec<profile_evidence::Model>> {
    let profile_kind_value = profile_kind.as_str();
    ensure_profile_belongs_to_root(db, root_project_id, profile_kind, profile_id).await?;
    let anchor_ids = refs
        .iter()
        .map(|item| item.text_anchor_id)
        .collect::<Vec<_>>();
    validate_anchor_ref_records(db, root_project_id, source_snapshot_id, &[], &anchor_ids).await?;

    let existing = profile_evidence::Entity::find()
        .filter(profile_evidence::Column::ProfileKind.eq(profile_kind_value))
        .filter(profile_evidence::Column::ProfileId.eq(profile_id))
        .all(db)
        .await?
        .into_iter()
        .map(|item| (item.text_anchor_id, item.evidence_role))
        .collect::<BTreeSet<_>>();
    let now = Utc::now().naive_utc();
    let mut inserted = Vec::new();
    for item in refs {
        if existing.contains(&(item.text_anchor_id, item.evidence_role.clone())) {
            continue;
        }
        inserted.push(
            profile_evidence::ActiveModel {
                profile_kind: Set(profile_kind_value.to_string()),
                profile_id: Set(profile_id),
                text_anchor_id: Set(item.text_anchor_id),
                evidence_role: Set(item.evidence_role),
                created_at: Set(now),
                ..Default::default()
            }
            .insert(db)
            .await?,
        );
    }
    Ok(inserted)
}
