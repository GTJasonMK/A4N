use super::validation::{
    optional_json_positive_i32_field, optional_json_string, required_json_array,
    required_json_nonnegative_i32_field, required_json_object, required_json_positive_i32_field,
    required_json_string, required_json_text, required_text,
};
use a4n_core::models::chapter::{chapter, chapter_outline};
use serde_json::Value;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ImportPayload {
    pub(super) chapters: Vec<ImportChapter>,
    pub(super) blueprint: Option<ImportBlueprint>,
    pub(super) blueprint_versions: Vec<ImportBlueprintVersion>,
    pub(super) characters: Vec<ImportCharacter>,
    pub(super) relationships: Vec<ImportRelationship>,
    pub(super) part_outlines: Vec<ImportPartOutline>,
    pub(super) chapter_outlines: Vec<ImportChapterOutline>,
    pub(super) has_preserved_chapter_tags: bool,
}

impl ImportPayload {
    pub(super) fn chapter_bound_counts(&self) -> Result<(i32, i32, i32), String> {
        let mut max_chapter_number = 0;
        let mut max_available_content_chapter_number = 0;
        let mut available_content_chapter_count = 0_i32;
        for chapter in &self.chapters {
            max_chapter_number = max_chapter_number.max(chapter.chapter_number);
            if chapter.status != chapter::STATUS_DELETED && !chapter.content.trim().is_empty() {
                max_available_content_chapter_number =
                    max_available_content_chapter_number.max(chapter.chapter_number);
                available_content_chapter_count += 1;
            }
        }
        Ok((
            max_chapter_number,
            max_available_content_chapter_number,
            available_content_chapter_count,
        ))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ImportChapter {
    pub(super) old_outline_id: Option<i64>,
    pub(super) chapter_number: i32,
    pub(super) title: Option<String>,
    pub(super) content: String,
    pub(super) word_count: i32,
    pub(super) status: String,
    pub(super) summary: Option<String>,
    pub(super) style_tags: Option<Value>,
    pub(super) analysis_data: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ImportBlueprint {
    pub(super) title: String,
    pub(super) genre: String,
    pub(super) style: Option<String>,
    pub(super) tone: Option<String>,
    pub(super) target_audience: Option<String>,
    pub(super) one_sentence_summary: Option<String>,
    pub(super) full_synopsis: Option<String>,
    pub(super) world_setting: Option<Value>,
    pub(super) current_version: String,
    pub(super) needs_part_outlines: bool,
    pub(super) total_chapters: Option<i32>,
    pub(super) chapters_per_part: Option<i32>,
    pub(super) avatar_svg: Option<String>,
    pub(super) avatar_animal: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ImportBlueprintVersion {
    pub(super) version: String,
    pub(super) change_description: Option<String>,
    pub(super) blueprint_data: Value,
    pub(super) world_setting_snapshot: Option<Value>,
    pub(super) characters_snapshot: Option<Value>,
    pub(super) relationships_snapshot: Option<Value>,
    pub(super) synopsis_snapshot: Option<String>,
    pub(super) created_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ImportCharacter {
    pub(super) name: String,
    pub(super) character_type: String,
    pub(super) gender: Option<String>,
    pub(super) age: Option<String>,
    pub(super) appearance: Option<String>,
    pub(super) personality: Option<String>,
    pub(super) background: Option<String>,
    pub(super) motivation: Option<String>,
    pub(super) arc_summary: Option<String>,
    pub(super) relationship_to_protagonist: Option<String>,
    pub(super) arc_stages: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ImportRelationship {
    pub(super) relationship_type: String,
    pub(super) character_a: String,
    pub(super) character_b: String,
    pub(super) description: Option<String>,
    pub(super) strength: Option<i32>,
    pub(super) evolution: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ImportPartOutline {
    pub(super) old_id: Option<i64>,
    pub(super) part_number: i32,
    pub(super) title: String,
    pub(super) start_chapter: i32,
    pub(super) end_chapter: i32,
    pub(super) summary: Option<String>,
    pub(super) theme: Option<String>,
    pub(super) key_events: Option<Value>,
    pub(super) character_focus: Option<Value>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ImportChapterOutline {
    pub(super) old_id: Option<i64>,
    pub(super) old_part_id: Option<i64>,
    pub(super) chapter_number: i32,
    pub(super) title: Option<String>,
    pub(super) summary: Option<String>,
    pub(super) beats: Option<Value>,
    pub(super) key_scenes: Option<Value>,
    pub(super) pacing_score: Option<f32>,
    pub(super) foreshadowing_setup: Option<Value>,
    pub(super) foreshadowing_payoff: Option<Value>,
    pub(super) characters_involved: Option<Value>,
    pub(super) word_count_target: Option<i32>,
    pub(super) status: String,
}

pub(super) fn parse_import_payload(root: &Value) -> Result<ImportPayload, String> {
    let chapters = parse_import_chapters(root)?;
    let part_outlines = parse_import_part_outlines(root)?;
    let chapter_outlines = parse_import_chapter_outlines(root)?;
    validate_import_chapter_outline_part_references(&chapter_outlines, &part_outlines)?;
    validate_import_chapter_outline_references(&chapters, &chapter_outlines)?;
    Ok(ImportPayload {
        has_preserved_chapter_tags: chapters.iter().any(|chapter| chapter.style_tags.is_some()),
        chapters,
        blueprint: parse_import_blueprint(root)?,
        blueprint_versions: parse_import_blueprint_versions(root)?,
        characters: parse_import_characters(root)?,
        relationships: parse_import_relationships(root)?,
        part_outlines,
        chapter_outlines,
    })
}

fn validate_import_chapter_outline_references(
    chapters: &[ImportChapter],
    chapter_outlines: &[ImportChapterOutline],
) -> Result<(), String> {
    let outline_chapters = chapter_outlines
        .iter()
        .filter_map(|outline| {
            outline
                .old_id
                .map(|old_id| (old_id, outline.chapter_number))
        })
        .collect::<HashMap<_, _>>();

    for chapter in chapters {
        let chapter_number = chapter.chapter_number;
        let label = format!("第{chapter_number}章");
        let Some(outline_id) = chapter.old_outline_id else {
            continue;
        };
        let Some(outline_chapter_number) = outline_chapters.get(&outline_id).copied() else {
            return Err(format!(
                "{label}引用的章节大纲不存在: outline_id={outline_id}"
            ));
        };
        if outline_chapter_number != chapter_number {
            return Err(format!(
                "{label}引用的章节大纲属于第{outline_chapter_number}章: outline_id={outline_id}"
            ));
        }
    }
    Ok(())
}

fn parse_import_chapters(root: &Value) -> Result<Vec<ImportChapter>, String> {
    let mut seen_chapter_numbers = HashSet::new();
    required_json_array(root, "chapters", "导入文件")?
        .iter()
        .map(|chapter_value| {
            let chapter_number =
                required_json_positive_i32_field(chapter_value, "chapter_number", "导入章节")?;
            if !seen_chapter_numbers.insert(chapter_number) {
                return Err(format!("导入文件包含重复章节序号: 第{chapter_number}章"));
            }
            let label = format!("第{chapter_number}章");
            let content = required_json_string(chapter_value, "content", &label)?.to_string();
            let word_count =
                required_json_nonnegative_i32_field(chapter_value, "word_count", &label)?;
            let actual_word_count = content.chars().count();
            if usize::try_from(word_count)
                .map_err(|_| format!("{label}字数不能为负数: word_count={word_count}"))?
                != actual_word_count
            {
                return Err(format!(
                    "{label}字数与正文字符数不一致: word_count={word_count}, actual={actual_word_count}"
                ));
            }
            Ok(ImportChapter {
                old_outline_id: optional_json_positive_i64_field(
                    chapter_value,
                    "outline_id",
                    &label,
                )?,
                chapter_number,
                title: optional_json_string(chapter_value, "title", &label)?
                    .map(ToString::to_string),
                content,
                word_count,
                status: import_chapter_status(chapter_value, &label)?,
                summary: optional_json_string(chapter_value, "summary", &label)?
                    .map(ToString::to_string),
                style_tags: chapter_value.get("style_tags").cloned(),
                analysis_data: chapter_value.get("analysis_data").cloned(),
            })
        })
        .collect()
}

fn validate_import_chapter_outline_part_references(
    chapter_outlines: &[ImportChapterOutline],
    part_outlines: &[ImportPartOutline],
) -> Result<(), String> {
    let part_ranges = part_outlines
        .iter()
        .filter_map(|part| {
            part.old_id.map(|old_id| {
                (
                    old_id,
                    (part.part_number, part.start_chapter, part.end_chapter),
                )
            })
        })
        .collect::<HashMap<_, _>>();

    for outline in chapter_outlines {
        let Some(part_id) = outline.old_part_id else {
            continue;
        };
        let Some((part_number, start_chapter, end_chapter)) = part_ranges.get(&part_id).copied()
        else {
            return Err(format!(
                "第{}章大纲引用的分部大纲不存在: part_id={part_id}",
                outline.chapter_number
            ));
        };
        if outline.chapter_number < start_chapter || outline.chapter_number > end_chapter {
            return Err(format!(
                "第{}章大纲引用的第{part_number}部分部范围不包含该章节: part_id={part_id}, range={start_chapter}-{end_chapter}",
                outline.chapter_number
            ));
        }
    }
    Ok(())
}

fn parse_import_blueprint(root: &Value) -> Result<Option<ImportBlueprint>, String> {
    let Some(value) = optional_top_object(root, "blueprint", "导入文件")? else {
        return Ok(None);
    };
    Ok(Some(ImportBlueprint {
        title: required_json_text(value, "title", "导入蓝图标题")?.to_string(),
        genre: required_json_text(value, "genre", "导入蓝图题材")?.to_string(),
        style: optional_json_string(value, "style", "导入蓝图风格")?.map(ToString::to_string),
        tone: optional_json_string(value, "tone", "导入蓝图基调")?.map(ToString::to_string),
        target_audience: optional_json_string(value, "target_audience", "导入蓝图受众")?
            .map(ToString::to_string),
        one_sentence_summary: optional_json_string(
            value,
            "one_sentence_summary",
            "导入蓝图一句话概要",
        )?
        .map(ToString::to_string),
        full_synopsis: optional_json_string(value, "full_synopsis", "导入蓝图完整梗概")?
            .map(ToString::to_string),
        world_setting: optional_json_object_clone(value, "world_setting", "导入蓝图世界观")?,
        current_version: required_json_text(value, "current_version", "导入蓝图版本")?.to_string(),
        needs_part_outlines: required_json_bool(value, "needs_part_outlines", "导入蓝图")?,
        total_chapters: optional_json_positive_i32_field(
            value,
            "total_chapters",
            "导入蓝图总章节数",
        )?,
        chapters_per_part: optional_json_positive_i32_field(
            value,
            "chapters_per_part",
            "导入蓝图分部章节数",
        )?,
        avatar_svg: optional_json_string(value, "avatar_svg", "导入蓝图头像 SVG")?
            .map(ToString::to_string),
        avatar_animal: optional_json_string(value, "avatar_animal", "导入蓝图头像")?
            .map(ToString::to_string),
    }))
}

fn parse_import_blueprint_versions(root: &Value) -> Result<Vec<ImportBlueprintVersion>, String> {
    optional_top_array(root, "blueprint_versions", "导入文件")?
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let label = format!("导入蓝图版本{}", index + 1);
            let value = value
                .as_object()
                .map(|_| value)
                .ok_or_else(|| format!("{label}必须是对象"))?;
            Ok(ImportBlueprintVersion {
                version: required_json_text(value, "version", &label)?.to_string(),
                change_description: optional_json_string(value, "change_description", &label)?
                    .map(ToString::to_string),
                blueprint_data: required_json_object(value, "blueprint_data", &label)?.clone(),
                world_setting_snapshot: optional_json_value(value, "world_setting_snapshot"),
                characters_snapshot: optional_json_value(value, "characters_snapshot"),
                relationships_snapshot: optional_json_value(value, "relationships_snapshot"),
                synopsis_snapshot: optional_json_string(value, "synopsis_snapshot", &label)?
                    .map(ToString::to_string),
                created_by: optional_json_string(value, "created_by", &label)?
                    .map(ToString::to_string),
            })
        })
        .collect()
}

fn parse_import_characters(root: &Value) -> Result<Vec<ImportCharacter>, String> {
    optional_top_array(root, "characters", "导入文件")?
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let label = format!("导入角色{}", index + 1);
            let value = value
                .as_object()
                .map(|_| value)
                .ok_or_else(|| format!("{label}必须是对象"))?;
            Ok(ImportCharacter {
                name: required_json_text(value, "name", &label)?.to_string(),
                character_type: required_json_text(value, "character_type", &label)?.to_string(),
                gender: optional_json_string(value, "gender", &label)?.map(ToString::to_string),
                age: optional_json_string(value, "age", &label)?.map(ToString::to_string),
                appearance: optional_json_string(value, "appearance", &label)?
                    .map(ToString::to_string),
                personality: optional_json_string(value, "personality", &label)?
                    .map(ToString::to_string),
                background: optional_json_string(value, "background", &label)?
                    .map(ToString::to_string),
                motivation: optional_json_string(value, "motivation", &label)?
                    .map(ToString::to_string),
                arc_summary: optional_json_string(value, "arc_summary", &label)?
                    .map(ToString::to_string),
                relationship_to_protagonist: optional_json_string(
                    value,
                    "relationship_to_protagonist",
                    &label,
                )?
                .map(ToString::to_string),
                arc_stages: optional_json_value(value, "arc_stages"),
            })
        })
        .collect()
}

fn parse_import_relationships(root: &Value) -> Result<Vec<ImportRelationship>, String> {
    optional_top_array(root, "relationships", "导入文件")?
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let label = format!("导入角色关系{}", index + 1);
            let value = value
                .as_object()
                .map(|_| value)
                .ok_or_else(|| format!("{label}必须是对象"))?;
            Ok(ImportRelationship {
                relationship_type: required_json_text(value, "relationship_type", &label)?
                    .to_string(),
                character_a: required_json_text(value, "character_a", &label)?.to_string(),
                character_b: required_json_text(value, "character_b", &label)?.to_string(),
                description: optional_json_string(value, "description", &label)?
                    .map(ToString::to_string),
                strength: optional_json_positive_i32_field(value, "strength", &label)?,
                evolution: optional_json_value(value, "evolution"),
            })
        })
        .collect()
}

fn parse_import_part_outlines(root: &Value) -> Result<Vec<ImportPartOutline>, String> {
    let mut seen_numbers = HashSet::new();
    let mut seen_old_ids = HashSet::new();
    let parts = optional_top_array(root, "part_outlines", "导入文件")?
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let label = format!("导入分部大纲{}", index + 1);
            let value = value
                .as_object()
                .map(|_| value)
                .ok_or_else(|| format!("{label}必须是对象"))?;
            let old_id = optional_json_positive_i64_field(value, "id", &label)?;
            if let Some(old_id) = old_id
                && !seen_old_ids.insert(old_id)
            {
                return Err(format!("{label}旧 ID 重复: {old_id}"));
            }
            let part_number = required_json_positive_i32_field(value, "part_number", &label)?;
            if !seen_numbers.insert(part_number) {
                return Err(format!("导入文件包含重复分部序号: 第{part_number}部"));
            }
            let start_chapter = required_json_positive_i32_field(value, "start_chapter", &label)?;
            let end_chapter = required_json_positive_i32_field(value, "end_chapter", &label)?;
            if end_chapter < start_chapter {
                return Err(format!(
                    "{label}结束章节不能小于开始章节: start={start_chapter}, end={end_chapter}"
                ));
            }
            Ok(ImportPartOutline {
                old_id,
                part_number,
                title: required_json_text(value, "title", &label)?.to_string(),
                start_chapter,
                end_chapter,
                summary: optional_json_string(value, "summary", &label)?.map(ToString::to_string),
                theme: optional_json_string(value, "theme", &label)?.map(ToString::to_string),
                key_events: optional_json_value(value, "key_events"),
                character_focus: optional_json_value(value, "character_focus"),
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    validate_import_part_outline_ranges(&parts)?;
    Ok(parts)
}

fn validate_import_part_outline_ranges(parts: &[ImportPartOutline]) -> Result<(), String> {
    let mut ranges = parts
        .iter()
        .map(|part| (part.start_chapter, part.end_chapter, part.part_number))
        .collect::<Vec<_>>();
    ranges.sort_by_key(|(start_chapter, end_chapter, part_number)| {
        (*start_chapter, *end_chapter, *part_number)
    });
    for pair in ranges.windows(2) {
        let (prev_start, prev_end, prev_part) = pair[0];
        let (start, end, part) = pair[1];
        if start <= prev_end {
            return Err(format!(
                "导入文件分部章节范围重叠: 第{prev_part}部({prev_start}-{prev_end}) 与第{part}部({start}-{end})"
            ));
        }
    }
    Ok(())
}

fn parse_import_chapter_outlines(root: &Value) -> Result<Vec<ImportChapterOutline>, String> {
    let mut seen_numbers = HashSet::new();
    let mut seen_old_ids = HashSet::new();
    optional_top_array(root, "chapter_outlines", "导入文件")?
        .iter()
        .enumerate()
        .map(|(index, value)| {
            let label = format!("导入章节大纲{}", index + 1);
            let value = value
                .as_object()
                .map(|_| value)
                .ok_or_else(|| format!("{label}必须是对象"))?;
            let old_id = optional_json_positive_i64_field(value, "id", &label)?;
            if let Some(old_id) = old_id
                && !seen_old_ids.insert(old_id)
            {
                return Err(format!("{label}旧 ID 重复: {old_id}"));
            }
            let chapter_number = required_json_positive_i32_field(value, "chapter_number", &label)?;
            if !seen_numbers.insert(chapter_number) {
                return Err(format!(
                    "导入文件包含重复章节大纲序号: 第{chapter_number}章"
                ));
            }
            Ok(ImportChapterOutline {
                old_id,
                old_part_id: optional_json_positive_i64_field(value, "part_id", &label)?,
                chapter_number,
                title: optional_json_string(value, "title", &label)?.map(ToString::to_string),
                summary: optional_json_string(value, "summary", &label)?.map(ToString::to_string),
                beats: optional_json_value(value, "beats"),
                key_scenes: optional_json_value(value, "key_scenes"),
                pacing_score: optional_json_f32_field(value, "pacing_score", &label)?,
                foreshadowing_setup: optional_json_value(value, "foreshadowing_setup"),
                foreshadowing_payoff: optional_json_value(value, "foreshadowing_payoff"),
                characters_involved: optional_json_value(value, "characters_involved"),
                word_count_target: optional_json_positive_i32_field(
                    value,
                    "word_count_target",
                    &label,
                )?,
                status: import_chapter_outline_status(value, &label)?,
            })
        })
        .collect()
}

fn optional_top_object<'a>(
    value: &'a Value,
    field: &str,
    label: &str,
) -> Result<Option<&'a Value>, String> {
    match value.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(field_value) if field_value.is_object() => Ok(Some(field_value)),
        Some(_) => Err(format!("{label}字段 {field} 必须是对象或 null")),
    }
}

fn optional_top_array<'a>(
    value: &'a Value,
    field: &str,
    label: &str,
) -> Result<&'a [Value], String> {
    match value.get(field) {
        None | Some(Value::Null) => Ok(&[]),
        Some(field_value) => field_value
            .as_array()
            .map(Vec::as_slice)
            .ok_or_else(|| format!("{label}字段 {field} 必须是数组或 null")),
    }
}

