use std::collections::HashSet;

use serde_json::{Value, json};

pub(in crate::manga) fn page_reference_bindings(
    page: &Value,
    reference_assets_metadata: &Value,
) -> Value {
    let portraits = reference_assets_metadata
        .get("character_portraits")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut bindings = Vec::new();
    let mut seen_slots = HashSet::<String>::new();

    let mut character_names = HashSet::new();
    for panel in page
        .get("panels")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
    {
        if let Some(value) = panel.get("participants") {
            collect_character_names_from_value(value, &mut character_names);
        }
    }

    let mut character_names = character_names.into_iter().collect::<Vec<_>>();
    character_names.sort();
    for name in character_names {
        let slot_id = format!("character:{name}");
        if !seen_slots.insert(slot_id.clone()) {
            continue;
        }
        let mut binding = json!({
            "slot_id": slot_id,
            "target_type": "character",
            "target_name": name,
            "label": name,
            "purpose": "保持本页角色外观一致",
            "required": true,
            "asset_kind": "character_portrait",
            "status": "missing",
        });
        bind_character_portrait_if_available(&mut binding, &portraits);
        bindings.push(binding);
    }

    Value::Array(bindings)
}

fn bind_character_portrait_if_available(binding: &mut Value, portraits: &[Value]) {
    let Some(name) = binding
        .get("target_name")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(ToString::to_string)
    else {
        return;
    };
    let Some(portrait) = portraits.iter().find(|portrait| {
        portrait
            .get("character_name")
            .and_then(Value::as_str)
            .is_some_and(|portrait_name| portrait_name.trim() == name)
    }) else {
        return;
    };
    let Some(image_path) = portrait
        .get("image_path")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|path| !path.is_empty())
    else {
        return;
    };
    let Some(portrait_id) = portrait.get("portrait_id").and_then(Value::as_i64) else {
        return;
    };

    if let Some(map) = binding.as_object_mut() {
        map.insert("asset_kind".into(), json!("character_portrait"));
        map.insert("status".into(), json!("bound"));
        map.insert(
            "bound_asset".into(),
            json!({
                "asset_kind": "character_portrait",
                "asset_id": portrait_id,
                "portrait_id": portrait_id,
                "profile_id": portrait.get("profile_id").cloned().unwrap_or(Value::Null),
                "label": name,
                "image_path": image_path,
                "prompt": portrait.get("prompt").cloned().unwrap_or(Value::Null),
                "portrait_type": portrait.get("portrait_type").cloned().unwrap_or(Value::Null),
                "expression": portrait.get("expression").cloned().unwrap_or(Value::Null),
                "pose": portrait.get("pose").cloned().unwrap_or(Value::Null),
                "is_primary": portrait.get("is_primary").cloned().unwrap_or(Value::Null),
            }),
        );
    }
}

fn collect_character_names_from_value(value: &Value, names: &mut HashSet<String>) {
    match value {
        Value::String(name) => {
            let name = name.trim();
            if !name.is_empty() {
                names.insert(name.to_string());
            }
        }
        Value::Array(items) => {
            for item in items {
                collect_character_names_from_value(item, names);
            }
        }
        Value::Object(map) => {
            for key in ["name", "character_name", "speaker"] {
                if let Some(Value::String(name)) = map.get(key) {
                    let name = name.trim();
                    if !name.is_empty() {
                        names.insert(name.to_string());
                    }
                }
            }
        }
        _ => {}
    }
}
