//! 大纲反推输出合同。

use serde_json::{Value, json};

pub(super) fn reverse_outline_output_contract() -> Value {
    json!({
        "format": "json_object",
        "required_root_field": "outlines",
        "forbidden_root_fields": ["chapters", "chapter_outlines", "items"],
        "item_fields": ["chapter_number", "title", "summary"],
        "optional_item_fields": ["word_count_target"],
        "numeric_rules": {
            "word_count_target": "如输出，必须为正整数"
        },
        "system_derived_fields_do_not_output": ["id", "project_id", "outline_id", "created_at", "updated_at"],
        "length_limits": {
            "title": "15 个中文字符以内",
            "summary": "50-100 个中文字符"
        },
        "count_rule": "outlines 数量必须等于输入 source_context.summaries 数量；不要合并或新增章节"
    })
}

pub(super) fn reverse_part_outline_output_contract() -> Value {
    json!({
        "format": "json_object",
        "required_root_field": "parts",
        "forbidden_root_fields": ["part_outlines", "outlines", "chapters"],
        "item_fields": [
            "part_number",
            "title",
            "start_chapter",
            "end_chapter",
            "summary",
            "theme",
            "key_events",
            "character_focus"
        ],
        "system_derived_fields_do_not_output": ["id", "project_id", "created_at", "updated_at"],
        "length_limits": {
            "title": "20 个中文字符以内",
            "summary": "100-160 个中文字符",
            "theme": "30 个中文字符以内",
            "key_events_each": "20-60 个中文字符",
            "character_focus_each": "20-80 个中文字符"
        },
        "count_limits": {
            "parts": "按剧情自然阶段划分，通常 2-6 部",
            "key_events": "每部分 3-5 条"
        },
        "range_rule": "章节范围必须连续、无重叠、无遗漏"
    })
}