fn optional_json_value(value: &Value, field: &str) -> Option<Value> {
    match value.get(field) {
        None | Some(Value::Null) => None,
        Some(field_value) => Some(field_value.clone()),
    }
}

fn optional_json_object_clone(
    value: &Value,
    field: &str,
    label: &str,
) -> Result<Option<Value>, String> {
    match value.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(field_value) if field_value.is_object() => Ok(Some(field_value.clone())),
        Some(_) => Err(format!("{label}字段 {field} 必须是对象或 null")),
    }
}

fn required_json_bool(value: &Value, field: &str, label: &str) -> Result<bool, String> {
    value
        .get(field)
        .ok_or_else(|| format!("{label}缺少字段: {field}"))?
        .as_bool()
        .ok_or_else(|| format!("{label}字段 {field} 必须是布尔值"))
}

pub(super) fn optional_json_positive_i64_field(
    value: &Value,
    field: &str,
    label: &str,
) -> Result<Option<i64>, String> {
    match value.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(field_value) => {
            let number = field_value
                .as_i64()
                .ok_or_else(|| format!("{label}字段 {field} 必须是整数"))?;
            if number > 0 {
                Ok(Some(number))
            } else {
                Err(format!("{label}字段 {field} 必须为正整数"))
            }
        }
    }
}

