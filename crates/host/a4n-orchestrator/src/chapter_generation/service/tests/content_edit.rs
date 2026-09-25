use std::sync::Arc;

use a4n_db_models::models::chapter::chapter;
use a4n_domain::chapter::{EditChapterContentInput, chapter_edit_revision};
use sea_orm::*;

use super::support::{insert_chapter, insert_project, setup_db};

#[tokio::test]
async fn save_content_edit_rejects_empty_content_without_changing_chapter() {
    let db = Arc::new(setup_db().await);
    let project = insert_project(db.as_ref()).await;
    let chapter = insert_chapter(db.as_ref(), project.id).await;
    let tx = db.begin().await.unwrap();

    let err = a4n_db::chapter_content::save_chapter_content_edit_record(
        &tx,
        EditChapterContentInput {
            chapter_id: chapter.id,
            expected_revision: chapter_edit_revision(&chapter.content, chapter.title.as_deref()),
            content: " \n\t ".to_string(),
            title: None,
        },
    )
    .await
    .expect_err("blank content must fail");
    tx.rollback().await.unwrap();

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

#[tokio::test]
async fn save_content_edit_rejects_a_stale_editor_without_overwriting_the_newer_save() {
    let db = setup_db().await;
    let project = insert_project(&db).await;
    let initial = insert_chapter(&db, project.id).await;
    let initial_revision = chapter_edit_revision(&initial.content, initial.title.as_deref());

    let tx = db.begin().await.unwrap();
    let saved = a4n_db::chapter_content::save_chapter_content_edit_record(
        &tx,
        EditChapterContentInput {
            chapter_id: initial.id,
            expected_revision: initial_revision.clone(),
            content: "窗口 A 保存的正文。".into(),
            title: Some("窗口 A 的标题".into()),
        },
    )
    .await
    .unwrap();
    tx.commit().await.unwrap();

    let tx = db.begin().await.unwrap();
    let error = a4n_db::chapter_content::save_chapter_content_edit_record(
        &tx,
        EditChapterContentInput {
            chapter_id: initial.id,
            expected_revision: initial_revision,
            content: "窗口 B 基于旧版本的正文。".into(),
            title: initial.title,
        },
    )
    .await
    .expect_err("a stale editor must not overwrite another save");
    assert!(matches!(
        error,
        a4n_db::chapter_content::ChapterContentEditError::InvalidState { .. }
    ));
    tx.rollback().await.unwrap();

    let stored = chapter::Entity::find_by_id(initial.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(stored.content, saved.content);
    assert_eq!(stored.title, saved.title);
    assert_eq!(stored.updated_at, saved.updated_at);
}
