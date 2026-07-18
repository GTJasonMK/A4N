use serde_json::{Value, json};

pub(super) fn minimal_blueprint_refine_context(blueprint_context: &Value) -> Value {
    let Some(object) = blueprint_context.as_object() else {
        return blueprint_context.clone();
    };
    let mut minimal = serde_json::Map::new();
    if let Some(blueprint) = object.get("blueprint") {
        minimal.insert(
            "blueprint".to_string(),
            minimal_blueprint_model_context(blueprint),
        );
    }
    if let Some(characters) = object.get("characters") {
        minimal.insert(
            "character_count".to_string(),
            json!(characters.as_array().map_or(0, Vec::len)),
        );
    }
    if let Some(relationships) = object.get("relationships") {
        minimal.insert(
            "relationship_count".to_string(),
            json!(relationships.as_array().map_or(0, Vec::len)),
        );
    }
    Value::Object(minimal)
}

fn minimal_blueprint_model_context(blueprint: &Value) -> Value {
    let Some(object) = blueprint.as_object() else {
        return blueprint.clone();
    };
    let mut minimal = serde_json::Map::new();
    for key in [
        "title",
        "genre",
        "style",
        "tone",
        "target_audience",
        "one_sentence_summary",
        "full_synopsis",
        "total_chapters",
        "chapters_per_part",
    ] {
        if let Some(value) = object.get(key) {
            minimal.insert(key.to_string(), value.clone());
        }
    }
    Value::Object(minimal)
}
