use serde_json::json;

use super::RagDataType;

#[test]
fn canonical_names_parse_and_round_trip_through_adapters() {
    for data_type in [
        RagDataType::Chapter,
        RagDataType::ChapterSummary,
        RagDataType::ChapterMetadata,
        RagDataType::KeyEvent,
        RagDataType::Foreshadowing,
        RagDataType::Outline,
        RagDataType::PartOutline,
        RagDataType::Blueprint,
        RagDataType::BlueprintMetadata,
        RagDataType::WorldSetting,
        RagDataType::Character,
        RagDataType::Protagonist,
        RagDataType::CharacterState,
        RagDataType::CharacterAttributeEvent,
        RagDataType::Relationship,
        RagDataType::SceneProfile,
        RagDataType::SceneOccurrence,
        RagDataType::SceneTransition,
    ] {
        assert_eq!(RagDataType::parse(data_type.as_str()), Some(data_type));
        assert_eq!(data_type.to_string(), data_type.as_str());
        assert_eq!(data_type.as_ref(), data_type.as_str());
        assert_eq!(RagDataType::try_from(data_type.as_str()), Ok(data_type));
        assert_eq!(
            serde_json::to_value(data_type).expect("RAG data type wire value"),
            json!(data_type.as_str())
        );
    }
    assert_eq!(RagDataType::parse(" CHAPTER "), Some(RagDataType::Chapter));
}

#[test]
fn catalogs_preserve_chapter_derived_and_ingestable_order() {
    assert_eq!(
        RagDataType::CHAPTER_DERIVED.map(RagDataType::as_str),
        [
            "chapter",
            "chapter_summary",
            "chapter_metadata",
            "key_event",
            "foreshadowing",
        ]
    );
    assert_eq!(
        RagDataType::INGESTABLE.map(RagDataType::as_str),
        [
            "chapter",
            "foreshadowing",
            "outline",
            "part_outline",
            "blueprint",
            "blueprint_metadata",
            "world_setting",
            "character",
            "protagonist",
            "character_state",
            "character_attribute_event",
            "relationship",
            "scene_profile",
            "scene_occurrence",
            "scene_transition",
        ]
    );
    assert_eq!(
        RagDataType::ingest_hint(),
        RagDataType::INGESTABLE.map(RagDataType::as_str).join(", ")
    );
}

#[test]
fn retrieval_weight_prioritizes_core_story_constraints() {
    assert_eq!(RagDataType::Blueprint.retrieval_weight(), 1.0);
    assert!(RagDataType::WorldSetting.retrieval_weight() > RagDataType::Chapter.retrieval_weight());
    assert!(
        RagDataType::Character.retrieval_weight() > RagDataType::Protagonist.retrieval_weight()
    );
    assert!(
        RagDataType::KeyEvent.retrieval_weight() > RagDataType::Foreshadowing.retrieval_weight()
    );
    assert!(
        RagDataType::CharacterState.retrieval_weight()
            > RagDataType::CharacterAttributeEvent.retrieval_weight()
    );
    assert!(
        RagDataType::SceneProfile.retrieval_weight()
            > RagDataType::CharacterAttributeEvent.retrieval_weight()
    );
}

#[test]
fn aliases_and_unknown_wire_values_are_rejected() {
    for alias in [
        "chapters",
        "chapter_content",
        "summary",
        "metadata",
        "events",
        "chapter_outline",
        "part_outlines",
        "blueprint_meta",
        "world",
        "protagonist_profile",
        "character_states",
        "protagonist_change",
        "attribute_event",
        "relationships",
        "scene",
        "scenes",
        "scene_profiles",
        "locations",
    ] {
        assert_eq!(
            RagDataType::parse(alias),
            None,
            "alias should be rejected: {alias}"
        );
        assert_eq!(RagDataType::try_from(alias), Err(()));
    }
    assert!(serde_json::from_value::<RagDataType>(json!("chapter_content")).is_err());
}
