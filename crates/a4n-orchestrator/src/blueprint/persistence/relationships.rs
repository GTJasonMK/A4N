use a4n_db::project::BlueprintRelationshipDraft;
use a4n_domain::project::ParsedBlueprintRelationship;

pub(in crate::blueprint) fn blueprint_relationship_drafts(
    relationships: &[ParsedBlueprintRelationship],
) -> Vec<BlueprintRelationshipDraft> {
    relationships
        .iter()
        .map(|relationship| BlueprintRelationshipDraft {
            relationship_type: relationship.relationship_type.clone(),
            character_a: relationship.character_a.clone(),
            character_b: relationship.character_b.clone(),
            description: Some(relationship.description.clone()),
            strength: relationship.strength,
            evolution: relationship.evolution.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn relationship_draft_preserves_all_blueprint_fields() {
        let evolution = json!([{"phase": "对立"}, {"phase": "合作"}]);
        let drafts = blueprint_relationship_drafts(&[ParsedBlueprintRelationship {
            relationship_type: "rival".to_string(),
            character_a: "林舟".to_string(),
            character_b: "沈雾".to_string(),
            description: "由竞争转为共谋".to_string(),
            strength: Some(8),
            evolution: Some(evolution.clone()),
        }]);

        let draft = &drafts[0];
        assert_eq!(draft.relationship_type, "rival");
        assert_eq!(draft.character_a, "林舟");
        assert_eq!(draft.character_b, "沈雾");
        assert_eq!(draft.description.as_deref(), Some("由竞争转为共谋"));
        assert_eq!(draft.strength, Some(8));
        assert_eq!(draft.evolution, Some(evolution));
    }
}
