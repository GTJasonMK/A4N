use std::collections::{HashMap, HashSet};

use a4n_db_models::models::asset::object_profile;
#[cfg(test)]
use a4n_domain::auth::ROLE_USER;
use a4n_domain::profile_ownership::PROFILE_OWNERSHIP_AI_SYNC;
use a4n_domain::profile_text::merge_profile_evidence_json;
use chrono::Utc;
use sea_orm::sea_query::{CaseStatement, Expr};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

use crate::asset::AssetDbError;
use crate::asset::contracts::ObjectProfileUpsertModels;
use a4n_domain::object_profile::ObjectProfileRecordInput;

use super::record::object_profile_active_model;

pub(in crate::asset) async fn upsert_object_profiles_in<C>(
    db: &C,
    project_id: i64,
    inputs: Vec<ObjectProfileRecordInput>,
) -> Result<ObjectProfileUpsertModels, AssetDbError>
where
    C: ConnectionTrait,
{
    let mut result = ObjectProfileUpsertModels::default();
    let mut seen = HashSet::new();
    let normalized_names = inputs
        .iter()
        .map(|input| input.normalized_name.clone())
        .collect::<Vec<_>>();
    for input in &inputs {
        if !seen.insert(input.normalized_name.clone()) {
            return Err(AssetDbError::InvalidArgument(format!(
                "物体发现结果存在重复物体: {}",
                input.name
            )));
        }
    }
    let existing_by_name = if normalized_names.is_empty() {
        HashMap::new()
    } else {
        object_profile::Entity::find()
            .filter(object_profile::Column::ProjectId.eq(project_id))
            .filter(object_profile::Column::NormalizedName.is_in(normalized_names))
            .all(db)
            .await?
            .into_iter()
            .map(|profile| (profile.normalized_name.clone(), profile))
            .collect::<HashMap<_, _>>()
    };
    let mut created_rows = Vec::new();
    let mut pending_updates = Vec::new();
    let mut updated_order = Vec::new();
    let mut unchanged_updates = HashMap::new();
    for input in inputs {
        match existing_by_name.get(&input.normalized_name).cloned() {
            Some(model) => {
                updated_order.push(model.id);
                let merged = merge_object_profile_record_input(&model, input);
                if !object_profile_matches_input(&model, &merged) {
                    pending_updates.push((model.id, merged));
                } else {
                    unchanged_updates.insert(model.id, model);
                }
            }
            None => created_rows.push(object_profile_active_model(
                project_id,
                input,
                PROFILE_OWNERSHIP_AI_SYNC,
                Utc::now().naive_utc(),
            )),
        }
    }
    if !created_rows.is_empty() {
        let expected = created_rows.len();
        let created = object_profile::Entity::insert_many(created_rows)
            .exec_with_returning_many(db)
            .await?;
        if created.len() != expected {
            return Err(AssetDbError::InvalidState {
                current: format!(
                    "物体档案批量写入返回 {} 行，预期 {} 行",
                    created.len(),
                    expected
                ),
                operation: "批量写入物体档案".to_string(),
            });
        }
        result.created = created;
    }
    if !pending_updates.is_empty() {
        let mut name_case = CaseStatement::new();
        let mut normalized_name_case = CaseStatement::new();
        let mut aliases_case = CaseStatement::new();
        let mut object_type_case = CaseStatement::new();
        let mut importance_case = CaseStatement::new();
        let mut visual_description_case = CaseStatement::new();
        let mut material_and_state_case = CaseStatement::new();
        let mut story_usage_case = CaseStatement::new();
        let mut visual_usage_case = CaseStatement::new();
        let mut generation_usage_case = CaseStatement::new();
        let mut location_context_case = CaseStatement::new();
        let mut first_seen_chapter_case = CaseStatement::new();
        let mut last_seen_chapter_case = CaseStatement::new();
        let mut evidence_case = CaseStatement::new();
        let mut image_prompt_seed_case = CaseStatement::new();
        let updated_ids = pending_updates
            .iter()
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        for (id, input) in pending_updates {
            name_case = name_case.case(object_profile::Column::Id.eq(id), Expr::value(input.name));
            normalized_name_case = normalized_name_case.case(
                object_profile::Column::Id.eq(id),
                Expr::value(input.normalized_name),
            );
            aliases_case = aliases_case.case(
                object_profile::Column::Id.eq(id),
                Expr::value(input.aliases_json).cast_as("json"),
            );
            object_type_case = object_type_case.case(
                object_profile::Column::Id.eq(id),
                Expr::value(input.object_type),
            );
            importance_case = importance_case.case(
                object_profile::Column::Id.eq(id),
                Expr::value(input.importance),
            );
            visual_description_case = visual_description_case.case(
                object_profile::Column::Id.eq(id),
                Expr::value(input.visual_description),
            );
            material_and_state_case = material_and_state_case.case(
                object_profile::Column::Id.eq(id),
                Expr::value(input.material_and_state),
            );
            story_usage_case = story_usage_case.case(
                object_profile::Column::Id.eq(id),
                Expr::value(input.story_usage),
            );
            visual_usage_case = visual_usage_case.case(
                object_profile::Column::Id.eq(id),
                Expr::value(input.visual_usage),
            );
            generation_usage_case = generation_usage_case.case(
                object_profile::Column::Id.eq(id),
                Expr::value(input.generation_usage),
            );
            location_context_case = location_context_case.case(
                object_profile::Column::Id.eq(id),
                Expr::value(input.location_context),
            );
            first_seen_chapter_case = first_seen_chapter_case.case(
                object_profile::Column::Id.eq(id),
                Expr::value(input.first_seen_chapter),
            );
            last_seen_chapter_case = last_seen_chapter_case.case(
                object_profile::Column::Id.eq(id),
                Expr::value(input.last_seen_chapter),
            );
            evidence_case = evidence_case.case(
                object_profile::Column::Id.eq(id),
                Expr::value(input.evidence_json).cast_as("json"),
            );
            image_prompt_seed_case = image_prompt_seed_case.case(
                object_profile::Column::Id.eq(id),
                Expr::value(input.image_prompt_seed),
            );
        }
        let now = Utc::now().naive_utc();
        let updated = object_profile::Entity::update_many()
            .col_expr(
                object_profile::Column::Name,
                name_case
                    .finally(Expr::col(object_profile::Column::Name))
                    .into(),
            )
            .col_expr(
                object_profile::Column::NormalizedName,
                normalized_name_case
                    .finally(Expr::col(object_profile::Column::NormalizedName))
                    .into(),
            )
            .col_expr(
                object_profile::Column::AliasesJson,
                aliases_case
                    .finally(Expr::col(object_profile::Column::AliasesJson))
                    .into(),
            )
            .col_expr(
                object_profile::Column::ObjectType,
                object_type_case
                    .finally(Expr::col(object_profile::Column::ObjectType))
                    .into(),
            )
            .col_expr(
                object_profile::Column::Importance,
                importance_case
                    .finally(Expr::col(object_profile::Column::Importance))
                    .into(),
            )
            .col_expr(
                object_profile::Column::VisualDescription,
                visual_description_case
                    .finally(Expr::col(object_profile::Column::VisualDescription))
                    .into(),
            )
            .col_expr(
                object_profile::Column::MaterialAndState,
                material_and_state_case
                    .finally(Expr::col(object_profile::Column::MaterialAndState))
                    .into(),
            )
            .col_expr(
                object_profile::Column::StoryUsage,
                story_usage_case
                    .finally(Expr::col(object_profile::Column::StoryUsage))
                    .into(),
            )
            .col_expr(
                object_profile::Column::VisualUsage,
                visual_usage_case
                    .finally(Expr::col(object_profile::Column::VisualUsage))
                    .into(),
            )
            .col_expr(
                object_profile::Column::GenerationUsage,
                generation_usage_case
                    .finally(Expr::col(object_profile::Column::GenerationUsage))
                    .into(),
            )
            .col_expr(
                object_profile::Column::LocationContext,
                location_context_case
                    .finally(Expr::col(object_profile::Column::LocationContext))
                    .into(),
            )
            .col_expr(
                object_profile::Column::FirstSeenChapter,
                first_seen_chapter_case
                    .finally(Expr::col(object_profile::Column::FirstSeenChapter))
                    .into(),
            )
            .col_expr(
                object_profile::Column::LastSeenChapter,
                last_seen_chapter_case
                    .finally(Expr::col(object_profile::Column::LastSeenChapter))
                    .into(),
            )
            .col_expr(
                object_profile::Column::EvidenceJson,
                evidence_case
                    .finally(Expr::col(object_profile::Column::EvidenceJson))
                    .into(),
            )
            .col_expr(
                object_profile::Column::ImagePromptSeed,
                image_prompt_seed_case
                    .finally(Expr::col(object_profile::Column::ImagePromptSeed))
                    .into(),
            )
            .col_expr(object_profile::Column::UpdatedAt, Expr::value(now))
            .filter(object_profile::Column::ProjectId.eq(project_id))
            .filter(object_profile::Column::Id.is_in(updated_ids.clone()))
            .exec(db)
            .await?;
        if updated.rows_affected != updated_ids.len() as u64 {
            return Err(AssetDbError::InvalidState {
                current: format!(
                    "物体档案批量更新影响 {} 行，预期 {} 行",
                    updated.rows_affected,
                    updated_ids.len()
                ),
                operation: "批量更新物体档案".to_string(),
            });
        }
        let updated_by_id = object_profile::Entity::find()
            .filter(object_profile::Column::ProjectId.eq(project_id))
            .filter(object_profile::Column::Id.is_in(updated_ids.clone()))
            .all(db)
            .await?
            .into_iter()
            .map(|profile| (profile.id, profile))
            .collect::<HashMap<_, _>>();
        unchanged_updates.extend(updated_by_id);
    }
    for id in updated_order {
        result
            .updated
            .push(
                unchanged_updates
                    .remove(&id)
                    .ok_or_else(|| AssetDbError::InvalidState {
                        current: format!("物体档案批量更新结果缺少记录: {id}"),
                        operation: "读取批量更新物体档案".to_string(),
                    })?,
            );
    }
    Ok(result)
}

