use serde_json::{Map, Value};

pub(super) fn json_diff(a: &Value, b: &Value) -> Value {
    fn diff_recursive(a: &Value, b: &Value) -> Value {
        match (a, b) {
            (Value::Object(map_a), Value::Object(map_b)) => {
                let mut changes = Map::new();
                for (key, value_b) in map_b {
                    match map_a.get(key) {
                        Some(value_a) if value_a != value_b => {
                            changes.insert(key.clone(), diff_recursive(value_a, value_b));
                        }
                        None => {
                            changes.insert(key.clone(), serde_json::json!({"added": value_b}));
                        }
                        _ => {}
                    }
                }
                for key in map_a.keys() {
                    if !map_b.contains_key(key) {
                        changes.insert(key.clone(), serde_json::json!({"removed": &map_a[key]}));
                    }
                }
                Value::Object(changes)
            }
            _ if a != b => serde_json::json!({"old": a, "new": b}),
            _ => Value::Null,
        }
    }

    diff_recursive(a, b)
}
