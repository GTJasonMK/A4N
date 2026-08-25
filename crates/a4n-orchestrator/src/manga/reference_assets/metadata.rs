use serde_json::{Value, json};

use a4n_db::project::MangaCharacterPortraitRefSummary;

fn character_portraits_json(refs: &[MangaCharacterPortraitRefSummary]) -> Value {
    Value::Array(refs.iter().map(character_portrait_json).collect())
}

fn character_portrait_json(portrait: &MangaCharacterPortraitRefSummary) -> Value {
    json!({
        "profile_id": portrait.profile_id,
        "portrait_id": portrait.portrait_id,
        "character_name": portrait.character_name,
        "image_path": portrait.image_path,
        "prompt": portrait.prompt,
        "portrait_type": portrait.portrait_type,
        "expression": portrait.expression,
        "pose": portrait.pose,
        "is_primary": true,
    })
}

pub(in crate::manga) fn manga_reference_assets_json(
    refs: &[MangaCharacterPortraitRefSummary],
) -> Value {
    json!({
        "character_portraits": character_portraits_json(refs),
    })
}
