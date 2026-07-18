use a4n_core::error::AppError;
use serde_json::{Value, json};

pub(in crate::video::prompt_context::builders) fn focused_segment_plan(
    segment_plan: &Value,
    current_segment: &Value,
) -> Result<Value, AppError> {
    let items = segment_plan_items(segment_plan);
    let current_number = required_segment_number(current_segment, "current_segment")?;
    let current_index = items
        .iter()
        .position(|segment| segment_number(segment) == Some(current_number))
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "视频提示词请求构造失败：片段规划中不存在 segment_number={current_number}"
            ))
        })?;

    let previous_segment = current_index
        .checked_sub(1)
        .and_then(|index| items.get(index))
        .cloned()
        .cloned();
    let next_segment = items.get(current_index + 1).cloned().cloned();

    Ok(json!({
        "segment_count": items.len(),
        "current_segment": current_segment,
        "previous_segment": previous_segment,
        "next_segment": next_segment
    }))
}

pub(super) fn segment_number(value: &Value) -> Option<i64> {
    value.get("segment_number").and_then(Value::as_i64)
}

pub(super) fn required_segment_number(value: &Value, context: &str) -> Result<i64, AppError> {
    segment_number(value).ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "视频提示词请求构造失败：{context} 缺少有效 segment_number"
        ))
    })
}

pub(super) fn segment_plan_items(segment_plan: &Value) -> Vec<&Value> {
    segment_plan
        .get("segments")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .collect()
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::focused_segment_plan;

    #[test]
    fn focused_segment_plan_requires_current_segment_number() {
        let err = focused_segment_plan(
            &json!({"segments": [{"segment_number": 1}]}),
            &json!({"title": "缺少编号"}),
        )
        .expect_err("current segment without stable number must be rejected");

        assert!(err.to_string().contains("缺少有效 segment_number"));
    }

    #[test]
    fn focused_segment_plan_rejects_unknown_current_segment() {
        let err = focused_segment_plan(
            &json!({"segments": [{"segment_number": 1}]}),
            &json!({"segment_number": 2}),
        )
        .expect_err("current segment outside the plan must be rejected");

        assert!(err.to_string().contains("不存在 segment_number=2"));
    }
}
