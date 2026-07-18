use super::*;

pub fn validate_import_text_project_chapter_bounds(
    target_chapters: i32,
    current_chapter: i32,
    total_chapters: Option<i32>,
    chapters_per_part: Option<i32>,
    max_chapter_number: i32,
    max_available_content_chapter_number: i32,
    available_content_chapter_count: i32,
) -> Result<(), ImportTextProjectError> {
    let effective_total = total_chapters.unwrap_or(target_chapters);
    if current_chapter > effective_total {
        return Err(ImportTextProjectError::InvalidPackage(format!(
            "当前章节数不能大于总章节数: current_chapter={current_chapter}, total={effective_total}"
        )));
    }
    if max_chapter_number > effective_total {
        return Err(ImportTextProjectError::InvalidPackage(format!(
            "导入章节序号不能大于总章节数: max_chapter_number={max_chapter_number}, total={effective_total}"
        )));
    }
    if current_chapter != max_available_content_chapter_number {
        return Err(ImportTextProjectError::InvalidPackage(format!(
            "当前章节数必须等于导入可用正文中的最高章节序号: current_chapter={current_chapter}, max_content_chapter={max_available_content_chapter_number}"
        )));
    }
    if current_chapter != available_content_chapter_count {
        return Err(ImportTextProjectError::InvalidPackage(format!(
            "导入可用正文必须从第1章连续到当前章节: current_chapter={current_chapter}, available_content_chapters={available_content_chapter_count}"
        )));
    }
    if let Some(chapters_per_part) = chapters_per_part
        && chapters_per_part > effective_total
    {
        return Err(ImportTextProjectError::InvalidPackage(format!(
            "分部章节数不能大于总章节数: chapters_per_part={chapters_per_part}, total={effective_total}"
        )));
    }
    Ok(())
}

pub(super) fn validate_import_text_project_payload(
    payload: &ImportTextProjectPayload,
) -> Result<(), ImportTextProjectError> {
    for chapter in &payload.chapters {
        if !chapter::is_valid_status(&chapter.status) {
            return Err(ImportTextProjectError::InvalidPackage(format!(
                "第{}章状态无效: status={}",
                chapter.chapter_number, chapter.status
            )));
        }
        if chapter.status == chapter::STATUS_GENERATING {
            return Err(ImportTextProjectError::InvalidPackage(format!(
                "第{}章状态不能是生成中: status={}",
                chapter.chapter_number,
                chapter::STATUS_GENERATING
            )));
        }
    }
    for outline in &payload.chapter_outlines {
        if !chapter_outline::is_valid_status(&outline.status) {
            return Err(ImportTextProjectError::InvalidPackage(format!(
                "第{}章大纲状态无效: status={}",
                outline.chapter_number, outline.status
            )));
        }
    }
    Ok(())
}
