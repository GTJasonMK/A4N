use a4n_core::{error::AppError, models::chapter::chapter_scene};
use a4n_db::project::ProjectChapterSceneUpdateInput;
use chrono::NaiveDateTime;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ChapterSceneUpdatePatch {
    pub content: Option<String>,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub scene_type: Option<String>,
    pub location: Option<String>,
    pub timeline_position: Option<String>,
}

pub(crate) fn chapter_scene_update_record_input(
    scene: chapter_scene::Model,
    patch: ChapterSceneUpdatePatch,
    updated_at: NaiveDateTime,
) -> Result<ProjectChapterSceneUpdateInput, AppError> {
    Ok(ProjectChapterSceneUpdateInput {
        scene,
        content: patch
            .content
            .map(|value| required_updated_text(&value, "场景正文"))
            .transpose()?,
        title: patch.title.map(|value| trimmed_optional_text(&value)),
        summary: patch.summary.map(|value| trimmed_optional_text(&value)),
        scene_type: patch
            .scene_type
            .map(|value| required_updated_text(&value, "场景类型"))
            .transpose()?,
        location: patch.location.map(|value| trimmed_optional_text(&value)),
        timeline_position: patch
            .timeline_position
            .map(|value| trimmed_optional_text(&value)),
        updated_at,
    })
}

pub(crate) fn chapter_scene_reorder_is_noop(order: &[(i64, i32)]) -> bool {
    order.is_empty()
}

fn trimmed_optional_text(value: &str) -> Option<String> {
    let value = value.trim();
    if value.is_empty() {
        None
    } else {
        Some(value.to_string())
    }
}

fn required_updated_text(value: &str, field: &str) -> Result<String, AppError> {
    trimmed_optional_text(value)
        .ok_or_else(|| AppError::InvalidArgument(format!("{field}不能为空")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    #[test]
    fn update_patch_normalizes_nullable_text_and_requires_core_fields() {
        let input = chapter_scene_update_record_input(
            scene(),
            ChapterSceneUpdatePatch {
                content: Some("  新正文  ".to_string()),
                title: Some("   ".to_string()),
                scene_type: Some("  climax  ".to_string()),
                location: Some("  旧城  ".to_string()),
                ..Default::default()
            },
            now(),
        )
        .expect("valid patch");

        assert_eq!(input.content.as_deref(), Some("新正文"));
        assert_eq!(input.title, Some(None));
        assert_eq!(input.scene_type.as_deref(), Some("climax"));
        assert_eq!(input.location, Some(Some("旧城".to_string())));

        let error = chapter_scene_update_record_input(
            scene(),
            ChapterSceneUpdatePatch {
                content: Some("   ".to_string()),
                ..Default::default()
            },
            now(),
        )
        .expect_err("blank scene content must fail");
        assert!(matches!(
            error,
            AppError::InvalidArgument(message) if message == "场景正文不能为空"
        ));
    }

    #[test]
    fn empty_reorder_is_an_explicit_noop() {
        assert!(chapter_scene_reorder_is_noop(&[]));
        assert!(!chapter_scene_reorder_is_noop(&[(1, 2)]));
    }

    fn scene() -> chapter_scene::Model {
        chapter_scene::Model {
            id: 1,
            chapter_id: 2,
            scene_number: 1,
            title: None,
            content: "原正文".to_string(),
            word_count: 3,
            summary: None,
            scene_type: "development".to_string(),
            characters_present: None,
            location: None,
            timeline_position: None,
            transition_from_previous: None,
            created_at: now(),
            updated_at: now(),
        }
    }

    fn now() -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 1, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
    }
}
