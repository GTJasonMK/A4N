use a4n_db::project::BlueprintRelationshipDraft;
use serde_json::Value;

use crate::import_reverse::application::reverse_json::{
    optional_field_clone, optional_i32, required_array, required_string,
};

pub(in crate::import_reverse::application::apply::blueprint) fn reverse_blueprint_relationships(
    data: &Value,
) -> Result<Vec<BlueprintRelationshipDraft>, String> {
    let relationships = required_array(data, "relationships", "reverse_blueprint")?;

    let mut drafts = Vec::with_capacity(relationships.len());
    for (index, item) in relationships.iter().enumerate() {
        let context = format!("reverse_blueprint.relationships[{index}]");
        let character_a = required_string(item, "character_a", &context)?;
        let character_b = required_string(item, "character_b", &context)?;
        let relationship_type = required_string(item, "relationship_type", &context)?;
        let description = required_string(item, "description", &context)?;
        drafts.push(BlueprintRelationshipDraft {
            relationship_type,
            character_a,
            character_b,
            description: Some(description),
            strength: optional_i32(item, "strength", &context)?,
            evolution: optional_field_clone(item, "evolution"),
        });
    }

    Ok(drafts)
}
