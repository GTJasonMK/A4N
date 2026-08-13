use super::*;
use serde_json::json;

#[test]
fn missing_image_cleanup_removes_only_deleted_generated_image_refs() {
    let mut config = json!({
        "generated_image_id": 10,
        "title": "still valid"
    });
    let object = config.as_object_mut().expect("object config");

    remove_config_any_i64_ref(object, IMAGE_REF_CONFIG_KEY, &[10]);

    assert!(config.get("generated_image_id").is_none());
    assert_eq!(config["title"], json!("still valid"));
}
