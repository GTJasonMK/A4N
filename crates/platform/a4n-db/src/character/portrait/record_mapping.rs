use a4n_db_models::models::character::character_portrait;
use a4n_db_models::models::character::protagonist_profile;
use a4n_domain::character::{CharacterPortraitRecord, CharacterProfileRecord};

pub(crate) fn character_profile_record_from_model(
    model: protagonist_profile::Model,
) -> CharacterProfileRecord {
    CharacterProfileRecord {
        id: model.id,
        project_id: model.project_id,
        name: model.name,
        gender: model.gender,
        age: model.age,
        base_attributes: model.base_attributes,
        current_state: model.current_state,
        character_arc: model.character_arc,
        voice_fingerprint: model.voice_fingerprint,
        appearance: model.appearance,
        background: model.background,
        personality: model.personality,
        last_synced_chapter: model.last_synced_chapter,
        created_at: model.created_at,
        updated_at: model.updated_at,
    }
}

pub(crate) fn character_portrait_record_from_model(
    model: character_portrait::Model,
) -> CharacterPortraitRecord {
    CharacterPortraitRecord {
        id: model.id,
        profile_id: model.profile_id,
        project_id: model.project_id,
        portrait_type: model.portrait_type,
        image_path: model.image_path,
        prompt: model.prompt,
        expression: model.expression,
        pose: model.pose,
        is_primary: model.is_primary,
        created_at: model.created_at,
    }
}
