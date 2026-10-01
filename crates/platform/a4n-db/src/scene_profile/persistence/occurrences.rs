use super::refs::resolve_ref;
use crate::scene_profile::{SceneProfileDbError, SceneProfileDbResult};
use a4n_db_models::models::scene::scene_occurrence;
use a4n_domain::scene_profile::SceneOccurrenceRecordInput;
use chrono::Utc;
use sea_orm::{ConnectionTrait, EntityTrait, Set};
use std::collections::{BTreeMap, BTreeSet};

pub(in crate::scene_profile) async fn insert_scene_occurrence_records<C>(
    db: &C,
    project_id: i64,
    chapter_id: i64,
    chapter_number: i32,
    drafts: &[SceneOccurrenceRecordInput],
    scene_ref_map: &BTreeMap<String, i64>,
) -> SceneProfileDbResult<BTreeMap<String, i64>>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    let mut refs = BTreeSet::new();
    let mut rows = Vec::with_capacity(drafts.len());
    let mut occurrence_refs = Vec::with_capacity(drafts.len());
    for draft in drafts {
        if !refs.insert(draft.occurrence_ref.clone()) {
            return Err(SceneProfileDbError::InvalidArgument(format!(
                "场景同步存在重复 occurrence_ref: {}",
                draft.occurrence_ref
            )));
        }
        let scene_profile_id =
            resolve_ref(scene_ref_map, &draft.scene_ref, "occurrence.scene_ref")?;
        rows.push(scene_occurrence::ActiveModel {
            project_id: Set(project_id),
            scene_profile_id: Set(scene_profile_id),
            chapter_id: Set(chapter_id),
            chapter_number: Set(chapter_number),
            occurrence_order: Set(draft.occurrence_order),
            timeline_position: Set(draft.timeline_position.clone()),
            summary: Set(draft.summary.clone()),
            mood: Set(draft.mood.clone()),
            weather_or_lighting: Set(draft.weather_or_lighting.clone()),
            source_span_ids_json: Set(draft.source_span_ids_json.clone()),
            evidence_json: Set(draft.evidence_json.clone()),
            characters_json: Set(draft.characters_json.clone()),
            objects_json: Set(draft.objects_json.clone()),
            key_events_json: Set(draft.key_events_json.clone()),
            entry_state_json: Set(draft.entry_state_json.clone()),
            exit_state_json: Set(draft.exit_state_json.clone()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        });
        occurrence_refs.push(draft.occurrence_ref.clone());
    }
    if rows.is_empty() {
        return Ok(BTreeMap::new());
    }
    let inserted = scene_occurrence::Entity::insert_many(rows)
        .exec_with_returning_many(db)
        .await?;
    if inserted.len() != occurrence_refs.len() {
        return Err(SceneProfileDbError::InvalidArgument(
            "场景出场批量写入返回行数不一致".to_string(),
        ));
    }
    Ok(occurrence_refs
        .into_iter()
        .zip(inserted)
        .map(|(occurrence_ref, occurrence)| (occurrence_ref, occurrence.id))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::insert_scene_occurrence_records;
    use crate::scene_profile::create_scene_profile_model;
    use a4n_db_models::models::{
        chapter::chapter,
        scene::{scene_entity_state, scene_occurrence, scene_profile, scene_transition},
    };
    use a4n_domain::profile_ownership::PROFILE_OWNERSHIP_AI_SYNC;
    use a4n_domain::scene_profile::{
        SceneEntityStateRecordInput, SceneOccurrenceRecordInput, SceneProfileRecordInput,
        SceneTransitionRecordInput,
    };
    use chrono::Utc;
    use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter, QueryOrder, Set};
    use serde_json::json;
    use std::collections::{BTreeMap, BTreeSet};

    #[tokio::test]
    async fn inserts_occurrences_in_batch_and_returns_each_reference_id() {
        let db = a4n_migration::test_support::connect_test_database().await;
        let project = crate::project::create_project_record(
            &db,
            crate::project::CreateProjectRecordInput {
                user_id: 1,
                title: "场景出场批量写入".to_string(),
                description: String::new(),
                project_type: "novel_long".to_string(),
            },
        )
        .await
        .unwrap();
        let now = Utc::now().naive_utc();
        let chapter = chapter::ActiveModel {
            project_id: Set(project.id),
            chapter_number: Set(1),
            title: Set(Some("第一章".to_string())),
            content: Set("正文".to_string()),
            word_count: Set(2),
            status: Set(chapter::STATUS_COMPLETED.to_string()),
            summary: Set(None),
            style_tags: Set(None),
            analysis_data: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .unwrap();
        let profile_input = |name: &str| SceneProfileRecordInput {
            name: name.to_string(),
            normalized_name: name.to_string(),
            aliases_json: json!([]),
            scene_type: "location".to_string(),
            importance: "major".to_string(),
            parent_scene_id: None,
            base_description: String::new(),
            visual_description: String::new(),
            spatial_layout: String::new(),
            atmosphere: String::new(),
            entry_points_json: json!([]),
            rules_json: json!([]),
            first_seen_chapter: 1,
            last_seen_chapter: 1,
            evidence_json: json!([]),
            image_prompt_seed: String::new(),
        };
        let first_profile = create_scene_profile_model(
            &db,
            project.id,
            profile_input("大厅"),
            PROFILE_OWNERSHIP_AI_SYNC,
        )
        .await
        .unwrap();
        let second_profile = create_scene_profile_model(
            &db,
            project.id,
            profile_input("走廊"),
            PROFILE_OWNERSHIP_AI_SYNC,
        )
        .await
        .unwrap();
        let scene_ref_map = BTreeMap::from([
            ("hall".to_string(), first_profile.id),
            ("corridor".to_string(), second_profile.id),
        ]);
        let drafts = (0..20)
            .map(|index| SceneOccurrenceRecordInput {
                occurrence_ref: format!("occ-{index}"),
                scene_ref: if index % 2 == 0 {
                    "hall".to_string()
                } else {
                    "corridor".to_string()
                },
                occurrence_order: index + 1,
                timeline_position: None,
                summary: format!("出场 {index}"),
                mood: None,
                weather_or_lighting: None,
                source_span_ids_json: json!([]),
                evidence_json: json!([]),
                characters_json: json!([]),
                objects_json: json!([]),
                key_events_json: json!([]),
                entry_state_json: json!({}),
                exit_state_json: json!({}),
            })
            .collect::<Vec<_>>();
        let refs = insert_scene_occurrence_records(
            &db,
            project.id,
            chapter.id,
            1,
            &drafts,
            &scene_ref_map,
        )
        .await
        .unwrap();

        let rows = scene_occurrence::Entity::find()
            .filter(scene_occurrence::Column::ProjectId.eq(project.id))
            .filter(scene_occurrence::Column::ChapterId.eq(chapter.id))
            .order_by_asc(scene_occurrence::Column::OccurrenceOrder)
            .all(&db)
            .await
            .unwrap();
        assert_eq!(rows.len(), drafts.len());
        assert_eq!(refs.len(), drafts.len());
        for row in rows {
            assert_eq!(
                refs.get(&format!("occ-{}", row.occurrence_order - 1)),
                Some(&row.id)
            );
        }

        let transitions = (0..5)
            .map(|index| SceneTransitionRecordInput {
                from_scene_ref: (index > 0).then(|| "hall".to_string()),
                to_scene_ref: "corridor".to_string(),
                occurrence_ref: Some(format!("occ-{index}")),
                transition_order: index + 1,
                movement_type: "walk".to_string(),
                route_description: format!("路线 {index}"),
                time_cost: None,
                distance_hint: None,
                participants_json: json!([]),
                object_refs_json: json!([]),
                source_span_ids_json: json!([]),
                evidence_json: json!([]),
            })
            .collect::<Vec<_>>();
        assert_eq!(
            super::super::relations::insert_scene_transition_records(
                &db,
                project.id,
                chapter.id,
                1,
                &transitions,
                &scene_ref_map,
                &refs,
            )
            .await
            .unwrap(),
            transitions.len()
        );
        let entity_states = (0..5)
            .map(|index| SceneEntityStateRecordInput {
                scene_ref: "hall".to_string(),
                occurrence_ref: format!("occ-{index}"),
                entity_type: "object".to_string(),
                entity_id: None,
                entity_name_snapshot: format!("物体 {index}"),
                state_key: "location".to_string(),
                state_before_json: json!({"value": "old"}),
                state_after_json: json!({"value": "new"}),
                position_description: None,
                action_description: "移动".to_string(),
                source_span_ids_json: json!([]),
                evidence_json: json!([]),
            })
            .collect::<Vec<_>>();
        assert_eq!(
            super::super::relations::insert_scene_entity_state_records(
                &db,
                project.id,
                &entity_states,
                &scene_ref_map,
                &refs,
            )
            .await
            .unwrap(),
            entity_states.len()
        );
        assert_eq!(
            scene_transition::Entity::find()
                .filter(scene_transition::Column::ProjectId.eq(project.id))
                .all(&db)
                .await
                .unwrap()
                .len(),
            transitions.len()
        );
        assert_eq!(
            scene_entity_state::Entity::find()
                .filter(scene_entity_state::Column::ProjectId.eq(project.id))
                .all(&db)
                .await
                .unwrap()
                .len(),
            entity_states.len()
        );

        super::super::refresh::refresh_scene_profiles_after_occurrence_change(
            &db,
            project.id,
            &BTreeSet::from([first_profile.id, second_profile.id]),
            false,
        )
        .await
        .unwrap();
        let refreshed_profiles = scene_profile::Entity::find()
            .filter(scene_profile::Column::ProjectId.eq(project.id))
            .all(&db)
            .await
            .unwrap();
        assert_eq!(refreshed_profiles.len(), 2);
        for profile in refreshed_profiles {
            assert_eq!(profile.first_seen_chapter, 1);
            assert_eq!(profile.last_seen_chapter, 1);
            assert_eq!(profile.current_state_json["chapter_number"], 1);
        }
    }
}
