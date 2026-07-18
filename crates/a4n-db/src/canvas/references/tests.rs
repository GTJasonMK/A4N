use super::*;
use serde_json::json;

#[test]
fn missing_image_cleanup_removes_only_deleted_generated_image_refs() {
    let mut config = json!({
        "generated_image_id": 10,
        "source_image_id": 11,
        "reference_image_ids": [10, 12],
        "title": "still valid"
    });
    let object = config.as_object_mut().expect("object config");

    remove_config_any_i64_refs(object, IMAGE_REF_CONFIG_KEYS, &[10]);
    remove_config_i64_array_refs_for_any(object, "reference_image_ids", &[10]);

    assert!(config.get("generated_image_id").is_none());
    assert_eq!(config["source_image_id"], json!(11));
    assert_eq!(config["reference_image_ids"], json!([12]));
    assert_eq!(config["title"], json!("still valid"));
}
