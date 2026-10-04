use super::contracts::{ChapterGenerationReferenceContextRows, ProjectChapterContextError};
use a4n_db_models::models::{
    character::protagonist_profile,
    project::{blueprint_character, blueprint_relationship, novel_blueprint},
};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};

pub async fn chapter_generation_reference_context_rows(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_number: i32,
) -> Result<ChapterGenerationReferenceContextRows, ProjectChapterContextError> {
    if chapter_number <= 0 {
        return Err(ProjectChapterContextError::InvalidArgument(
            "目标章节号必须大于 0".into(),
        ));
    }
    let (blueprint, characters, relationships, protagonist_profiles) = futures::join!(
        async {
            novel_blueprint::Entity::find()
                .filter(novel_blueprint::Column::ProjectId.eq(project_id))
                .one(db)
                .await
                .map_err(ProjectChapterContextError::Database)
        },
        async {
            blueprint_character::Entity::find()
                .filter(blueprint_character::Column::ProjectId.eq(project_id))
                .order_by_asc(blueprint_character::Column::Id)
                .all(db)
                .await
                .map_err(ProjectChapterContextError::Database)
        },
        async {
            blueprint_relationship::Entity::find()
                .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
                .order_by_desc(blueprint_relationship::Column::Strength)
                .order_by_asc(blueprint_relationship::Column::Id)
                .all(db)
                .await
                .map_err(ProjectChapterContextError::Database)
        },
        async {
            protagonist_profile::Entity::find()
                .filter(protagonist_profile::Column::ProjectId.eq(project_id))
                .order_by_asc(protagonist_profile::Column::Id)
                .all(db)
                .await
                .map_err(ProjectChapterContextError::Database)
        },
    );
    let mut blueprint = blueprint?;
    if let Some(current) = blueprint.as_mut() {
        current.world_setting = Some(
            crate::fact_feedback::world_setting_before_chapter(
                db,
                &crate::project::blueprint_record(current.clone()),
                chapter_number,
            )
            .await
            .map_err(|error| ProjectChapterContextError::InvalidArgument(error.to_string()))?,
        );
    }
    let characters = characters?;
    let relationships = relationships?;
    let mut historical_profiles = Vec::new();
    for mut profile in protagonist_profiles? {
        if profile
            .first_seen_chapter()
            .is_some_and(|first| first >= chapter_number)
        {
            continue;
        }
        if profile.is_text_discovered()
            && crate::text_anchor::earliest_current_profile_evidence_chapter(
                db,
                project_id,
                a4n_domain::text_anchor::TextAnchorProfileKind::Character,
                profile.id,
            )
            .await
            .map_err(|error| ProjectChapterContextError::InvalidArgument(error.to_string()))?
            .is_none_or(|first| first >= chapter_number)
        {
            continue;
        }
        let state = crate::character::replay_profile_state_through_chapter(
            db,
            &profile,
            chapter_number - 1,
        )
        .await
        .map_err(|error| match error {
            crate::character::CharacterDbError::Database(error) => {
                ProjectChapterContextError::Database(error)
            }
            error => ProjectChapterContextError::InvalidArgument(error.to_string()),
        })?;
        let fields = crate::character::profile_current_state_fields(state);
        profile.gender = fields.gender;
        profile.age = fields.age;
        profile.appearance = fields.appearance;
        profile.background = fields.background;
        profile.personality = fields.personality;
        profile.current_state = fields.current_state;
        crate::fact_feedback::profile_enrichment_before_chapter(db, &mut profile, chapter_number)
            .await
            .map_err(|error| ProjectChapterContextError::InvalidArgument(error.to_string()))?;
        profile.last_synced_chapter = profile.last_synced_chapter.min(chapter_number - 1);
        historical_profiles.push(profile);
    }

    Ok(ChapterGenerationReferenceContextRows {
        blueprint: blueprint.map(crate::project::blueprint_record),
        characters: characters
            .into_iter()
            .map(crate::project::blueprint_character_record)
            .collect(),
        relationships: relationships
            .into_iter()
            .map(crate::project::blueprint_relationship_record)
            .collect(),
        protagonist_profiles: historical_profiles
            .into_iter()
            .map(crate::character::character_profile_record)
            .collect(),
    })
}
