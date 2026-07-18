use serde_json::Value;

pub fn format_json_for_rag(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.trim().to_string(),
        Value::Array(items) => items
            .iter()
            .enumerate()
            .filter_map(|(index, item)| {
                let text = format_json_for_rag(item);
                if text.trim().is_empty() {
                    None
                } else {
                    Some(format!("{}. {}", index + 1, text))
                }
            })
            .collect::<Vec<_>>()
            .join("\n"),
        Value::Object(object) => {
            if let Some(text) = format_named_object(object) {
                return text;
            }
            object
                .iter()
                .filter_map(|(key, value)| {
                    let text = format_json_for_rag(value);
                    if text.trim().is_empty() {
                        None
                    } else {
                        Some(format!("{}: {}", field_label(key), text))
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
    }
}

fn format_named_object(object: &serde_json::Map<String, Value>) -> Option<String> {
    let name = object
        .get("name")
        .or_else(|| object.get("title"))
        .or_else(|| object.get("mystery"))
        .or_else(|| object.get("period"))
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|text| !text.is_empty())?;

    let details = object
        .iter()
        .filter_map(|(key, value)| {
            if matches!(key.as_str(), "name" | "title" | "mystery" | "period") {
                return None;
            }
            let text = format_json_for_rag(value);
            if text.trim().is_empty() {
                None
            } else {
                Some(format!("{}: {}", field_label(key), text))
            }
        })
        .collect::<Vec<_>>();

    if details.is_empty() {
        Some(name.to_string())
    } else {
        Some(format!("{name}\n{}", details.join("\n")))
    }
}

fn field_label(key: &str) -> &'static str {
    match key {
        "description" => "描述",
        "story_function" => "叙事功能",
        "agenda" => "目标/诉求",
        "representative_characters" => "代表角色",
        "event" => "事件",
        "impact" => "影响",
        "source" => "来源",
        "rules" => "规则",
        "limits" => "限制",
        "costs" => "代价",
        "values" => "价值观",
        "class_structure" => "阶层结构",
        "daily_life" => "日常生活",
        "stakes" => "失败代价",
        "early_clues" => "早期线索",
        "payoff_direction" => "回收方向",
        "location" | "current_location" => "位置",
        "goal" | "current_goal" => "目标",
        "state" | "status" => "状态",
        "emotion" | "emotional_state" => "情绪",
        "start" => "起点",
        "core_wound" => "核心伤口",
        "desire" => "表层欲望",
        "need" => "真正需求",
        "secret" => "秘密",
        "voice_style" => "声音风格",
        "turning_point" => "转折点",
        "end" => "终点",
        "tension" => "张力",
        "turning_points" => "关键转折",
        "hidden_truth" => "隐藏真相",
        _ => "字段",
    }
}
