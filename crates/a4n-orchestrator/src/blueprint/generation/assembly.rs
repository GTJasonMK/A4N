use a4n_db::project::{CHAPTERS_PER_PART, needs_part_outlines_for_chapters};
use serde_json::{Map, Value};

pub(super) fn assemble_staged_blueprint_json(
    novel_title: &str,
    genre: &str,
    target_length: i32,
    core_output: &Value,
    world_output: &Value,
    characters_output: &Value,
    relationships_output: &Value,
) -> Value {
    let mut blueprint = Map::new();
    for key in [
        "title",
        "target_audience",
        "genre",
        "style",
        "tone",
        "one_sentence_summary",
        "full_synopsis",
    ] {
        if let Some(value) = core_output.get(key) {
            blueprint.insert(key.to_string(), value.clone());
        }
    }
    blueprint
        .entry("title".to_string())
        .or_insert_with(|| Value::String(novel_title.to_string()));
    blueprint
        .entry("genre".to_string())
        .or_insert_with(|| Value::String(genre.to_string()));

    if let Some(value) = world_output.get("world_setting") {
        blueprint.insert("world_setting".to_string(), value.clone());
    }
    if let Some(value) = characters_output.get("characters") {
        blueprint.insert("characters".to_string(), value.clone());
    }
    if let Some(value) = relationships_output.get("relationships") {
        blueprint.insert("relationships".to_string(), value.clone());
    }

    blueprint.insert("chapter_outline".to_string(), Value::Array(Vec::new()));
    blueprint.insert(
        "needs_part_outlines".to_string(),
        Value::Bool(needs_part_outlines_for_chapters(target_length)),
    );
    blueprint.insert("total_chapters".to_string(), Value::from(target_length));
    blueprint.insert(
        "chapters_per_part".to_string(),
        Value::from(CHAPTERS_PER_PART),
    );

    Value::Object(blueprint)
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn assembles_stage_outputs_and_derived_chapter_fields() {
        let assembled = assemble_staged_blueprint_json(
            "回退标题",
            "回退类型",
            60,
            &json!({
                "title": "星潮档案",
                "genre": "科幻奇幻",
                "style": "冷峻写实",
                "full_synopsis": "主角追查星潮真相。",
            }),
            &json!({"world_setting": {"premise": "记忆可以交易"}}),
            &json!({"characters": [{"name": "林烬"}]}),
            &json!({"relationships": [{"character_a": "林烬", "character_b": "沈白"}]}),
        );

        assert_eq!(assembled["title"], "星潮档案");
        assert_eq!(assembled["genre"], "科幻奇幻");
        assert_eq!(assembled["world_setting"]["premise"], "记忆可以交易");
        assert_eq!(assembled["characters"][0]["name"], "林烬");
        assert_eq!(assembled["relationships"][0]["character_b"], "沈白");
        assert_eq!(assembled["total_chapters"], 60);
        assert_eq!(assembled["chapters_per_part"], CHAPTERS_PER_PART);
        assert_eq!(assembled["needs_part_outlines"], true);
        assert_eq!(assembled["chapter_outline"], json!([]));
    }

    #[test]
    fn uses_title_and_genre_fallbacks_without_overwriting_stage_values() {
        let fallback = assemble_staged_blueprint_json(
            "回退标题",
            "回退类型",
            10,
            &json!({}),
            &json!({}),
            &json!({}),
            &json!({}),
        );

        assert_eq!(fallback["title"], "回退标题");
        assert_eq!(fallback["genre"], "回退类型");
        assert_eq!(fallback["needs_part_outlines"], false);
    }
}
