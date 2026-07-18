use serde_json::Value;

pub(super) fn value_to_string_list(value: Option<&Value>, limit: usize) -> Vec<String> {
    let Some(Value::Array(items)) = value else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for item in items {
        if out.len() >= limit {
            break;
        }
        match item {
            Value::String(text) => {
                let text = text.trim();
                if !text.is_empty() {
                    out.push(text.to_owned());
                }
            }
            Value::Number(number) => out.push(number.to_string()),
            Value::Object(obj) => {
                let parts = [
                    "name",
                    "title",
                    "description",
                    "summary",
                    "event",
                    "state_summary",
                    "goal",
                ]
                .into_iter()
                .filter_map(|key| {
                    obj.get(key)
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(ToOwned::to_owned)
                })
                .collect::<Vec<_>>();
                if !parts.is_empty() {
                    out.push(parts.join("："));
                }
            }
            _ => {}
        }
    }
    out
}
