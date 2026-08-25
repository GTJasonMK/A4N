use a4n_core::blueprint::ParsedBlueprintCharacter;
use a4n_db::project::BlueprintCharacterDraft;

pub(in crate::blueprint) fn blueprint_character_drafts(
    characters: &[ParsedBlueprintCharacter],
) -> Vec<BlueprintCharacterDraft> {
    characters
        .iter()
        .map(|character| BlueprintCharacterDraft {
            name: character.name.clone(),
            character_type: character.character_type.clone(),
            gender: character.gender.clone(),
            age: character.age.clone(),
            appearance: Some(character.appearance.clone()),
            personality: Some(character.personality.clone()),
            background: Some(character.background.clone()),
            motivation: Some(character.motivation.clone()),
            arc_summary: Some(character.arc_summary.clone()),
            relationship_to_protagonist: Some(character.relationship_to_protagonist.clone()),
            arc_stages: character.arc_stages.clone(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn character_draft_preserves_all_blueprint_fields() {
        let arc_stages = json!([{"stage": "觉醒"}]);
        let drafts = blueprint_character_drafts(&[ParsedBlueprintCharacter {
            name: "林舟".to_string(),
            character_type: "protagonist".to_string(),
            gender: Some("男".to_string()),
            age: Some("24".to_string()),
            appearance: "黑发".to_string(),
            personality: "谨慎".to_string(),
            background: "边城出身".to_string(),
            motivation: "寻找真相".to_string(),
            arc_summary: "从逃避到承担".to_string(),
            relationship_to_protagonist: "本人".to_string(),
            arc_stages: Some(arc_stages.clone()),
        }]);

        let draft = &drafts[0];
        assert_eq!(draft.name, "林舟");
        assert_eq!(draft.character_type, "protagonist");
        assert_eq!(draft.gender.as_deref(), Some("男"));
        assert_eq!(draft.age.as_deref(), Some("24"));
        assert_eq!(draft.appearance.as_deref(), Some("黑发"));
        assert_eq!(draft.personality.as_deref(), Some("谨慎"));
        assert_eq!(draft.background.as_deref(), Some("边城出身"));
        assert_eq!(draft.motivation.as_deref(), Some("寻找真相"));
        assert_eq!(draft.arc_summary.as_deref(), Some("从逃避到承担"));
        assert_eq!(draft.relationship_to_protagonist.as_deref(), Some("本人"));
        assert_eq!(draft.arc_stages, Some(arc_stages));
    }
}
