use super::*;

pub(super) fn validate_import_chapter_outline_references(
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

pub(super) fn parse_import_chapters(root: &Value) -> Result<Vec<ImportChapter>, String> {
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

pub(super) fn validate_import_chapter_outline_part_references(
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
