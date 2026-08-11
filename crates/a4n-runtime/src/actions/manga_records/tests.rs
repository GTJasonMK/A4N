use super::{OptimizeMangaPromptInput, UpdateMangaPromptInput};
use serde_json::json;

#[test]
fn manga_prompt_inputs_reject_unknown_fields() {
    let update_err = serde_json::from_value::<UpdateMangaPromptInput>(json!({
        "prompt_id": 1,
        "image_prompt": "新的漫画提示词",
        "downstream_strategy": "reject",
        "legacy_notes": "旧字段",
    }))
    .expect_err("unknown update input fields should fail");
    assert!(update_err.to_string().contains("unknown field"));

    let optimize_err = serde_json::from_value::<OptimizeMangaPromptInput>(json!({
        "prompt_id": 1,
        "instruction": "增强构图",
        "downstream_strategy": "reject",
        "legacy_notes": "旧字段",
    }))
    .expect_err("unknown optimize input fields should fail");
    assert!(optimize_err.to_string().contains("unknown field"));
}
