use serde_json::{Value, json};

pub fn chapter_outline_batch_response_schema() -> Value {
    json!({
        "type": "object",
        "additionalProperties": false,
        "required": ["chapters"],
        "properties": {
            "chapters": {
                "type": "array",
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "required": ["title", "summary"],
                    "properties": {
                        "title": {"type": "string"},
                        "summary": {"type": "string"},
                        "beats": {"type": "array", "items": {}},
                        "key_scenes": {"type": "array", "items": {}},
                        "pacing_score": {"type": "number"},
                        "foreshadowing_setup": {"type": "array", "items": {}},
                        "foreshadowing_payoff": {"type": "array", "items": {}},
                        "characters_involved": {"type": "array", "items": {}},
                        "word_count_target": {"type": "integer"}
                    }
                }
            }
        }
    })
}