fn merge_object_profile_record_input(
    existing: &object_profile::Model,
    mut input: ObjectProfileRecordInput,
) -> ObjectProfileRecordInput {
    let range_unconfirmed = existing.evidence_json.as_array().is_some_and(Vec::is_empty);
    let first_seen_chapter = if range_unconfirmed {
        input.first_seen_chapter
    } else {
        existing.first_seen_chapter.min(input.first_seen_chapter)
    };
    let last_seen_chapter = if range_unconfirmed {
        input.last_seen_chapter
    } else {
        existing.last_seen_chapter.max(input.last_seen_chapter)
    };
    let evidence_json = merge_profile_evidence_json(&existing.evidence_json, &input.evidence_json);
    let aliases_json = merge_profile_evidence_json(&existing.aliases_json, &input.aliases_json);
    if existing.ownership_kind != PROFILE_OWNERSHIP_AI_SYNC {
        input = merge_user_object_profile_fields(existing, input);
    }
    input.first_seen_chapter = first_seen_chapter;
    input.last_seen_chapter = last_seen_chapter;
    input.evidence_json = evidence_json;
    input.aliases_json = aliases_json;
    input
}

fn merge_user_object_profile_fields(
    existing: &object_profile::Model,
    incoming: ObjectProfileRecordInput,
) -> ObjectProfileRecordInput {
    let mut merged = object_profile_record_input(existing);
    if existing.aliases_json.as_array().is_some_and(Vec::is_empty) {
        merged.aliases_json = incoming.aliases_json;
    }
    merged.object_type = incoming.object_type;
    fill_blank(&mut merged.importance, incoming.importance);
    merged.visual_description = incoming.visual_description;
    merged.material_and_state = incoming.material_and_state;
    fill_blank(&mut merged.story_usage, incoming.story_usage);
    fill_blank(&mut merged.visual_usage, incoming.visual_usage);
    fill_blank(&mut merged.generation_usage, incoming.generation_usage);
    if incoming.location_context.is_some() {
        merged.location_context = incoming.location_context;
    }
    fill_blank(&mut merged.image_prompt_seed, incoming.image_prompt_seed);
    merged
}

