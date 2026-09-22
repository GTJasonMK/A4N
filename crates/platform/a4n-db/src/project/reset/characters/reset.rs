use super::{
    cleanup::{delete_retained_profile_related_rows, delete_text_discovered_profiles},
    contracts::{CharacterProfileResetScope, ResetCharacterProfilesResult},
    queries::load_character_profile_reset_scope,
};
use crate::{character, project::reset::ProjectResetDbError};
use a4n_db_models::models::character::protagonist_profile;
use chrono::NaiveDateTime;
use sea_orm::{
    ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter,
    sea_query::{CaseStatement, Expr},
};

pub(in crate::project::reset) async fn reset_character_profiles_for_project<C>(
    tx: &C,
    project_id: i64,
    now: NaiveDateTime,
) -> Result<ResetCharacterProfilesResult, ProjectResetDbError>
where
    C: ConnectionTrait,
{
    let CharacterProfileResetScope {
        text_profile_ids,
        retained_profiles,
        retained_profile_ids,
        related_counts,
    } = load_character_profile_reset_scope(tx, project_id).await?;

    delete_retained_profile_related_rows(tx, &retained_profile_ids).await?;
    delete_text_discovered_profiles(tx, project_id, &text_profile_ids).await?;

    if !retained_profiles.is_empty() {
        let mut gender = CaseStatement::new();
        let mut age = CaseStatement::new();
        let mut appearance = CaseStatement::new();
        let mut background = CaseStatement::new();
        let mut personality = CaseStatement::new();
        let mut current_state = CaseStatement::new();
        for profile in &retained_profiles {
            let fields = character::profile_current_state_fields(profile.base_attributes.clone());
            gender = gender.case(
                protagonist_profile::Column::Id.eq(profile.id),
                fields.gender,
            );
            age = age.case(protagonist_profile::Column::Id.eq(profile.id), fields.age);
            appearance = appearance.case(
                protagonist_profile::Column::Id.eq(profile.id),
                fields.appearance,
            );
            background = background.case(
                protagonist_profile::Column::Id.eq(profile.id),
                fields.background,
            );
            personality = personality.case(
                protagonist_profile::Column::Id.eq(profile.id),
                fields.personality,
            );
            current_state = current_state.case(
                protagonist_profile::Column::Id.eq(profile.id),
                fields.current_state,
            );
        }
        protagonist_profile::Entity::update_many()
            .col_expr(protagonist_profile::Column::Gender, gender.into())
            .col_expr(protagonist_profile::Column::Age, age.into())
            .col_expr(protagonist_profile::Column::Appearance, appearance.into())
            .col_expr(protagonist_profile::Column::Background, background.into())
            .col_expr(protagonist_profile::Column::Personality, personality.into())
            .col_expr(
                protagonist_profile::Column::CurrentState,
                current_state.into(),
            )
            .col_expr(
                protagonist_profile::Column::LastSyncedChapter,
                Expr::value(0),
            )
            .col_expr(protagonist_profile::Column::UpdatedAt, Expr::value(now))
            .filter(protagonist_profile::Column::Id.is_in(retained_profile_ids.iter().copied()))
            .exec(tx)
            .await?;
    }

    Ok(ResetCharacterProfilesResult {
        reset_character_profiles: retained_profile_ids.len() as u64,
        deleted_character_profiles: text_profile_ids.len() as u64,
        deleted_character_snapshots: related_counts.character_snapshots,
        deleted_character_timeline_rows: related_counts.timeline_rows(),
        deleted_character_portraits: related_counts.character_portraits,
    })
}
