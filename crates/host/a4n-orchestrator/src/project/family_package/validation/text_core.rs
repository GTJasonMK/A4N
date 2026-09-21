use super::*;

pub(super) fn validate_text_core_part_outline_ranges(
    package: &ProjectFamilyPackage,
) -> Result<(), String> {
    let mut ranges = package
        .text_core
        .part_outlines
        .iter()
        .map(|part| {
            (
                part.start_chapter,
                part.end_chapter,
                part.part_number,
                part.id,
            )
        })
        .collect::<Vec<_>>();
    ranges.sort_by_key(|(start_chapter, end_chapter, part_number, id)| {
        (*start_chapter, *end_chapter, *part_number, *id)
    });
    for pair in ranges.windows(2) {
        let (prev_start, prev_end, prev_part, prev_id) = pair[0];
        let (start, end, part, id) = pair[1];
        if start <= prev_end {
            return Err(format!(
                "项目族导入包分部章节范围重叠: part_outline_id={prev_id} 第{prev_part}部({prev_start}-{prev_end}) 与 part_outline_id={id} 第{part}部({start}-{end})"
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_text_core_chapter_outline_statuses(
    package: &ProjectFamilyPackage,
) -> Result<(), String> {
    for outline in &package.text_core.chapter_outlines {
        if !chapter_outline::is_valid_status(&outline.status) {
            return Err(format!(
                "项目族导入包章节大纲状态无效: chapter_outline_id={}, status={}",
                outline.id, outline.status
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_text_core_chapter_outline_references(
    package: &ProjectFamilyPackage,
) -> Result<(), String> {
    let outline_chapters = package
        .text_core
        .chapter_outlines
        .iter()
        .map(|outline| (outline.id, outline.chapter_number))
        .collect::<HashMap<_, _>>();

    for chapter in &package.text_core.chapters {
        let Some(outline_id) = chapter.outline_id else {
            continue;
        };
        let Some(outline_chapter_number) = outline_chapters.get(&outline_id).copied() else {
            return Err(format!(
                "项目族导入包章节引用了不存在的章节大纲: chapter_id={}, outline_id={outline_id}",
                chapter.id
            ));
        };
        if outline_chapter_number != chapter.chapter_number {
            return Err(format!(
                "项目族导入包章节引用的章节大纲属于第{outline_chapter_number}章: chapter_id={}, chapter_number={}, outline_id={outline_id}",
                chapter.id, chapter.chapter_number
            ));
        }
    }
    Ok(())
}

pub(super) fn validate_text_core_chapter_outline_part_references(
    package: &ProjectFamilyPackage,
) -> Result<(), String> {
    let part_ranges = package
        .text_core
        .part_outlines
        .iter()
        .map(|part| {
            (
                part.id,
                (part.part_number, part.start_chapter, part.end_chapter),
            )
        })
        .collect::<HashMap<_, _>>();

    for outline in &package.text_core.chapter_outlines {
        let Some(part_id) = outline.part_id else {
            continue;
        };
        let Some((part_number, start_chapter, end_chapter)) = part_ranges.get(&part_id).copied()
        else {
            return Err(format!(
                "项目族导入包章节大纲引用了不存在的分部: chapter_outline_id={}, part_id={part_id}",
                outline.id
            ));
        };
        if outline.chapter_number < start_chapter || outline.chapter_number > end_chapter {
            return Err(format!(
                "项目族导入包章节大纲引用的第{part_number}部分部范围不包含该章节: chapter_outline_id={}, chapter_number={}, part_id={part_id}, range={start_chapter}-{end_chapter}",
                outline.id, outline.chapter_number
            ));
        }
    }
    Ok(())
}
