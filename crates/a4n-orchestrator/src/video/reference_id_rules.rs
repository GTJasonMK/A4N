//! 参考图 ID 列表纯规则（无 DB、无 Host IO）。

use serde_json::Value;

/// 从 JSON 数组解析正整数参考图 ID，并拒绝重复。
pub fn reference_image_ids(value: Option<&Value>) -> Result<Vec<i64>, String> {
    let Some(value) = value else {
        return Ok(Vec::new());
    };
    let values = value
        .as_array()
        .ok_or_else(|| "reference_image_ids 必须是数组".to_string())?;
    let mut ids = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let id = value
            .as_i64()
            .filter(|id| *id > 0)
            .ok_or_else(|| format!("reference_image_ids[{index}] 必须是正整数"))?;
        if ids.contains(&id) {
            return Err(format!("reference_image_ids 包含重复 ID：{id}"));
        }
        ids.push(id);
    }
    Ok(ids)
}

/// 校验已解析的参考图 ID 列表（正整数、无重复）。
pub fn ensure_reference_image_ids_valid(ids: &[i64]) -> Result<(), String> {
    let mut seen = Vec::with_capacity(ids.len());
    for (index, id) in ids.iter().copied().enumerate() {
        if id <= 0 {
            return Err(format!("reference_image_ids[{index}] 必须是正整数"));
        }
        if seen.contains(&id) {
            return Err(format!("reference_image_ids 包含重复 ID：{id}"));
        }
        seen.push(id);
    }
    Ok(())
}

/// 从 batch 图片项上的 `reference_image_ids` 字段解析 ID 列表。
pub fn batch_item_reference_image_ids(item: &Value) -> Result<Vec<i64>, String> {
    let Some(value) = item
        .get("reference_image_ids")
        .filter(|value| !value.is_null())
    else {
        return Ok(Vec::new());
    };
    reference_image_ids(Some(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn accepts_missing_and_integer_array() {
        assert_eq!(reference_image_ids(None), Ok(Vec::new()));
        assert_eq!(reference_image_ids(Some(&json!([3, 5]))), Ok(vec![3, 5]));
        assert_eq!(batch_item_reference_image_ids(&json!({})), Ok(Vec::new()));
        assert_eq!(
            batch_item_reference_image_ids(&json!({"reference_image_ids": [1, 2]})),
            Ok(vec![1, 2])
        );
    }

    #[test]
    fn rejects_invalid_or_duplicate_values() {
        for value in [json!("3"), json!(["3"]), json!([0]), json!([3, 3])] {
            reference_image_ids(Some(&value)).expect_err("invalid ids should fail");
        }
        ensure_reference_image_ids_valid(&[1, 1]).expect_err("dup");
        ensure_reference_image_ids_valid(&[0]).expect_err("non-positive");
    }
}
