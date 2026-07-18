use a4n_core::models::chapter::chapter_manga_prompt;
use serde_json::{Value, json};

pub(super) fn optimization_input_fingerprint(
    panel: &chapter_manga_prompt::Model,
    instruction: &str,
    request_json: Value,
) -> Value {
    json!({
        "prompt_names": [
            "manga_prompt_optimization",
            "manga_prompt_optimization_user",
        ],
        "request": request_json,
        "input_variables": {
            "instruction": instruction,
            "prompt_id": panel.id,
        },
        "source_refs": {
            "previous_prompt_id": panel.id,
            "previous_image_prompt": &panel.image_prompt,
            "dialogue_text": &panel.dialogue_text,
            "camera_angle": &panel.camera_angle,
            "dialogue_position": &panel.dialogue_position,
            "layout": &panel.layout,
            "characters_in_frame": &panel.characters_in_frame,
        },
    })
}
