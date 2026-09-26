use a4n_db::project as project_db;
use a4n_db_models::models::chapter::chapter_outline;
use a4n_error::AppError;

use crate::outline::OrchestratorContext;

pub(super) fn existing_chapter_outline_for_number(
    outlines: &[chapter_outline::Model],
    chapter_number: i32,
) -> Result<Option<chapter_outline::Model>, AppError> {
    let mut matches = outlines
        .iter()
        .filter(|outline| outline.chapter_number == chapter_number);
    let Some(first) = matches.next() else {
        return Ok(None);
    };
    let Some(second) = matches.next() else {
        return Ok(Some(first.clone()));
    };
    let ids = std::iter::once(first)
        .chain(std::iter::once(second))
        .chain(matches)
        .map(|outline| outline.id.to_string())
        .collect::<Vec<_>>()
        .join(", ");
    Err(AppError::InvalidArgument(format!(
        "第 {chapter_number} 章已有重复章纲记录：{ids}，请先清理后再生成"
    )))
}

impl OrchestratorContext {
    pub(super) async fn ensure_no_other_outline_for_chapter(
        &self,
        project_id: i64,
        chapter_number: i32,
        outline_id: i64,
    ) -> Result<(), AppError> {
        let conflicts = project_db::chapter_outlines_for_project_chapter_number(
            &self.database,
            project_id,
            chapter_number,
        )
        .await
        .map_err(crate::project::project_outline_record_error_to_app_error)?
        .into_iter()
        .filter(|outline| outline.id != outline_id)
        .map(|outline| outline.id)
        .collect::<Vec<_>>();
        if conflicts.is_empty() {
            return Ok(());
        }

        Err(AppError::InvalidArgument(format!(
            "第 {chapter_number} 章存在其他章纲记录：{}",
            conflicts
                .iter()
                .map(i64::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn outline(id: i64, chapter_number: i32) -> chapter_outline::Model {
        let now = Utc::now().naive_utc();
        chapter_outline::Model {
            id,
            project_id: 1,
            part_id: None,
            chapter_number,
            title: None,
            summary: None,
            beats: None,
            key_scenes: None,
            pacing_score: None,
            foreshadowing_setup: None,
            foreshadowing_payoff: None,
            characters_involved: None,
            word_count_target: None,
            status: chapter_outline::STATUS_DRAFT.to_string(),
            created_at: now,
            updated_at: now,
        }
    }

    #[test]
    fn finds_existing_outline_and_preserves_duplicate_error() {
        let rows = vec![outline(11, 1), outline(12, 2), outline(13, 2)];
        assert_eq!(
            existing_chapter_outline_for_number(&rows, 1)
                .unwrap()
                .map(|row| row.id),
            Some(11)
        );
        assert!(matches!(
            existing_chapter_outline_for_number(&rows, 2),
            Err(AppError::InvalidArgument(message)) if message.contains("12, 13")
        ));
    }
}
