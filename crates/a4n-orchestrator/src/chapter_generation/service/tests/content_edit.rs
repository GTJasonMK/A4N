use std::sync::Arc;

use a4n_core::models::chapter::chapter;
use sea_orm::*;

use super::support::{insert_chapter, insert_project, setup_db};

#[tokio::test]
async fn save_content_edit_rejects_empty_content_without_changing_chapter() {
    let db = Arc::new(setup_db().await);
    let project = insert_project(db.as_ref()).await;
    let chapter = insert_chapter(db.as_ref(), project.id).await;

    let err = a4n_db::chapter_content::save_chapter_content_edit_record(
        db.as_ref(),
        a4n_db::chapter_content::EditChapterContentInput {
            chapter_id: chapter.id,
            content: " \n\t ".to_string(),
            title: None,
        },
    )
    .await
    .expect_err("blank content must fail");

    assert!(matches!(
        err,
        a4n_db::chapter_content::ChapterContentEditError::InvalidArgument(message)
            if message == "章节正文不能为空"
    ));
    let stored = chapter::Entity::find_by_id(chapter.id)
        .one(db.as_ref())
        .await
        .unwrap()
        .expect("chapter must still exist");
    assert_eq!(stored.content, chapter.content);
    assert_eq!(stored.word_count, chapter.word_count);
    assert_eq!(stored.status, chapter.status);
}
