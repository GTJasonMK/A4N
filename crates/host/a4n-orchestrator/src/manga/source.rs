use a4n_error::AppError;

use crate::project::ChapterMangaSourceSummary;

pub fn ensure_manga_source_can_write(
    source: &ChapterMangaSourceSummary,
    operation: &str,
) -> Result<(), AppError> {
    if source.status == "deleted" {
        return Err(AppError::InvalidArgument(format!(
            "第 {} 章已删除，无法{operation}",
            source.chapter_number
        )));
    }
    if source.status == "generating" {
        return Err(AppError::InvalidArgument(format!(
            "第 {} 章正在生成中，请等待完成后再{operation}",
            source.chapter_number
        )));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
