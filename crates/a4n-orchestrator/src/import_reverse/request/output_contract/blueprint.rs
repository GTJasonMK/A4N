//! 蓝图反推输出合同。

use serde_json::{Value, json};

pub(super) fn reverse_blueprint_output_contract() -> Value {
    json!({
        "format": "json_object",
        "required_top_level_fields": [
            "title",
            "genre",
            "style",
            "tone",
            "target_audience",
            "one_sentence_summary",
            "full_synopsis",
            "world_setting",
            "characters",
            "relationships"
        ],
        "optional_top_level_fields": [
            "total_chapters",
            "needs_part_outlines"
        ],
        "consistency_rules": {
            "total_chapters": "如输出，必须为正整数；已有蓝图总章节数时必须一致",
            "needs_part_outlines": "如输出，必须是布尔值，且必须与有效总章节数推导结果一致"
        },
        "system_derived_or_downstream_fields_do_not_output": [
            "chapters_per_part",
            "chapter_outline",
            "chapter_outlines",
            "part_outlines"
        ],
        "length_limits": {
            "one_sentence_summary": "30 个中文字符以内",
            "full_synopsis": "300-500 个中文字符，保留起承转合但不要展开到章节级",
            "world_setting_text_fields": "每项 20-80 个中文字符",
            "character_text_fields": "每项 10-60 个中文字符",
            "relationship_description": "20-80 个中文字符"
        },
        "count_limits": {
            "characters": "3-8 个主要角色；缺少主要角色时宁缺毋滥",
            "relationships": "0-12 条高价值关系",
            "world_setting.key_locations": "0-6 条",
            "world_setting.factions": "0-6 条",
            "world_setting.history_timeline": "0-6 条",
            "world_setting.conflict_sources": "1-6 条",
            "world_setting.mysteries_and_foreshadowing": "0-6 条"
        }
    })
}
