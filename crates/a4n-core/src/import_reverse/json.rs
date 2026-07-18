//! Shared JSON primitives used by reverse-context projection and validation.

mod strings;

pub fn optional_string(value: &serde_json::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(serde_json::Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(ToOwned::to_owned)
}

pub use strings::{collect_json_strings, stringify_json_value};

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::optional_string;

    #[test]
    fn optional_string_trims_non_empty_values() {
        assert_eq!(
            optional_string(&json!({"name": " 林岚 "}), "name"),
            Some("林岚".into())
        );
    }

    #[test]
    fn optional_string_ignores_missing_and_blank_values() {
        let value = json!({"blank": "  "});

        assert_eq!(optional_string(&value, "missing"), None);
        assert_eq!(optional_string(&value, "blank"), None);
    }
}
