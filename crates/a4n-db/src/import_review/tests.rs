use super::{
    ImportReviewCreateInput, ImportReviewMutationError, accept_import_review,
    chapter::required_import_chapter_number, create_import_reviews, reject_import_review,
};
use crate::project::{CreateProjectRecordInput, PROJECT_TYPE_NOVEL_LONG, create_project_record};
use a4n_core::models::chapter::chapter;
use a4n_domain::import_review::ImportReviewStatus;
use sea_orm::EntityTrait;
use serde_json::json;

#[test]
fn required_import_chapter_number_rejects_non_positive_number() {
    let err = required_import_chapter_number(&json!({"chapter_number": 0}), 42)
        .expect_err("chapter_number 0 must fail before writing chapters");
    assert!(err.to_string().contains("章节序号必须为正整数"));
    assert!(err.to_string().contains("review_id=42"));
}

#[tokio::test]
async fn review_transitions_are_atomic_and_single_use() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let project_id = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: 1,
            title: "导入审核事务测试".to_string(),
            description: String::new(),
            project_type: PROJECT_TYPE_NOVEL_LONG.to_string(),
        },
    )
    .await
    .expect("test project must be created")
    .id;
    let reviews = create_import_reviews(
        &db,
        (1..=3)
            .map(|chapter_number| ImportReviewCreateInput {
                project_id,
                filename: "chapters.md".to_string(),
                analysis_result: json!({
                    "chapter_number": chapter_number,
                    "chapter_title": format!("第{chapter_number}章"),
                    "content": format!("第{chapter_number}章正文"),
                }),
            })
            .collect(),
    )
    .await
    .expect("reviews must be created");

    let (left, right) = tokio::join!(
        accept_import_review(&db, reviews[0].id, None),
        accept_import_review(&db, reviews[0].id, None),
    );
    let accepted = match (left, right) {
        (Ok(accepted), Err(ImportReviewMutationError::InvalidState { .. }))
        | (Err(ImportReviewMutationError::InvalidState { .. }), Ok(accepted)) => accepted,
        result => panic!("exactly one concurrent acceptance must succeed: {result:?}"),
    };
    assert_eq!(
        accepted.review.review_status,
        ImportReviewStatus::Accepted.as_str()
    );

    let modified = accept_import_review(
        &db,
        reviews[1].id,
        Some(json!({
            "chapter_number": 2,
            "chapter_title": "修改后的第二章",
            "content": "修改后的正文",
            "summary": "修改后的摘要",
        })),
    )
    .await
    .expect("modified review must be accepted");
    assert_eq!(
        modified.review.review_status,
        ImportReviewStatus::Modified.as_str()
    );
    let chapter = chapter::Entity::find_by_id(modified.chapter_id)
        .one(&db)
        .await
        .expect("chapter query must succeed")
        .expect("accepted chapter must exist");
    assert_eq!(chapter.title.as_deref(), Some("修改后的第二章"));
    assert_eq!(chapter.content, "修改后的正文");
    assert_eq!(chapter.summary.as_deref(), Some("修改后的摘要"));

    let rejected = reject_import_review(&db, reviews[2].id)
        .await
        .expect("pending review must be rejected");
    assert_eq!(
        rejected.review_status,
        ImportReviewStatus::Rejected.as_str()
    );
    assert!(matches!(
        reject_import_review(&db, reviews[2].id).await,
        Err(ImportReviewMutationError::InvalidState { .. })
    ));
}
