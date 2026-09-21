use serde_json::{Value, json};

pub fn reverse_outline_apply_result(outline_count: usize) -> Value {
    json!({ "outline_count": outline_count })
}

pub fn reverse_part_outline_apply_result(part_count: usize) -> Value {
    json!({ "part_outline_count": part_count })
}
