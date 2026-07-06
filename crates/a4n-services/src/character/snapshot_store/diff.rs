use super::Sha256SnapshotStore;
use a4n_core::error::AppError;

impl Sha256SnapshotStore {
    pub(super) fn diff_json(
        from: &serde_json::Value,
        to: &serde_json::Value,
    ) -> Result<serde_json::Value, AppError> {
        if let Some(delta) = diff_character_snapshot(from, to)? {
            return Ok(delta);
        }

        Ok(match (from, to) {
            (serde_json::Value::Object(left), serde_json::Value::Object(right)) => {
                let mut added = serde_json::Map::new();
                let mut removed = serde_json::Map::new();
                let mut changed = serde_json::Map::new();

                for (key, left_value) in left {
                    match right.get(key) {
                        Some(right_value) if right_value != left_value => {
                            changed.insert(
                                key.clone(),
                                serde_json::json!({
                                    "from": left_value,
                                    "to": right_value,
                                }),
                            );
                        }
                        Some(_) => {}
                        None => {
                            removed.insert(key.clone(), left_value.clone());
                        }
                    }
                }

                for (key, right_value) in right {
                    if !left.contains_key(key) {
                        added.insert(key.clone(), right_value.clone());
                    }
                }

                serde_json::json!({
                    "added": added,
                    "removed": removed,
                    "changed": changed,
                })
            }
            _ if from == to => serde_json::json!({
                "added": {},
                "removed": {},
                "changed": {},
            }),
            _ => serde_json::json!({
                "from": from,
                "to": to,
            }),
        })
    }
}

fn diff_character_snapshot(
    from: &serde_json::Value,
    to: &serde_json::Value,
) -> Result<Option<serde_json::Value>, AppError> {
    let has_character_attrs = from.get("attributes").is_some() || to.get("attributes").is_some();
    if !has_character_attrs {
        return Ok(None);
    }

    let explicit = diff_attributes(
        required_category_attributes(from, "explicit", "from")?,
        required_category_attributes(to, "explicit", "to")?,
    );
    let implicit = diff_attributes(
        required_category_attributes(from, "implicit", "from")?,
        required_category_attributes(to, "implicit", "to")?,
    );
    let social = diff_attributes(
        required_category_attributes(from, "social", "from")?,
        required_category_attributes(to, "social", "to")?,
    );

    Ok(Some(serde_json::json!({
        "attributes": {
            "explicit": explicit,
            "implicit": implicit,
            "social": social,
        },
        "summary": {
            "from_chapter": from.get("synced_chapter"),
            "to_chapter": to.get("synced_chapter"),
            "from_state_summary": from.get("state_summary"),
            "to_state_summary": to.get("state_summary"),
            "from_changes_in_chapter": from.get("changes_in_chapter"),
            "to_changes_in_chapter": to.get("changes_in_chapter"),
            "from_behaviors_in_chapter": from.get("behaviors_in_chapter"),
            "to_behaviors_in_chapter": to.get("behaviors_in_chapter"),
        }
    })))
}

fn required_category_attributes<'a>(
    value: &'a serde_json::Value,
    category: &str,
    side: &str,
) -> Result<&'a serde_json::Map<String, serde_json::Value>, AppError> {
    let direct_key = match category {
        "explicit" => "explicit_attributes",
        "implicit" => "implicit_attributes",
        "social" => "social_attributes",
        _ => category,
    };
    value
        .get("attributes")
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("角色快照 {side}.attributes 缺少或不是对象"))
        })?
        .get(direct_key)
        .and_then(serde_json::Value::as_object)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "角色快照 {side}.attributes.{direct_key} 缺少或不是对象"
            ))
        })
}

fn diff_attributes(
    from: &serde_json::Map<String, serde_json::Value>,
    to: &serde_json::Map<String, serde_json::Value>,
) -> serde_json::Value {
    let mut added = serde_json::Map::new();
    let mut modified = serde_json::Map::new();
    let mut deleted = serde_json::Map::new();

    for (key, value) in to {
        match from.get(key) {
            Some(old_value) if old_value != value => {
                modified.insert(
                    key.clone(),
                    serde_json::json!({
                        "from": old_value,
                        "to": value,
                    }),
                );
            }
            Some(_) => {}
            None => {
                added.insert(key.clone(), value.clone());
            }
        }
    }

    for (key, value) in from {
        if !to.contains_key(key) {
            deleted.insert(key.clone(), value.clone());
        }
    }

    serde_json::json!({
        "added": added,
        "modified": modified,
        "deleted": deleted,
    })
}