fn fill_blank(current: &mut String, incoming: String) {
    if current.trim().is_empty() {
        *current = incoming;
    }
}

fn object_profile_record_input(existing: &object_profile::Model) -> ObjectProfileRecordInput {
    ObjectProfileRecordInput {
        name: existing.name.clone(),
        normalized_name: existing.normalized_name.clone(),
        aliases_json: existing.aliases_json.clone(),
        object_type: existing.object_type.clone(),
        importance: existing.importance.clone(),
        visual_description: existing.visual_description.clone(),
        material_and_state: existing.material_and_state.clone(),
        story_usage: existing.story_usage.clone(),
        visual_usage: existing.visual_usage.clone(),
        generation_usage: existing.generation_usage.clone(),
        location_context: existing.location_context.clone(),
        first_seen_chapter: existing.first_seen_chapter,
        last_seen_chapter: existing.last_seen_chapter,
        evidence_json: existing.evidence_json.clone(),
        image_prompt_seed: existing.image_prompt_seed.clone(),
    }
}

fn object_profile_matches_input(
    existing: &object_profile::Model,
    input: &ObjectProfileRecordInput,
) -> bool {
    existing.name == input.name
        && existing.normalized_name == input.normalized_name
        && existing.aliases_json == input.aliases_json
        && existing.object_type == input.object_type
        && existing.importance == input.importance
        && existing.visual_description == input.visual_description
        && existing.material_and_state == input.material_and_state
        && existing.story_usage == input.story_usage
        && existing.visual_usage == input.visual_usage
        && existing.generation_usage == input.generation_usage
        && existing.location_context == input.location_context
        && existing.first_seen_chapter == input.first_seen_chapter
        && existing.last_seen_chapter == input.last_seen_chapter
        && existing.evidence_json == input.evidence_json
        && existing.image_prompt_seed == input.image_prompt_seed
}

