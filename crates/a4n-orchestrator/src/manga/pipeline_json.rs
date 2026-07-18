mod fields;
mod page_plan;
mod page_prompts;

pub(crate) use fields::required_object_field;
pub(crate) use page_plan::{page_plan_page_number, page_plan_pages, parse_manga_page_plan};
pub(crate) use page_prompts::{
    PageDrawingPrompt, page_drawing_prompt_from_response, validate_page_drawing_prompts,
};

pub(crate) fn manga_llm_output_schema(
    prompt_name: &str,
) -> Result<serde_json::Value, a4n_core::error::AppError> {
    use serde_json::json;
    let string_array = || json!({"type": "array", "items": {"type": "string"}});
    let schema = match prompt_name {
        "manga_character_binding" => json!({
            "type": "object", "additionalProperties": false, "required": ["characters"],
            "properties": {"characters": {"type": "object", "additionalProperties": {
                "type": "object", "additionalProperties": false,
                "required": ["name", "appearance", "personality", "role", "gender", "age_description", "source_span_ids"],
                "properties": {
                    "name": {"type": "string"}, "appearance": {"type": "string"},
                    "personality": {"type": "string"},
                    "role": {"enum": ["protagonist", "antagonist", "supporting", "minor", "background"]},
                    "gender": {"enum": ["male", "female", "unknown"]},
                    "age_description": {"type": "string"}, "source_span_ids": string_array()
                }
            }}}
        }),
        "manga_speech_selection" => json!({
            "type": "object", "additionalProperties": false, "required": ["dialogues", "narrations"],
            "properties": {
                "dialogues": {"type": "array", "items": {
                    "type": "object", "additionalProperties": false,
                    "required": ["index", "speaker", "content", "source_span_id", "emotion", "target", "beat_ref", "is_internal", "bubble_type"],
                    "properties": {
                        "index": {"type": "integer"}, "speaker": {"type": "string"},
                        "content": {"type": "string"}, "source_span_id": {"type": "string"},
                        "emotion": {"type": "string"}, "target": {"type": ["string", "null"]},
                        "beat_ref": manga_beat_ref_schema(), "is_internal": {"type": "boolean"},
                        "bubble_type": {"type": "string"}
                    }
                }},
                "narrations": {"type": "array", "items": {
                    "type": "object", "additionalProperties": false,
                    "required": ["index", "content", "source_span_id", "narration_type", "beat_ref", "position"],
                    "properties": {
                        "index": {"type": "integer"}, "content": {"type": "string"},
                        "source_span_id": {"type": "string"}, "narration_type": {"type": "string"},
                        "beat_ref": manga_beat_ref_schema(), "position": {"type": "string"}
                    }
                }}
            }
        }),
        "manga_visual_anchor_extraction" => json!({
            "type": "object", "additionalProperties": false, "required": ["scenes"],
            "properties": {"scenes": {"type": "array", "items": {
                "type": "object", "additionalProperties": false,
                "required": ["index", "location", "location_en", "time_of_day", "atmosphere", "weather", "lighting", "indoor_outdoor", "description", "beat_refs", "source_span_ids"],
                "properties": {
                    "index": {"type": "integer"}, "location": {"type": "string"},
                    "location_en": {"type": "string"}, "time_of_day": {"type": "string"},
                    "atmosphere": {"type": "string"}, "weather": {"type": ["string", "null"]},
                    "lighting": {"type": "string"}, "indoor_outdoor": {"enum": ["indoor", "outdoor"]},
                    "description": {"type": "string"},
                    "beat_refs": {"type": "array", "items": manga_beat_ref_schema()},
                    "source_span_ids": string_array()
                }
            }}}
        }),
        "manga_item_extraction" => json!({
            "type": "object", "additionalProperties": false, "required": ["items"],
            "properties": {"items": {"type": "array", "items": {
                "type": "object", "additionalProperties": false,
                "required": ["name", "description", "description_en", "importance", "first_appearance_beat_ref", "visual_features", "source_span_ids"],
                "properties": {
                    "name": {"type": "string"}, "description": {"type": "string"},
                    "description_en": {"type": "string"}, "importance": {"type": "string"},
                    "first_appearance_beat_ref": manga_beat_ref_schema(),
                    "visual_features": {"type": "string"}, "source_span_ids": string_array()
                }
            }}}
        }),
        "manga_chapter_comic_synthesis" => json!({
            "type": "object", "additionalProperties": false,
            "required": ["chapter_summary", "mood_progression", "total_estimated_pages"],
            "properties": {
                "chapter_summary": {"type": "string"},
                "mood_progression": string_array(),
                "total_estimated_pages": {"type": "integer"}
            }
        }),
        "manga_page_planning" => {
            serde_json::to_value(schemars::schema_for!(a4n_core::manga::MangaPagePlanV3))?
        }
        "manga_page_drawing_prompt" => serde_json::to_value(schemars::schema_for!(
            a4n_core::manga::MangaPageDrawingPromptV2
        ))?,
        other => {
            return Err(a4n_core::error::AppError::ConfigError(format!(
                "未注册的漫画结构化输出合同: {other}"
            )));
        }
    };
    Ok(schema)
}

fn manga_beat_ref_schema() -> serde_json::Value {
    serde_json::json!({
        "type": "object", "additionalProperties": false,
        "required": ["scene_index", "shot_index", "beat_index"],
        "properties": {
            "scene_index": {"type": "integer"},
            "shot_index": {"type": "integer"},
            "beat_index": {"type": "integer"}
        }
    })
}

#[cfg(test)]
mod tests;
