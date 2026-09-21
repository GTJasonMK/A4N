use a4n_db::project::BlueprintCharacterDraft;
use serde_json::Value;

use crate::import_reverse::application::reverse_json::{
    optional_field_clone, required_array, required_string,
};

pub(in crate::import_reverse::application::apply::blueprint) fn reverse_blueprint_characters(
    data: &Value,
) -> Result<Vec<BlueprintCharacterDraft>, String> {
    let characters = required_array(data, "characters", "reverse_blueprint")?;

    let mut drafts = Vec::with_capacity(characters.len());
    for (index, item) in characters.iter().enumerate() {
        let context = format!("reverse_blueprint.characters[{index}]");
        let name = required_string(item, "name", &context)?;
        let character_type = required_string(item, "character_type", &context)?;
        let gender = required_string(item, "gender", &context)?;
        let age = required_string(item, "age", &context)?;
        let appearance = required_string(item, "appearance", &context)?;
        let personality = required_string(item, "personality", &context)?;
        let background = required_string(item, "background", &context)?;
        let motivation = required_string(item, "motivation", &context)?;
        let arc_summary = required_string(item, "arc_summary", &context)?;
        let relationship_to_protagonist =
            required_string(item, "relationship_to_protagonist", &context)?;
        drafts.push(BlueprintCharacterDraft {
            name,
            character_type,
            gender: Some(gender),
            age: Some(age),
            appearance: Some(appearance),
            personality: Some(personality),
            background: Some(background),
            motivation: Some(motivation),
            arc_summary: Some(arc_summary),
            relationship_to_protagonist: Some(relationship_to_protagonist),
            arc_stages: optional_field_clone(item, "arc_stages"),
        });
    }

    Ok(drafts)
}