#[cfg(test)]
mod tests {
    use a4n_domain::profile_ownership::{PROFILE_OWNERSHIP_AI_SYNC, PROFILE_OWNERSHIP_USER};
    use chrono::NaiveDateTime;
    use serde_json::json;

    use super::*;

    fn profile(ownership_kind: &str) -> object_profile::Model {
        object_profile::Model {
            id: 1,
            project_id: 2,
            ownership_kind: ownership_kind.to_string(),
            name: "用户怀表".to_string(),
            normalized_name: "用户怀表".to_string(),
            aliases_json: json!(["旧称"]),
            object_type: "token".to_string(),
            importance: "primary".to_string(),
            visual_description: "用户外观".to_string(),
            material_and_state: "用户状态".to_string(),
            story_usage: "用户用途".to_string(),
            visual_usage: "用户视觉用途".to_string(),
            generation_usage: "用户生成用途".to_string(),
            location_context: Some("用户位置".to_string()),
            first_seen_chapter: 2,
            last_seen_chapter: 4,
            evidence_json: json!(["旧证据"]),
            image_prompt_seed: "用户提示词".to_string(),
            created_at: NaiveDateTime::MIN,
            updated_at: NaiveDateTime::MIN,
        }
    }

    fn discovery_input() -> ObjectProfileRecordInput {
        ObjectProfileRecordInput {
            name: "AI 怀表".to_string(),
            normalized_name: "用户怀表".to_string(),
            aliases_json: json!(["AI 别名"]),
            object_type: "prop".to_string(),
            importance: "minor".to_string(),
            visual_description: "AI 外观".to_string(),
            material_and_state: "AI 状态".to_string(),
            story_usage: "AI 用途".to_string(),
            visual_usage: "AI 视觉用途".to_string(),
            generation_usage: "AI 生成用途".to_string(),
            location_context: Some("AI 位置".to_string()),
            first_seen_chapter: 1,
            last_seen_chapter: 8,
            evidence_json: json!(["新证据"]),
            image_prompt_seed: "AI 提示词".to_string(),
        }
    }

    #[test]
    fn profile_merge_updates_facts_and_preserves_manual_usage_and_prompts() {
        let mut existing = profile(PROFILE_OWNERSHIP_USER);
        existing.material_and_state.clear();
        let merged = merge_object_profile_record_input(&existing, discovery_input());

        assert_eq!(merged.name, existing.name);
        assert_eq!(merged.visual_description, "AI 外观");
        assert_eq!(merged.story_usage, existing.story_usage);
        assert_eq!(merged.visual_usage, existing.visual_usage);
        assert_eq!(merged.generation_usage, existing.generation_usage);
        assert_eq!(merged.image_prompt_seed, existing.image_prompt_seed);
        assert_eq!(merged.material_and_state, "AI 状态");
        assert_eq!(merged.first_seen_chapter, 1);
        assert_eq!(merged.last_seen_chapter, 8);
        assert_eq!(merged.evidence_json, json!(["新证据", "旧证据"]));
    }

    #[test]
    fn profile_merge_can_refresh_ai_owned_fields() {
        let existing = profile(PROFILE_OWNERSHIP_AI_SYNC);
        let merged = merge_object_profile_record_input(&existing, discovery_input());

        assert_eq!(merged.name, "AI 怀表");
        assert_eq!(merged.visual_description, "AI 外观");
        assert_eq!(merged.first_seen_chapter, 1);
        assert_eq!(merged.last_seen_chapter, 8);
    }

