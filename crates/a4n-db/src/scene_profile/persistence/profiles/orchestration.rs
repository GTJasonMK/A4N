use std::collections::{BTreeMap, HashMap};

use a4n_core::models::scene::scene_profile;
use a4n_core::profile_ownership::PROFILE_OWNERSHIP_AI_SYNC;
use chrono::Utc;
use sea_orm::sea_query::{CaseStatement, Expr};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};

use super::rules::{
    SceneProfileDraftTracker, merge_scene_profile_record_input, scene_profile_matches_record_input,
};
use crate::scene_profile::{SceneProfileDbResult, SceneProfileRecordDraft};

pub(in crate::scene_profile) async fn upsert_scene_profile_records<C>(
    db: &C,
    project_id: i64,
    drafts: &[SceneProfileRecordDraft],
) -> SceneProfileDbResult<BTreeMap<String, i64>>
where
    C: ConnectionTrait,
{
    let mut tracker = SceneProfileDraftTracker::default();
    for draft in drafts {
        tracker.accept(draft)?;
    }
    if drafts.is_empty() {
        return Ok(BTreeMap::new());
    }

    let explicit_ids = drafts
        .iter()
        .filter_map(|draft| draft.scene_profile_id)
        .collect::<Vec<_>>();
    let normalized_names = drafts
        .iter()
        .filter(|draft| draft.scene_profile_id.is_none())
        .map(|draft| draft.input.normalized_name.clone())
        .collect::<Vec<_>>();
    let explicit_profiles = if explicit_ids.is_empty() {
        Vec::new()
    } else {
        scene_profile::Entity::find()
            .filter(scene_profile::Column::Id.is_in(explicit_ids))
            .all(db)
            .await?
    };
    let named_profiles = if normalized_names.is_empty() {
        Vec::new()
    } else {
        scene_profile::Entity::find()
            .filter(scene_profile::Column::ProjectId.eq(project_id))
            .filter(scene_profile::Column::NormalizedName.is_in(normalized_names))
            .all(db)
            .await?
    };
    let profiles_by_id = explicit_profiles
        .into_iter()
        .map(|profile| (profile.id, profile))
        .collect::<HashMap<_, _>>();
    let profiles_by_name = named_profiles
        .into_iter()
        .map(|profile| (profile.normalized_name.clone(), profile))
        .collect::<HashMap<_, _>>();

    let mut map = BTreeMap::new();
    let mut pending_updates = Vec::new();
    let mut new_rows = Vec::new();
    let mut new_refs = Vec::new();
    for draft in drafts {
        let existing = match draft.scene_profile_id {
            Some(id) => profiles_by_id.get(&id).cloned(),
            None => profiles_by_name.get(&draft.input.normalized_name).cloned(),
        };
        match (draft.scene_profile_id, existing) {
            (Some(id), Some(existing)) => {
                if existing.project_id != project_id {
                    return Err(crate::scene_profile::SceneProfileDbError::InvalidArgument(
                        format!("场景档案 #{id} 不属于项目 {project_id}"),
                    ));
                }
                let input = merge_scene_profile_record_input(&existing, draft.input.clone());
                if !scene_profile_matches_record_input(&existing, &input) {
                    pending_updates.push((existing.id, input));
                }
                map.insert(draft.scene_ref.clone(), id);
            }
            (Some(id), None) => {
                return Err(crate::scene_profile::SceneProfileDbError::NotFound(
                    format!("场景档案不存在: {id}"),
                ));
            }
            (None, Some(existing)) => {
                let input = merge_scene_profile_record_input(&existing, draft.input.clone());
                if !scene_profile_matches_record_input(&existing, &input) {
                    pending_updates.push((existing.id, input));
                }
                let existing_id = existing.id;
                map.insert(draft.scene_ref.clone(), existing_id);
            }
            (None, None) => {
                let now = Utc::now().naive_utc();
                let normalized_name = draft.input.normalized_name.clone();
                new_refs.push((draft.scene_ref.clone(), normalized_name));
                new_rows.push(scene_profile::ActiveModel {
                    project_id: Set(project_id),
                    ownership_kind: Set(PROFILE_OWNERSHIP_AI_SYNC.to_string()),
                    name: Set(draft.input.name.clone()),
                    normalized_name: Set(draft.input.normalized_name.clone()),
                    aliases_json: Set(draft.input.aliases_json.clone()),
                    scene_type: Set(draft.input.scene_type.clone()),
                    importance: Set(draft.input.importance.clone()),
                    parent_scene_id: Set(draft.input.parent_scene_id),
                    base_description: Set(draft.input.base_description.clone()),
                    visual_description: Set(draft.input.visual_description.clone()),
                    spatial_layout: Set(draft.input.spatial_layout.clone()),
                    atmosphere: Set(draft.input.atmosphere.clone()),
                    entry_points_json: Set(draft.input.entry_points_json.clone()),
                    rules_json: Set(draft.input.rules_json.clone()),
                    current_state_json: Set(serde_json::json!({})),
                    first_seen_chapter: Set(draft.input.first_seen_chapter),
                    last_seen_chapter: Set(draft.input.last_seen_chapter),
                    evidence_json: Set(draft.input.evidence_json.clone()),
                    image_prompt_seed: Set(draft.input.image_prompt_seed.clone()),
                    created_at: Set(now),
                    updated_at: Set(now),
                    ..Default::default()
                });
            }
        }
    }

    if !new_rows.is_empty() {
        let expected = new_rows.len();
        let created = scene_profile::Entity::insert_many(new_rows)
            .exec_with_returning_many(db)
            .await?;
        if created.len() != expected {
            return Err(crate::scene_profile::SceneProfileDbError::InvalidState {
                current: format!(
                    "场景档案批量写入返回 {} 行，预期 {} 行",
                    created.len(),
                    expected
                ),
                operation: "批量写入场景档案".to_string(),
            });
        }
        let created_by_name = created
            .into_iter()
            .map(|profile| (profile.normalized_name, profile.id))
            .collect::<HashMap<_, _>>();
        for (scene_ref, normalized_name) in new_refs {
            let id = created_by_name
                .get(&normalized_name)
                .copied()
                .ok_or_else(|| crate::scene_profile::SceneProfileDbError::InvalidState {
                    current: format!("场景档案批量写入结果缺少场景: {normalized_name}"),
                    operation: "读取批量写入场景档案".to_string(),
                })?;
            map.insert(scene_ref, id);
        }
    }

    if !pending_updates.is_empty() {
        let mut name_case = CaseStatement::new();
        let mut normalized_name_case = CaseStatement::new();
        let mut aliases_case = CaseStatement::new();
        let mut scene_type_case = CaseStatement::new();
        let mut importance_case = CaseStatement::new();
        let mut parent_scene_id_case = CaseStatement::new();
        let mut base_description_case = CaseStatement::new();
        let mut visual_description_case = CaseStatement::new();
        let mut spatial_layout_case = CaseStatement::new();
        let mut atmosphere_case = CaseStatement::new();
        let mut entry_points_case = CaseStatement::new();
        let mut rules_case = CaseStatement::new();
        let mut first_seen_chapter_case = CaseStatement::new();
        let mut last_seen_chapter_case = CaseStatement::new();
        let mut evidence_case = CaseStatement::new();
        let mut image_prompt_seed_case = CaseStatement::new();
        let updated_ids = pending_updates
            .iter()
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        for (id, input) in pending_updates {
            name_case = name_case.case(scene_profile::Column::Id.eq(id), Expr::value(input.name));
            normalized_name_case = normalized_name_case.case(
                scene_profile::Column::Id.eq(id),
                Expr::value(input.normalized_name),
            );
            aliases_case = aliases_case.case(
                scene_profile::Column::Id.eq(id),
                Expr::value(input.aliases_json).cast_as("json"),
            );
            scene_type_case = scene_type_case.case(
                scene_profile::Column::Id.eq(id),
                Expr::value(input.scene_type),
            );
            importance_case = importance_case.case(
                scene_profile::Column::Id.eq(id),
                Expr::value(input.importance),
            );
            parent_scene_id_case = parent_scene_id_case.case(
                scene_profile::Column::Id.eq(id),
                Expr::value(input.parent_scene_id),
            );
            base_description_case = base_description_case.case(
                scene_profile::Column::Id.eq(id),
                Expr::value(input.base_description),
            );
            visual_description_case = visual_description_case.case(
                scene_profile::Column::Id.eq(id),
                Expr::value(input.visual_description),
            );
            spatial_layout_case = spatial_layout_case.case(
                scene_profile::Column::Id.eq(id),
                Expr::value(input.spatial_layout),
            );
            atmosphere_case = atmosphere_case.case(
                scene_profile::Column::Id.eq(id),
                Expr::value(input.atmosphere),
            );
            entry_points_case = entry_points_case.case(
                scene_profile::Column::Id.eq(id),
                Expr::value(input.entry_points_json).cast_as("json"),
            );
            rules_case = rules_case.case(
                scene_profile::Column::Id.eq(id),
                Expr::value(input.rules_json).cast_as("json"),
            );
            first_seen_chapter_case = first_seen_chapter_case.case(
                scene_profile::Column::Id.eq(id),
                Expr::value(input.first_seen_chapter),
            );
            last_seen_chapter_case = last_seen_chapter_case.case(
                scene_profile::Column::Id.eq(id),
                Expr::value(input.last_seen_chapter),
            );
            evidence_case = evidence_case.case(
                scene_profile::Column::Id.eq(id),
                Expr::value(input.evidence_json).cast_as("json"),
            );
            image_prompt_seed_case = image_prompt_seed_case.case(
                scene_profile::Column::Id.eq(id),
                Expr::value(input.image_prompt_seed),
            );
        }
        let updated = scene_profile::Entity::update_many()
            .col_expr(
                scene_profile::Column::Name,
                name_case
                    .finally(Expr::col(scene_profile::Column::Name))
                    .into(),
            )
            .col_expr(
                scene_profile::Column::NormalizedName,
                normalized_name_case
                    .finally(Expr::col(scene_profile::Column::NormalizedName))
                    .into(),
            )
            .col_expr(
                scene_profile::Column::AliasesJson,
                aliases_case
                    .finally(Expr::col(scene_profile::Column::AliasesJson))
                    .into(),
            )
            .col_expr(
                scene_profile::Column::SceneType,
                scene_type_case
                    .finally(Expr::col(scene_profile::Column::SceneType))
                    .into(),
            )
            .col_expr(
                scene_profile::Column::Importance,
                importance_case
                    .finally(Expr::col(scene_profile::Column::Importance))
                    .into(),
            )
            .col_expr(
                scene_profile::Column::ParentSceneId,
                parent_scene_id_case
                    .finally(Expr::col(scene_profile::Column::ParentSceneId))
                    .into(),
            )
            .col_expr(
                scene_profile::Column::BaseDescription,
                base_description_case
                    .finally(Expr::col(scene_profile::Column::BaseDescription))
                    .into(),
            )
            .col_expr(
                scene_profile::Column::VisualDescription,
                visual_description_case
                    .finally(Expr::col(scene_profile::Column::VisualDescription))
                    .into(),
            )
            .col_expr(
                scene_profile::Column::SpatialLayout,
                spatial_layout_case
                    .finally(Expr::col(scene_profile::Column::SpatialLayout))
                    .into(),
            )
            .col_expr(
                scene_profile::Column::Atmosphere,
                atmosphere_case
                    .finally(Expr::col(scene_profile::Column::Atmosphere))
                    .into(),
            )
            .col_expr(
                scene_profile::Column::EntryPointsJson,
                entry_points_case
                    .finally(Expr::col(scene_profile::Column::EntryPointsJson))
                    .into(),
            )
            .col_expr(
                scene_profile::Column::RulesJson,
                rules_case
                    .finally(Expr::col(scene_profile::Column::RulesJson))
                    .into(),
            )
            .col_expr(
                scene_profile::Column::FirstSeenChapter,
                first_seen_chapter_case
                    .finally(Expr::col(scene_profile::Column::FirstSeenChapter))
                    .into(),
            )
            .col_expr(
                scene_profile::Column::LastSeenChapter,
                last_seen_chapter_case
                    .finally(Expr::col(scene_profile::Column::LastSeenChapter))
                    .into(),
            )
            .col_expr(
                scene_profile::Column::EvidenceJson,
                evidence_case
                    .finally(Expr::col(scene_profile::Column::EvidenceJson))
                    .into(),
            )
            .col_expr(
                scene_profile::Column::ImagePromptSeed,
                image_prompt_seed_case
                    .finally(Expr::col(scene_profile::Column::ImagePromptSeed))
                    .into(),
            )
            .col_expr(
                scene_profile::Column::UpdatedAt,
                Expr::value(Utc::now().naive_utc()),
            )
            .filter(scene_profile::Column::ProjectId.eq(project_id))
            .filter(scene_profile::Column::Id.is_in(updated_ids.clone()))
            .exec(db)
            .await?;
        if updated.rows_affected != updated_ids.len() as u64 {
            return Err(crate::scene_profile::SceneProfileDbError::InvalidState {
                current: format!(
                    "场景档案批量更新影响 {} 行，预期 {} 行",
                    updated.rows_affected,
                    updated_ids.len()
                ),
                operation: "批量更新场景档案".to_string(),
            });
        }
    }
    Ok(map)
}