fn optional_json_f32_field(value: &Value, field: &str, label: &str) -> Result<Option<f32>, String> {
    match value.get(field) {
        None | Some(Value::Null) => Ok(None),
        Some(field_value) => {
            let number = field_value
                .as_f64()
                .ok_or_else(|| format!("{label}字段 {field} 必须是数字"))?;
            if !number.is_finite() || number < f32::MIN as f64 || number > f32::MAX as f64 {
                return Err(format!("{label}字段 {field} 超出 f32 范围"));
            }
            Ok(Some(number as f32))
        }
    }
}

pub(super) fn import_chapter_status(value: &Value, label: &str) -> Result<String, String> {
    let status = optional_json_string(value, "status", label)?
        .map(|status| required_text(status, &format!("{label}状态")))
        .transpose()?
        .unwrap_or(chapter::STATUS_IMPORTED)
        .to_string();
    if !chapter::is_valid_status(&status) {
        return Err(format!("{label}状态无效: {status}"));
    }
    if status == chapter::STATUS_GENERATING {
        return Err(format!(
            "{label}状态不能是生成中的瞬态状态: {}",
            chapter::STATUS_GENERATING
        ));
    }
    Ok(status)
}

fn import_chapter_outline_status(value: &Value, label: &str) -> Result<String, String> {
    let status = optional_json_string(value, "status", label)?
        .map(|status| required_text(status, &format!("{label}状态")))
        .transpose()?
        .unwrap_or(chapter_outline::STATUS_DRAFT)
        .to_string();
    if !chapter_outline::is_valid_status(&status) {
        return Err(format!("{label}状态无效: {status}"));
    }
    Ok(status)
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
        })
    }

    fn outline(id: i64, chapter_number: i32) -> Value {
        json!({
            "id": id,
            "chapter_number": chapter_number,
        })
    }

    fn chapter_with_outline(chapter_number: i32, outline_id: i64) -> Value {
        json!({
            "chapter_number": chapter_number,
            "outline_id": outline_id,
            "content": "旧城A",
            "word_count": 3,
        })
    }

    fn part_with_id(id: i64, part_number: i32, start_chapter: i32, end_chapter: i32) -> Value {
        let mut value = part(part_number, start_chapter, end_chapter);
        value["id"] = json!(id);
        value
    }

    #[test]
    fn parse_import_payload_rejects_overlapping_part_ranges() {
        let payload = json!({
            "chapters": [],
            "part_outlines": [
                part(1, 1, 10),
                part(2, 10, 20)
            ]
        });

        let err =
            parse_import_payload(&payload).expect_err("overlapping part ranges must be rejected");

        assert!(err.contains("分部章节范围重叠"));
        assert!(err.contains("第1部(1-10)"));
        assert!(err.contains("第2部(10-20)"));
    }

    #[test]
    fn parse_import_payload_accepts_adjacent_part_ranges() {
        let payload = json!({
            "chapters": [],
            "part_outlines": [
                part(1, 1, 10),
                part(2, 11, 20)
            ]
        });

        let parsed = parse_import_payload(&payload).expect("adjacent part ranges should be valid");

        assert_eq!(parsed.part_outlines.len(), 2);
    }

    #[test]
    fn parse_import_payload_rejects_chapter_outline_reference_for_other_chapter() {
        let payload = json!({
            "chapters": [
                chapter_with_outline(1, 102)
            ],
            "chapter_outlines": [
                outline(102, 2)
            ]
        });

        let err = parse_import_payload(&payload)
            .expect_err("chapter content must not point at another chapter outline");

        assert!(err.contains("第1章引用的章节大纲属于第2章"));
        assert!(err.contains("outline_id=102"));
    }

    #[test]
    fn parse_import_payload_accepts_same_chapter_outline_reference() {
        let payload = json!({
            "chapters": [
                chapter_with_outline(1, 101)
            ],
            "chapter_outlines": [
                outline(101, 1)
            ]
        });

        let parsed =
            parse_import_payload(&payload).expect("same-chapter outline reference should import");

        assert_eq!(parsed.chapter_outlines.len(), 1);
    }

    #[test]
    fn parse_import_payload_rejects_chapter_outline_part_outside_range() {
        let payload = json!({
            "chapters": [],
            "part_outlines": [
                part_with_id(101, 1, 1, 10)
            ],
            "chapter_outlines": [
                { "id": 201, "part_id": 101, "chapter_number": 11 }
            ]
        });

        let err = parse_import_payload(&payload)
            .expect_err("chapter outline part_id must contain the chapter number");

        assert!(err.contains("第11章大纲引用的第1部分部范围不包含该章节"));
        assert!(err.contains("part_id=101"));
        assert!(err.contains("range=1-10"));
    }

    #[test]
    fn parse_import_payload_accepts_chapter_outline_part_inside_range() {
        let payload = json!({
            "chapters": [],
            "part_outlines": [
                part_with_id(101, 1, 1, 10)
            ],
            "chapter_outlines": [
                { "id": 201, "part_id": 101, "chapter_number": 10 }
            ]
        });

        let parsed = parse_import_payload(&payload)
            .expect("chapter outline may reference a part that contains the chapter");

        assert_eq!(parsed.chapter_outlines.len(), 1);
    }

    #[test]
    fn parse_import_payload_rejects_unknown_chapter_outline_status() {
        let payload = json!({
            "chapters": [],
            "chapter_outlines": [
                { "id": 201, "chapter_number": 1, "status": "bogus" }
            ]
        });

        let err = parse_import_payload(&payload)
            .expect_err("chapter outline status must use known persistent states");

        assert!(err.contains("导入章节大纲1状态无效: bogus"));
    }

    #[test]
    fn parse_import_payload_accepts_known_chapter_outline_statuses() {
        for status in [
            chapter_outline::STATUS_DRAFT,
            chapter_outline::STATUS_COMPLETED,
        ] {
            let payload = json!({
                "chapters": [],
                "chapter_outlines": [
                    { "id": 201, "chapter_number": 1, "status": status }
                ]
            });

            let parsed =
                parse_import_payload(&payload).expect("known chapter outline status should import");

            assert_eq!(parsed.chapter_outlines[0].status, status);
        }
    }

    #[test]
    fn parse_import_payload_defaults_missing_chapter_outline_status_to_draft() {
        let payload = json!({
            "chapters": [],
            "chapter_outlines": [
                { "id": 201, "chapter_number": 1 }
            ]
        });

        let parsed = parse_import_payload(&payload)
            .expect("missing chapter outline status should default to draft");

        assert_eq!(
            parsed.chapter_outlines[0].status,
            chapter_outline::STATUS_DRAFT
        );
    }

    #[test]
    fn parse_import_payload_rejects_chapter_word_count_mismatch() {
        let payload = json!({
            "chapters": [
                { "chapter_number": 1, "content": "旧城A", "word_count": 2 }
            ]
        });

        let err = parse_import_payload(&payload)
            .expect_err("declared word count must match imported content characters");

        assert!(err.contains("第1章字数与正文字符数不一致"));
        assert!(err.contains("word_count=2"));
        assert!(err.contains("actual=3"));
    }

    #[test]
    fn import_chapter_status_defaults_missing_status_to_imported() {
        let status = import_chapter_status(&json!({}), "第1章")
            .expect("missing chapter status should use imported");

        assert_eq!(status, chapter::STATUS_IMPORTED);
    }

    #[test]
    fn import_chapter_status_preserves_known_persistent_statuses() {
        for status in [
            chapter::STATUS_DRAFT,
            chapter::STATUS_IMPORTED,
            chapter::STATUS_COMPLETED,
            chapter::STATUS_REVIEWED,
            chapter::STATUS_PUBLISHED,
            chapter::STATUS_DELETED,
        ] {
            let parsed = import_chapter_status(&json!({ "status": status }), "第1章")
                .expect("known persistent status should remain importable");

            assert_eq!(parsed, status);
        }
    }

    #[test]
    fn import_chapter_status_rejects_unknown_status() {
        let err = import_chapter_status(&json!({ "status": "bogus" }), "第1章")
            .expect_err("unknown chapter status must fail");

        assert!(err.contains("第1章状态无效: bogus"));
    }

    #[test]
    fn import_chapter_status_rejects_generating_status() {
        let err = import_chapter_status(&json!({ "status": "generating" }), "第1章")
            .expect_err("import must not create a stuck generating chapter");

        assert!(err.contains("生成中的瞬态状态"));
        assert!(err.contains("generating"));
    }
}
