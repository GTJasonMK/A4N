//! 通用 JSON 差异。

use serde_json::{Map, Value, json};

pub(super) fn diff_generic_json(from: &Value, to: &Value) -> Value {
    match (from, to) {
        (Value::Object(left), Value::Object(right)) => {
            let mut added = Map::new();
            let mut removed = Map::new();
            let mut changed = Map::new();

            for (key, left_value) in left {
                match right.get(key) {
                    Some(right_value) if right_value != left_value => {
                        changed.insert(key.clone(), json!({"from": left_value, "to": right_value}));
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
            json!({"added": added, "removed": removed, "changed": changed})
        }
        _ if from == to => json!({"added": {}, "removed": {}, "changed": {}}),
        _ => json!({"from": from, "to": to}),
    }
}
