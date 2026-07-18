use super::{
    OptimizeMangaPromptInput, OptimizedMangaPromptFields, UpdateMangaPromptInput,
    layout::{
        bind_reference_asset_in_layout, optional_layout_string, required_layout_bool,
        required_layout_i32, required_layout_string,
    },
    optimization::manga_prompt_optimization_checkpoint_output,
};
use serde_json::{Value, json};

#[test]
fn bind_reference_asset_updates_existing_slot() {
    let layout = json!({
        "reference_bindings": [{"slot_id": "hero", "status": "missing"}],
    });
    let bound = json!({"asset_kind": "generated_image", "image_id": 7});
    let updated = bind_reference_asset_in_layout(Some(layout), "hero", bound)
        .expect("existing slot should be updated");
    let binding = updated["reference_bindings"][0]
        .as_object()
        .expect("binding should be object");
    assert_eq!(binding.get("status"), Some(&json!("bound")));
    assert_eq!(binding.get("asset_kind"), Some(&json!("generated_image")));
    assert_eq!(
        binding.get("bound_asset"),
        Some(&json!({"asset_kind": "generated_image", "image_id": 7}))
    );
}

#[test]
fn bind_reference_asset_appends_missing_slot() {
    let bound = json!({"asset_kind": "character_portrait", "portrait_id": 9});
    let updated = bind_reference_asset_in_layout(None, "hero", bound)
        .expect("missing layout should be created");
    let bindings = updated["reference_bindings"]
        .as_array()
        .expect("bindings should be array");
    assert_eq!(bindings.len(), 1);
    assert_eq!(bindings[0]["slot_id"], json!("hero"));
    assert_eq!(bindings[0]["status"], json!("bound"));
}

#[test]
fn bind_reference_asset_rejects_non_object_layout() {
    let err = bind_reference_asset_in_layout(Some(Value::Array(Vec::new())), "hero", json!({}))
        .expect_err("non object layout should fail");
    assert_eq!(err, "整页漫画提示词 layout 必须是 JSON 对象");
}

#[test]
fn bind_reference_asset_rejects_non_array_reference_bindings() {
    let layout = json!({"reference_bindings": {"slot_id": "hero"}});
    let err = bind_reference_asset_in_layout(
        Some(layout),
        "hero",
        json!({"asset_kind": "generated_image"}),
    )
    .expect_err("non-array reference_bindings should fail");
    assert_eq!(err, "整页漫画提示词 reference_bindings 必须是数组");
}

#[test]
fn bind_reference_asset_rejects_missing_asset_kind() {
    let err = bind_reference_asset_in_layout(None, "hero", json!({"image_id": 1}))
        .expect_err("bound asset kind must be explicit");
    assert_eq!(err, "绑定参考图素材缺少 asset_kind");
}

#[test]
fn pipeline_layout_fields_validate_types() {
    let layout = json!({
        "pipeline_status": "running",
        "current": 2,
        "can_cancel": true,
        "task_id": null,
    });
    assert_eq!(
        required_layout_string(&layout, "pipeline_status").unwrap(),
        "running"
    );
    assert_eq!(required_layout_i32(&layout, "current").unwrap(), 2);
    assert!(required_layout_bool(&layout, "can_cancel").unwrap());
    assert_eq!(optional_layout_string(&layout, "task_id").unwrap(), None);
}

#[test]
fn manga_prompt_inputs_reject_unknown_fields() {
    let update_err = serde_json::from_value::<UpdateMangaPromptInput>(json!({
        "prompt_id": 1,
        "image_prompt": "新的漫画提示词",
        "legacy_notes": "旧字段",
    }))
    .expect_err("unknown update input fields should fail");
    assert!(update_err.to_string().contains("unknown field"));

    let optimize_err = serde_json::from_value::<OptimizeMangaPromptInput>(json!({
        "prompt_id": 1,
        "instruction": "增强构图",
        "legacy_notes": "旧字段",
    }))
    .expect_err("unknown optimize input fields should fail");
    assert!(optimize_err.to_string().contains("unknown field"));
}

#[test]
fn manga_prompt_optimization_output_rejects_unknown_fields() {
    let err = serde_json::from_value::<OptimizedMangaPromptFields>(json!({
        "image_prompt": "优化后的漫画整页提示词",
        "legacy_notes": "旧字段",
    }))
    .expect_err("unknown optimized fields should fail");
    assert!(err.to_string().contains("unknown field"));
}

#[test]
fn manga_prompt_optimization_checkpoint_revalidates_parsed_output() {
    let checkpoint = json!({
        "response_content": r#"{"image_prompt":"优化后的漫画整页提示词"}"#,
        "parsed_output": {"image_prompt": "优化后的漫画整页提示词"},
        "usage": null,
        "finish_reason": "stop",
    });
    let (output, usage, finish_reason) =
        manga_prompt_optimization_checkpoint_output(&checkpoint, "optimize_manga_prompt:prompt:1")
            .expect("valid checkpoint should restore");
    assert_eq!(output.image_prompt, "优化后的漫画整页提示词");
    assert_eq!(usage, None);
    assert_eq!(finish_reason.as_deref(), Some("stop"));
}

#[test]
fn manga_prompt_optimization_checkpoint_restores_repaired_parsed_output() {
    let checkpoint = json!({
        "response_content": r#"{"image_prompt":"响应里的提示词"}"#,
        "parsed_output": {"image_prompt": "断点里的提示词"},
        "usage": null,
        "finish_reason": "stop",
    });
    let (output, _, _) =
        manga_prompt_optimization_checkpoint_output(&checkpoint, "optimize_manga_prompt:prompt:1")
            .expect("repaired checkpoint should restore parsed_output");
    assert_eq!(output.image_prompt, "断点里的提示词");
}