    #[test]
    fn profile_merge_replaces_unconfirmed_manual_chapter_placeholder() {
        let mut existing = profile(PROFILE_OWNERSHIP_USER);
        existing.first_seen_chapter = 1;
        existing.last_seen_chapter = 1;
        existing.evidence_json = json!([]);
        let mut incoming = discovery_input();
        incoming.first_seen_chapter = 5;
        incoming.last_seen_chapter = 5;

        let merged = merge_object_profile_record_input(&existing, incoming);

        assert_eq!(merged.first_seen_chapter, 5);
        assert_eq!(merged.last_seen_chapter, 5);
    }

    #[test]
    fn profile_merge_replaces_ai_range_after_source_evidence_was_removed() {
        let mut existing = profile(PROFILE_OWNERSHIP_AI_SYNC);
        existing.first_seen_chapter = 0;
        existing.last_seen_chapter = 0;
        existing.evidence_json = json!([]);
        let mut incoming = discovery_input();
        incoming.first_seen_chapter = 5;
        incoming.last_seen_chapter = 5;

        let merged = merge_object_profile_record_input(&existing, incoming);

        assert_eq!(merged.first_seen_chapter, 5);
        assert_eq!(merged.last_seen_chapter, 5);
    }

    #[tokio::test]
    async fn inserts_new_object_profiles_in_one_batch() {
        let db = a4n_migration::test_support::connect_test_database().await;
        let user = crate::auth::create_user(
            &db,
            "object_profile_batch_insert",
            "object-profile-batch-insert@example.invalid",
            "object-profile-batch-insert-password",
            ROLE_USER,
        )
        .await
        .unwrap();
        let project = crate::project::create_project_record(
            &db,
            crate::project::CreateProjectRecordInput {
                user_id: user.id,
                title: "物体档案批量写入".to_string(),
                description: String::new(),
                project_type: "novel_long".to_string(),
            },
        )
        .await
        .unwrap();
        let inputs = (0..20)
            .map(|index| {
                let mut input = discovery_input();
                input.name = format!("物体 {index}");
                input.normalized_name = format!("物体 {index}");
                input
            })
            .collect::<Vec<_>>();

        let result = upsert_object_profiles_in(&db, project.id, inputs)
            .await
            .unwrap();

        assert_eq!(result.created.len(), 20);
        assert!(result.updated.is_empty());
        assert_eq!(
            object_profile::Entity::find()
                .filter(object_profile::Column::ProjectId.eq(project.id))
                .all(&db)
                .await
                .unwrap()
                .len(),
            20
        );
    }

    #[tokio::test]
    async fn updates_existing_object_profiles_in_one_batch() {
        let db = a4n_migration::test_support::connect_test_database().await;
        let user = crate::auth::create_user(
            &db,
            "object_profile_batch_update",
            "object-profile-batch-update@example.invalid",
            "object-profile-batch-update-password",
            ROLE_USER,
        )
        .await
        .unwrap();
        let project = crate::project::create_project_record(
            &db,
            crate::project::CreateProjectRecordInput {
                user_id: user.id,
                title: "物体档案批量更新".to_string(),
                description: String::new(),
                project_type: "novel_long".to_string(),
            },
        )
        .await
        .unwrap();
        let initial_inputs = (0..20)
            .map(|index| {
                let mut input = discovery_input();
                input.name = format!("物体 {index}");
                input.normalized_name = format!("物体 {index}");
                input
            })
            .collect::<Vec<_>>();
        upsert_object_profiles_in(&db, project.id, initial_inputs)
            .await
            .unwrap();

        let refreshed_inputs = (0..20)
            .map(|index| {
                let mut input = discovery_input();
                input.name = format!("刷新物体 {index}");
                input.normalized_name = format!("物体 {index}");
                input.visual_description = format!("刷新外观 {index}");
                input
            })
            .collect::<Vec<_>>();
        let result = upsert_object_profiles_in(&db, project.id, refreshed_inputs)
            .await
            .unwrap();

        assert!(result.created.is_empty());
        assert_eq!(result.updated.len(), 20);
        assert_eq!(result.updated[0].name, "刷新物体 0");
        assert_eq!(result.updated[0].visual_description, "刷新外观 0");
        assert_eq!(
            object_profile::Entity::find()
                .filter(object_profile::Column::ProjectId.eq(project.id))
                .all(&db)
                .await
                .unwrap()
                .len(),
            20
        );
    }
}
