use super::super::{CharacterAiProgress, character_discovery::parse_character_discovery_output};
use crate::actions::character_ai::json_utils::required_i32;
use a4n_core::models::{chapter::chapter, project::text_anchor};
use serde_json::Value;
use std::collections::BTreeSet;

pub(in crate::actions::character_ai) fn select_character_discovery_chapters(
    chapters: Vec<chapter::Model>,
    project_id: i64,
    chapter_number: Option<i32>,
) -> Result<Vec<chapter::Model>, String> {
    if chapter_number.is_some_and(|chapter_number| chapter_number <= 0) {
        return Err("角色发现章节号必须大于 0".to_string());
    }
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

pub(in crate::actions::character_ai) fn validate_discovered_character_chapter_scope(
    characters: &[Value],
    analyzed_chapters: &BTreeSet<i32>,
) -> Result<(), String> {
    if analyzed_chapters.is_empty() {
        return Err("角色发现缺少已分析章节".to_string());
    }

    for (index, character) in characters.iter().enumerate() {
        let first_seen = required_i32(character, "first_seen_chapter")?;
        let latest_seen = required_i32(character, "latest_seen_chapter")?;
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

pub(super) fn parse_and_validate_character_discovery_content(
    content: &str,
    analyzed_chapters: &BTreeSet<i32>,
    chapters: &[chapter::Model],
    text_anchors: &[text_anchor::Model],
) -> Result<Vec<Value>, String> {
    let discovered = parse_character_discovery_output(content, chapters, text_anchors)?;
    validate_discovered_character_chapter_scope(&discovered, analyzed_chapters)?;
    Ok(discovered)
}

pub(super) fn report_character_discovery_progress(
    progress: Option<&CharacterAiProgress<'_>>,
    current: u32,
    total: u32,
    message: &str,
    cancelled_message: &str,
) -> Result<(), String> {
    if let Some(progress) = progress {
        progress.report(current, total, message, cancelled_message)?;
    }
    Ok(())
}
