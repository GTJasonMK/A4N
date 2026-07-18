use serde_json::Value;

pub(super) fn map_value_to_string_list(value: Option<&Value>, limit: usize) -> Vec<String> {
    let Some(Value::Object(map)) = value else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (key, val) in map {
        if out.len() >= limit {
            break;
        }
        let summary = if let Some(text) = val.as_str().map(str::trim).filter(|s| !s.is_empty()) {
            text.to_owned()
        } else if let Some(obj) = val.as_object() {
            let bits = [
                "summary",
                "description",
                "state_summary",
                "content",
                "goal",
                "location",
            ]
            .into_iter()
            .filter_map(|field| {
                obj.get(field)
                    .and_then(Value::as_str)
                    .map(str::trim)
                    .filter(|s| !s.is_empty())
                    .map(ToOwned::to_owned)
            })
            .collect::<Vec<_>>();
            if bits.is_empty() {
                continue;
            }
            bits.join("；")
        } else {
            continue;
        };
        out.push(format!("{key}: {summary}"));
    }
    out
}
