//! 角色发现章节选择与输出范围校验纯规则（无 DB）。

use a4n_core::models::chapter::chapter;
use serde_json::Value;
use std::collections::BTreeSet;

fn required_i32_field(value: &Value, field: &str) -> Result<i32, String> {
    value
        .get(field)
        .and_then(Value::as_i64)
        .and_then(|n| i32::try_from(n).ok())
        .ok_or_else(|| format!("{field} 必须是整数"))
}

/// 按可选章节号筛选可用于角色发现的章节。
pub fn select_character_discovery_chapters(
    chapters: Vec<chapter::Model>,
    project_id: i64,
    chapter_number: Option<i32>,
) -> Result<Vec<chapter::Model>, String> {
    if let Some(chapter_number) = chapter_number {
        let selected = chapters
            .into_iter()
            .filter(|chapter| chapter.chapter_number == chapter_number)
            .collect::<Vec<_>>();
        if selected.is_empty() {
            return Err(format!(
                "第 {chapter_number} 章没有可用于角色发现的有效正文，请先生成或导入正文"
            ));
        }
        return Ok(selected);
    }

    if chapters.is_empty() {
        return Err(format!(
            "项目还没有可用于角色发现的章节: project_id={project_id}"
        ));
    }
    Ok(chapters)
}

/// 校验发现结果中的 first/latest_seen 落在已分析章节集合内。
pub fn validate_discovered_character_chapter_scope(
    characters: &[Value],
    analyzed_chapters: &BTreeSet<i32>,
) -> Result<(), String> {
    if analyzed_chapters.is_empty() {
        return Err("角色发现缺少已分析章节".to_string());
    }

    for (index, character) in characters.iter().enumerate() {
        let first_seen = required_i32_field(character, "first_seen_chapter")?;
        let latest_seen = required_i32_field(character, "latest_seen_chapter")?;
        if !analyzed_chapters.contains(&first_seen) {
            return Err(format!(
                "characters[{index}].first_seen_chapter 第 {first_seen} 章不在本次角色发现输入章节中"
            ));
        }
        if !analyzed_chapters.contains(&latest_seen) {
            return Err(format!(
                "characters[{index}].latest_seen_chapter 第 {latest_seen} 章不在本次角色发现输入章节中"
            ));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn select_all_rejects_empty_project() {
        let err = select_character_discovery_chapters(vec![], 9, None).unwrap_err();
        assert!(err.contains("project_id=9"));
    }

    #[test]
    fn validate_scope_rejects_out_of_range_chapter() {
        let mut set = BTreeSet::new();
        set.insert(1);
        let err = validate_discovered_character_chapter_scope(
            &[json!({"first_seen_chapter": 2, "latest_seen_chapter": 2})],
            &set,
        )
        .unwrap_err();
        assert!(err.contains("first_seen_chapter"));
    }
}
