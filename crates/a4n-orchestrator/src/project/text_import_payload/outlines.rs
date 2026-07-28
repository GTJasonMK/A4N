use super::*;

pub(super) fn parse_import_part_outlines(root: &Value) -> Result<Vec<ImportPartOutline>, String> {
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

pub(super) fn parse_import_chapter_outlines(
    root: &Value,
) -> Result<Vec<ImportChapterOutline>, String> {
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
