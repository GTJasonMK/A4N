use serde_json::Value;

use a4n_error::AppError;

use super::ForeshadowingEntry;
use super::value::first_string;

pub fn foreshadowing_entries(analysis: &Value) -> Vec<ForeshadowingEntry> {
    let Some(planted) = analysis
        .get("foreshadowing")
        .and_then(|value| value.get("planted"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };

    planted
        .iter()
        .filter_map(|item| {
            first_string(item, &["description"]).map(|description| ForeshadowingEntry {
                id: first_string(item, &["id"]),
                description,
                original_text: first_string(item, &["original_text"]),
                category: first_string(item, &["category"]),
                priority: first_string(item, &["priority"]),
                related_entities: item.get("related_entities").cloned(),
            })
        })
        .collect()
}

pub fn priority_to_importance(priority: Option<&str>) -> Result<i32, AppError> {
    let priority = priority
        .map(str::trim)
        .filter(|text| !text.is_empty())
        .ok_or_else(|| AppError::InvalidArgument("伏笔优先级缺失，不能写入重要度".into()))?;
    match priority.to_ascii_lowercase().as_str() {
        "high" | "重要" | "高" => Ok(5),
        "medium" | "中" => Ok(3),
        "low" | "低" => Ok(1),
        _ => Err(AppError::InvalidArgument(format!(
            "伏笔优先级非法: {priority}"
        ))),
    }
}
