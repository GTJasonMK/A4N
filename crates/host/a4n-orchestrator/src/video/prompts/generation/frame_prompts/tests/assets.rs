use super::*;

#[test]
fn appends_previous_boundary_frame_as_regular_available_asset() {
    let available_assets = json!([
        {"image_id": 3, "display_order": 1, "display_label": "参考图 1"},
        {"image_id": 9, "display_order": 2, "display_label": "参考图 2"}
    ]);
    let previous_frame = frame(1, vec![3, 9]);

    let expanded = available_assets_with_previous_boundary_frame(
        &available_assets,
        Some(13),
        Some(&previous_frame),
    );

    let items = expanded
        .as_array()
        .expect("expanded assets should be array");
    assert_eq!(items.len(), 3);
    assert_eq!(items[2].get("image_id"), Some(&json!(13)));
    assert_eq!(items[2].get("display_label"), Some(&json!("参考图 3")));
    assert_eq!(
        previous_boundary_frame_asset_context(&expanded, Some(13))
            .and_then(|value| value.get("slot_label").cloned()),
        Some(json!("参考图 1"))
    );
    assert!(
        previous_boundary_frame_asset_context(&expanded, Some(13))
            .and_then(|value| value.get("image_id").cloned())
            .is_none()
    );
}

#[test]
fn frame_prompt_available_assets_hide_prompt_visible_asset_labels() {
    let available_assets = json!([
        {"image_id": 3, "display_order": 1, "display_label": "参考图 1", "label": "角色主图"},
        {"image_id": 9, "display_order": 2, "display_label": "参考图 2", "label": "场景主图"}
    ]);

    let prompt_assets = frame_prompt_available_assets_context(&available_assets)
        .expect("available assets should be sanitized");
    let items = prompt_assets
        .as_array()
        .expect("prompt assets should exist");
    let item = items.first().expect("prompt asset should exist");

    assert!(item.get("display_label").is_none());
    assert!(
        items
            .get(1)
            .and_then(|item| item.get("display_label"))
            .is_none()
    );
    assert!(item.get("candidate_label").is_none());
    assert!(item.get("source_display_label").is_none());
}

#[test]
fn frame_prompt_available_assets_do_not_expose_internal_image_ids() {
    let available_assets = json!([
        {
            "image_id": 59,
            "display_order": 1,
            "display_label": "参考图 1",
            "asset_id": "manual_library_asset_1",
            "source_id": "manual_library_asset_1",
            "candidate_label": "manual_library_asset_1",
            "source_display_label": "参考图 1",
            "project_id": 9,
            "profile_id": 7,
            "role": "chapter_material",
            "label": "manual_library_asset_1",
            "description": "林清辞角色参考 manual_library_asset_1",
            "prompt_excerpt": "清晨教室窗边的角色主图",
            "selection_reason": "manual_library",
            "profile_kind": "asset_id 59",
            "is_primary": true,
            "is_favorite": false
        }
    ]);

    let prompt_assets = frame_prompt_available_assets_context(&available_assets)
        .expect("available assets should be sanitized");
    let item = prompt_assets
        .as_array()
        .and_then(|items| items.first())
        .expect("prompt asset should exist");
    let serialized = serde_json::to_string(item).expect("asset context should serialize");

    assert!(item.get("image_id").is_none());
    assert!(item.get("display_label").is_none());
    assert!(item.get("label").is_none());
    assert!(item.get("profile_kind").is_none());
    assert_eq!(item.get("description"), Some(&json!("林清辞角色参考")));
    assert_eq!(
        item.get("prompt_excerpt"),
        Some(&json!("清晨教室窗边的角色主图"))
    );
    assert!(item.get("display_order").is_none());
    assert!(item.get("project_id").is_none());
    assert!(item.get("profile_id").is_none());
    assert!(item.get("asset_id").is_none());
    assert!(item.get("source_id").is_none());
    assert!(item.get("candidate_label").is_none());
    assert!(item.get("source_display_label").is_none());
    assert!(!serialized.contains("manual_library_asset_1"));
    assert!(!serialized.contains("候选素材"));
    assert!(!serialized.contains("asset_id"));
}

#[test]
fn reference_slots_do_not_repeat_internal_image_ids() {
    let slots = reference_slots_for_frame(2, Some("previous_boundary_frame"));
    let first = slots
        .as_array()
        .and_then(|items| items.first())
        .expect("reference slot should exist");

    assert_eq!(first.get("slot_label"), Some(&json!("参考图 1")));
    assert!(first.get("image_id").is_none());
}
