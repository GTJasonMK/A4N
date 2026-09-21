use serde_json::{Value, json};

use crate::project::{ProjectMutationError, source_snapshot::DerivedChapterScope};

pub(in crate::project::source_snapshot) fn normalize_chapter_scope(
    scope: DerivedChapterScope,
) -> Result<DerivedChapterScope, ProjectMutationError> {
    if scope.start_chapter <= 0 || scope.end_chapter <= 0 {
        return Err(ProjectMutationError::InvalidArgument(
            "派生章节范围必须为正整数".to_string(),
        ));
    }
    if scope.start_chapter > scope.end_chapter {
        return Err(ProjectMutationError::InvalidArgument(
            "派生章节范围起点不能大于终点".to_string(),
        ));
    }
    Ok(scope)
}

pub(in crate::project::source_snapshot) fn chapter_scope_json(scope: DerivedChapterScope) -> Value {
    json!({
        "start": scope.start_chapter,
        "end": scope.end_chapter,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chapter_scope_requires_positive_ordered_bounds() {
        assert!(matches!(
            normalize_chapter_scope(DerivedChapterScope {
                start_chapter: 0,
                end_chapter: 1,
            }),
            Err(ProjectMutationError::InvalidArgument(message))
                if message == "派生章节范围必须为正整数"
        ));
        assert!(matches!(
            normalize_chapter_scope(DerivedChapterScope {
                start_chapter: 3,
                end_chapter: 2,
            }),
            Err(ProjectMutationError::InvalidArgument(message))
                if message == "派生章节范围起点不能大于终点"
        ));
        assert_eq!(
            normalize_chapter_scope(DerivedChapterScope {
                start_chapter: 2,
                end_chapter: 3,
            })
            .expect("ordered positive scope should remain valid"),
            DerivedChapterScope {
                start_chapter: 2,
                end_chapter: 3,
            }
        );
    }

    #[test]
    fn chapter_scope_json_preserves_bounds() {
        assert_eq!(
            chapter_scope_json(DerivedChapterScope {
                start_chapter: 2,
                end_chapter: 4,
            }),
            json!({ "start": 2, "end": 4 })
        );
    }
}
