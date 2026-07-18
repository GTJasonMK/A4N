use serde_json::Value;

use crate::error::AppError;

pub fn normalize_grok_reference_mode_options_json(
    enabled: bool,
    kind: &str,
    value: Option<Value>,
) -> Result<Option<Value>, AppError> {
    let Some(value) = value.filter(|value| !value.is_null()) else {
        return Ok(None);
    };
    if !enabled {
        return Ok(None);
    }
    let Value::Object(mut map) = value else {
        return Err(AppError::InvalidArgument(format!(
            "{kind} provider_options_json 必须是 JSON 对象"
        )));
    };
    let Some(mode) = map
        .remove("grok_reference_mode")
        .filter(|value| !value.is_null())
    else {
        return Ok(None);
    };
    let mode = mode.as_str().map(str::trim).ok_or_else(|| {
        AppError::InvalidArgument(format!(
            "{kind} provider_options_json.grok_reference_mode 必须是字符串"
        ))
    })?;
    let normalized = match mode {
        "reference" | "first_frame" => mode,
        other => {
            return Err(AppError::InvalidArgument(format!(
                "{kind} provider_options_json.grok_reference_mode 无效：{other}"
            )));
        }
    };
    Ok(Some(serde_json::json!({
        "grok_reference_mode": normalized,
    })))
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::normalize_grok_reference_mode_options_json;

    #[test]
    fn normalizes_supported_reference_modes() {
        assert_eq!(
            normalize_grok_reference_mode_options_json(
                true,
                "图片",
                Some(json!({"grok_reference_mode": " first_frame "})),
            )
            .expect("supported mode"),
            Some(json!({"grok_reference_mode": "first_frame"}))
        );
    }

    #[test]
    fn disabled_or_missing_mode_produces_no_options() {
        assert_eq!(
            normalize_grok_reference_mode_options_json(
                false,
                "视频",
                Some(json!({"grok_reference_mode": "reference"})),
            )
            .expect("disabled mode"),
            None
        );
        assert_eq!(
            normalize_grok_reference_mode_options_json(true, "视频", Some(json!({})))
                .expect("missing mode"),
            None
        );
    }

    #[test]
    fn rejects_non_object_and_unsupported_modes() {
        assert!(
            normalize_grok_reference_mode_options_json(true, "图片", Some(json!([])))
                .expect_err("non-object options")
                .to_string()
                .contains("必须是 JSON 对象")
        );
        assert!(
            normalize_grok_reference_mode_options_json(
                true,
                "图片",
                Some(json!({"grok_reference_mode": "legacy"})),
            )
            .expect_err("unsupported mode")
            .to_string()
            .contains("grok_reference_mode 无效：legacy")
        );
    }
}
