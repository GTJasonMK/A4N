use serde_json::{Value, json};

pub const BLUEPRINT_STORY_CORE_SHAPE: &str = r#"{"title":"string","target_audience":"string","genre":"string","style":"string","tone":"string","one_sentence_summary":"string","full_synopsis":"string"}"#;
pub const BLUEPRINT_WORLD_SHAPE: &str = r#"{"world_setting":{"premise":"string","core_rules":"string","technology_or_magic":{"source":"string","rules":["string"],"limits":["string"],"costs":["string"]},"key_locations":[{"name":"string","description":"string","story_function":"string"}],"factions":[{"name":"string","description":"string","agenda":"string","representative_characters":["string"]}],"history_timeline":[{"period":"string","event":"string","impact":"string"}],"culture_and_society":{"values":"string","class_structure":"string","daily_life":"string"},"conflict_sources":[{"name":"string","description":"string","stakes":"string"}],"mysteries_and_foreshadowing":[{"mystery":"string","early_clues":["string"],"payoff_direction":"string"}],"genre_promises":["string"]}}"#;
pub const BLUEPRINT_CHARACTERS_SHAPE: &str = r#"{"characters":[{"name":"string","character_type":"protagonist|antagonist|supporting|minor","gender":"string","age":"string","appearance":"string","personality":"string","background":"string","motivation":"string","arc_summary":"string","relationship_to_protagonist":"string","arc_stages":{"start":"string","core_wound":"string","desire":"string","need":"string","secret":"string","voice_style":"string","turning_point":"string","end":"string"}}]}"#;
pub const BLUEPRINT_RELATIONSHIPS_SHAPE: &str = r#"{"relationships":[{"relationship_type":"family|friend|enemy|lover|rival|mentor|other","character_a":"string","character_b":"string","description":"string","strength":1,"evolution":{"start":"string","tension":"string","turning_points":["string"],"hidden_truth":"string","story_function":"string","end":"string"}}]}"#;

pub fn blueprint_output_schema() -> Value {
    let mut properties = serde_json::Map::new();
    for shape in [
        BLUEPRINT_STORY_CORE_SHAPE,
        BLUEPRINT_WORLD_SHAPE,
        BLUEPRINT_CHARACTERS_SHAPE,
        BLUEPRINT_RELATIONSHIPS_SHAPE,
    ] {
        let shape = serde_json::from_str(shape).expect("static blueprint shape");
        let schema = crate::json_contract::json_schema_from_output_shape(&shape)
            .expect("static blueprint schema");
        properties.extend(
            schema["properties"]
                .as_object()
                .expect("object schema")
                .clone(),
        );
    }
    json!({"type":"object","required":properties.keys().collect::<Vec<_>>(),"properties":properties,"additionalProperties":false})
}

pub fn blueprint_refinement_output_schema() -> Value {
    let mut schema = blueprint_output_schema();
    schema["properties"]
        .as_object_mut()
        .expect("object schema")
        .retain(|key, _| super::BLUEPRINT_REFINEMENT_FIELDS.contains(&key.as_str()));
    schema["required"] = json!([]);
    schema["minProperties"] = json!(1);
    for field in schema["properties"]
        .as_object_mut()
        .expect("object schema")
        .values_mut()
    {
        *field = json!({"anyOf":[field.clone(),{"type":"null"}]});
    }
    schema
}
