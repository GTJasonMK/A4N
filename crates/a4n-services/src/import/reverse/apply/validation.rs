use crate::import::reverse_json::{
    optional_i32, reject_unknown_keys, required_i32, required_string,
};
use serde_json::Value;
use std::collections::BTreeSet;

pub(super) fn required_positive_i32(
    value: &Value,
    key: &str,
    context: &str,
) -> Result<i32, String> {
    let number = required_i32(value, key, context)?;
    ensure_positive_i32(number, key, context)?;
    Ok(number)
}

pub(super) fn optional_positive_i32(
    value: &Value,
    key: &str,
    context: &str,
) -> Result<Option<i32>, String> {
    let number = optional_i32(value, key, context)?;
    if let Some(number) = number {
        ensure_positive_i32(number, key, context)?;
    }
    Ok(number)
}

fn ensure_positive_i32(number: i32, key: &str, context: &str) -> Result<(), String> {
    if number > 0 {
        Ok(())
    } else {
        Err(format!("{context}.{key} 必须大于 0"))
    }
}

pub(super) fn required_array_clone(
    value: &Value,
    key: &str,
    context: &str,
) -> Result<Value, String> {
    let field = value
        .get(key)
        .ok_or_else(|| format!("{context} 缺少必填数组字段 {key}"))?;
    if !field.is_array() {
        return Err(format!("{context}.{key} 必须是数组"));
    }
    Ok(field.clone())
}

pub(super) fn required_object_clone(
    value: &Value,
    key: &str,
    context: &str,
) -> Result<Value, String> {
    let field = value
        .get(key)
        .ok_or_else(|| format!("{context} 缺少必填对象字段 {key}"))?;
    if !field.is_object() {
        return Err(format!("{context}.{key} 必须是对象"));
    }
    Ok(field.clone())
}

pub(super) fn validate_reverse_outline_items(outlines: &[Value]) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for (index, item) in outlines.iter().enumerate() {
        let context = format!("reverse_outline.outlines[{index}]");
        reject_unknown_keys(
            item,
            &["chapter_number", "title", "summary", "word_count_target"],
            &context,
        )?;
        let chapter_number = required_positive_i32(item, "chapter_number", &context)?;
        if !seen.insert(chapter_number) {
            return Err(format!(
                "reverse_outline.outlines 包含重复章节: {chapter_number}"
            ));
        }
        required_string(item, "title", &context)?;
        required_string(item, "summary", &context)?;
        optional_positive_i32(item, "word_count_target", &context)?;
    }
    Ok(())
}

pub(super) fn validate_reverse_part_outline_items(parts: &[Value]) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    let mut ranges = Vec::with_capacity(parts.len());
    for (index, item) in parts.iter().enumerate() {
        let context = format!("reverse_part_outline.parts[{index}]");
        reject_unknown_keys(
            item,
            &[
                "part_number",
                "title",
                "start_chapter",
                "end_chapter",
                "summary",
                "theme",
                "key_events",
                "character_focus",
            ],
            &context,
        )?;
        let part_number = required_positive_i32(item, "part_number", &context)?;
        if !seen.insert(part_number) {
            return Err(format!(
                "reverse_part_outline.parts 包含重复分部: {part_number}"
            ));
        }
        let start_chapter = required_positive_i32(item, "start_chapter", &context)?;
        let end_chapter = required_positive_i32(item, "end_chapter", &context)?;
        if end_chapter < start_chapter {
            return Err(format!("{context}.end_chapter 不能小于 start_chapter"));
        }
        ranges.push((start_chapter, end_chapter, part_number));
        required_string(item, "title", &context)?;
        required_string(item, "summary", &context)?;
        required_string(item, "theme", &context)?;
        required_array_clone(item, "key_events", &context)?;
        required_object_clone(item, "character_focus", &context)?;
    }
    validate_part_chapter_ranges(&mut ranges)?;
    Ok(())
}

fn validate_part_chapter_ranges(ranges: &mut [(i32, i32, i32)]) -> Result<(), String> {
    ranges.sort_by_key(|(start_chapter, end_chapter, part_number)| {
        (*start_chapter, *end_chapter, *part_number)
    });
    for pair in ranges.windows(2) {
        let (prev_start, prev_end, prev_part) = pair[0];
        let (start, end, part) = pair[1];
        if start <= prev_end {
            return Err(format!(
                "reverse_part_outline.parts 章节范围重叠: part {prev_part}({prev_start}-{prev_end}) 与 part {part}({start}-{end})"
            ));
        }
        let expected_start = prev_end.saturating_add(1);
        if start > expected_start {
            return Err(format!(
                "reverse_part_outline.parts 章节范围有缺口: 第 {expected_start} 章到第 {} 章未覆盖",
                start - 1
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::{Value, json};

    fn part(part_number: i32, start_chapter: i32, end_chapter: i32) -> Value {
        json!({
            "part_number": part_number,
            "title": format!("第{part_number}部"),
            "start_chapter": start_chapter,
            "end_chapter": end_chapter,
            "summary": "分部摘要",
            "theme": "代价",
            "key_events": ["旧敌回归", "盟友背叛", "真相浮出"],
            "character_focus": {"主角": "从迟疑转为主动"}
        })
    }

    #[test]
    fn validate_reverse_part_outline_items_rejects_overlapping_ranges() {
        let parts = vec![part(1, 1, 10), part(2, 10, 20)];

        let err = validate_reverse_part_outline_items(&parts)
            .expect_err("overlapping chapter ranges must fail");

        assert!(err.contains("章节范围重叠"));
        assert!(err.contains("part 1(1-10)"));
        assert!(err.contains("part 2(10-20)"));
    }

    #[test]
    fn validate_reverse_part_outline_items_rejects_gapped_ranges() {
        let parts = vec![part(1, 1, 10), part(2, 12, 20)];

        let err = validate_reverse_part_outline_items(&parts)
            .expect_err("gapped chapter ranges must fail");

        assert!(err.contains("章节范围有缺口"));
        assert!(err.contains("第 11 章到第 11 章未覆盖"));
    }

    #[test]
    fn validate_reverse_part_outline_items_accepts_any_key_events_array() {
        let mut item = part(1, 1, 10);
        item["key_events"] = json!(["唯一事件"]);
        let parts = vec![item];

        validate_reverse_part_outline_items(&parts)
            .expect("apply contract only requires key_events to be an array");
    }

    #[test]
    fn validate_reverse_outline_items_rejects_unknown_fields() {
        let outlines = vec![json!({
            "chapter_number": 1,
            "title": "第一章",
            "summary": "章节摘要",
            "legacy_notes": "旧字段"
        })];

        let err = validate_reverse_outline_items(&outlines)
            .expect_err("unknown outline fields must fail");

        assert!(err.contains("未知字段 legacy_notes"));
    }

    #[test]
    fn validate_reverse_outline_items_rejects_general_outline_fields() {
        let outlines = vec![json!({
            "chapter_number": 1,
            "title": "第一章",
            "summary": "章节摘要",
            "beats": ["旧节拍"]
        })];

        let err = validate_reverse_outline_items(&outlines)
            .expect_err("reverse outline must not accept general outline fields");

        assert!(err.contains("未知字段 beats"));
    }

    #[test]
    fn validate_reverse_part_outline_items_rejects_unknown_fields() {
        let mut item = part(1, 1, 10);
        item["total_chapters"] = json!(10);
        let parts = vec![item];

        let err = validate_reverse_part_outline_items(&parts)
            .expect_err("unknown part outline fields must fail");

        assert!(err.contains("未知字段 total_chapters"));
    }
}
