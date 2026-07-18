use a4n_core::chapter_analysis::{
    CHAPTER_ANALYSIS_STAGE_CHARACTER_STATE, CHAPTER_ANALYSIS_STAGE_FACTS,
    CHAPTER_ANALYSIS_STAGE_FORESHADOWING_EVENTS, CHAPTER_ANALYSIS_STAGE_SUMMARY,
};
use a4n_core::error::AppError;
use serde_json::{Value, json};

pub(super) fn chapter_analysis_stage_schema(stage: &str) -> Result<Value, AppError> {
    let string_array = || json!({"type": "array", "items": {"type": "string"}});
    match stage {
        CHAPTER_ANALYSIS_STAGE_FACTS => Ok(json!({
            "type": "object",
            "additionalProperties": false,
            "required": ["characters", "locations", "items", "event_candidates", "foreshadowing_candidates"],
            "properties": {
                "characters": {"type": "array", "items": entity_candidate_schema()},
                "locations": {"type": "array", "items": entity_candidate_schema()},
                "items": {"type": "array", "items": entity_candidate_schema()},
                "event_candidates": {"type": "array", "items": {
                    "type": "object", "additionalProperties": false,
                    "required": ["event_ref", "type", "description", "importance", "involved_characters", "source_span_ids"],
                    "properties": {
                        "event_ref": {"type": "string"},
                        "type": {"enum": ["battle", "revelation", "relationship", "discovery", "decision", "death", "arrival", "departure"]},
                        "description": {"type": "string"},
                        "importance": {"enum": ["high", "medium", "low"]},
                        "involved_characters": string_array(),
                        "source_span_ids": string_array()
                    }
                }},
                "foreshadowing_candidates": {"type": "array", "items": {
                    "type": "object", "additionalProperties": false,
                    "required": ["foreshadowing_ref", "description", "category", "priority", "related_entities", "original_text_span_id"],
                    "properties": {
                        "foreshadowing_ref": {"type": "string"},
                        "description": {"type": "string"},
                        "category": {"enum": ["character_secret", "plot_twist", "item_mystery", "world_rule"]},
                        "priority": {"enum": ["high", "medium", "low"]},
                        "related_entities": string_array(),
                        "original_text_span_id": {"type": "string"}
                    }
                }}
            }
        })),
        CHAPTER_ANALYSIS_STAGE_SUMMARY => Ok(json!({
            "type": "object", "additionalProperties": false,
            "required": ["metadata_additions", "summaries"],
            "properties": {
                "metadata_additions": {
                    "type": "object", "additionalProperties": false,
                    "required": ["tags", "tone", "timeline_marker"],
                    "properties": {
                        "tags": string_array(),
                        "tone": {"type": ["string", "null"]},
                        "timeline_marker": {"type": ["string", "null"]}
                    }
                },
                "summaries": {
                    "type": "object", "additionalProperties": false,
                    "required": ["compressed", "one_line", "keywords"],
                    "properties": {
                        "compressed": {"type": "string"},
                        "one_line": {"type": "string"},
                        "keywords": string_array()
                    }
                }
            }
        })),
        CHAPTER_ANALYSIS_STAGE_CHARACTER_STATE => Ok(json!({
            "type": "object", "additionalProperties": false,
            "required": ["character_states"],
            "properties": {
                "character_states": {
                    "type": "object",
                    "additionalProperties": {
                        "type": "object", "additionalProperties": false,
                        "required": ["location", "status", "changes", "emotional_state"],
                        "properties": {
                            "location": {"type": ["string", "null"]},
                            "status": {"type": ["string", "null"]},
                            "changes": string_array(),
                            "emotional_state": {"type": ["string", "null"]}
                        }
                    }
                }
            }
        })),
        CHAPTER_ANALYSIS_STAGE_FORESHADOWING_EVENTS => Ok(json!({
            "type": "object", "additionalProperties": false,
            "required": ["foreshadowing", "key_events"],
            "properties": {
                "foreshadowing": {
                    "type": "object", "additionalProperties": false,
                    "required": ["planted", "resolved", "tensions"],
                    "properties": {
                        "planted": {"type": "array", "items": foreshadowing_item_schema()},
                        "resolved": {"type": "array", "items": foreshadowing_item_schema()},
                        "tensions": string_array()
                    }
                },
                "key_events": {"type": "array", "items": {
                    "type": "object", "additionalProperties": false,
                    "required": ["type", "description", "importance", "involved_characters", "source_span_ids"],
                    "properties": {
                        "type": {"enum": ["battle", "revelation", "relationship", "discovery", "decision", "death", "arrival", "departure"]},
                        "description": {"type": "string"},
                        "importance": {"enum": ["high", "medium", "low"]},
                        "involved_characters": string_array(),
                        "source_span_ids": string_array()
                    }
                }}
            }
        })),
        other => Err(AppError::ConfigError(format!(
            "未注册的章节分析结构化输出合同: {other}"
        ))),
    }
}

fn entity_candidate_schema() -> Value {
    json!({
        "type": "object", "additionalProperties": false,
        "required": ["name", "source_span_ids"],
        "properties": {
            "name": {"type": "string"},
            "source_span_ids": {"type": "array", "items": {"type": "string"}}
        }
    })
}

fn foreshadowing_item_schema() -> Value {
    json!({
        "type": "object", "additionalProperties": false,
        "required": ["description", "original_text_span_id", "category", "priority", "related_entities"],
        "properties": {
            "description": {"type": "string"},
            "original_text_span_id": {"type": "string"},
            "category": {"enum": ["character_secret", "plot_twist", "item_mystery", "world_rule"]},
            "priority": {"enum": ["high", "medium", "low"]},
            "related_entities": {"type": "array", "items": {"type": "string"}}
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_registered_stage_schemas_are_strict_objects() {
        for stage in [
            CHAPTER_ANALYSIS_STAGE_FACTS,
            CHAPTER_ANALYSIS_STAGE_SUMMARY,
            CHAPTER_ANALYSIS_STAGE_CHARACTER_STATE,
            CHAPTER_ANALYSIS_STAGE_FORESHADOWING_EVENTS,
        ] {
            let schema = chapter_analysis_stage_schema(stage).unwrap();

            assert_eq!(schema.get("type"), Some(&json!("object")), "{stage}");
            assert_eq!(
                schema.get("additionalProperties"),
                Some(&json!(false)),
                "{stage}"
            );
            assert!(
                schema
                    .get("required")
                    .and_then(Value::as_array)
                    .is_some_and(|required| !required.is_empty()),
                "{stage}"
            );
            assert!(
                schema
                    .get("properties")
                    .and_then(Value::as_object)
                    .is_some_and(|properties| !properties.is_empty()),
                "{stage}"
            );
        }
    }

    #[test]
    fn rejects_unregistered_stage_schema() {
        let error = chapter_analysis_stage_schema("unknown_stage").unwrap_err();

        match error {
            AppError::ConfigError(message) => {
                assert_eq!(message, "未注册的章节分析结构化输出合同: unknown_stage")
            }
            other => panic!("unexpected error: {other}"),
        }
    }
}
