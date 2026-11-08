use serde_json::{Value, json};

pub fn part_outline_output_schema() -> Value {
    json!({"type":"object","additionalProperties":false,
        "required":["title","summary","theme","key_events","character_focus"],
        "properties":{
            "title":nonblank_text(),"summary":nonblank_text(),"theme":nonblank_text(),
            "key_events":{"type":"array","minItems":3,"items":{}},
            "character_focus":{"type":"object"}
        }
    })
}

pub fn chapter_outline_output_schema(count: Option<usize>) -> Value {
    let mut chapters = json!({"type":"array","items":{
        "type":"object","required":["title","summary"],"additionalProperties":false,
        "properties":{
            "title":nonblank_text(),"summary":nonblank_text(),
            "beats":{"type":"array"},"key_scenes":{"type":"array"},
            "foreshadowing_setup":{"type":"array"},"foreshadowing_payoff":{"type":"array"},
            "characters_involved":{"type":"array"},"pacing_score":{"type":["number","null"],"minimum":f32::MIN as f64,"maximum":f32::MAX as f64},
            "word_count_target":{"type":["integer","null"],"minimum":1,"maximum":i32::MAX}
        }
    }});
    if let Some(count) = count {
        chapters["minItems"] = json!(count);
        chapters["maxItems"] = json!(count);
    }
    json!({"type":"object","required":["chapters"],"additionalProperties":false,"properties":{"chapters":chapters}})
}

fn nonblank_text() -> Value {
    json!({"type":"string","minLength":1,"pattern":"\\S"})
}
