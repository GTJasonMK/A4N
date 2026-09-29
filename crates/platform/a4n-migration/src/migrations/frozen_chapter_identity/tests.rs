use super::*;
use a4n_db_models::models::chapter::chapter;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, TransactionTrait};

#[tokio::test]
async fn restores_deleted_identity_without_replacing_current_chapter_and_protects_references() {
    let db = crate::test_support::connect_test_database().await;
    let tx = db.begin().await.unwrap();
    tx.execute_unprepared(r#"
        CREATE TEMP TABLE chapter (
            id bigserial PRIMARY KEY, project_id bigint NOT NULL, outline_id bigint,
            chapter_number integer NOT NULL, title text, content text NOT NULL,
            word_count integer NOT NULL, status text NOT NULL, summary text,
            style_tags jsonb, analysis_data jsonb,
            created_at timestamp NOT NULL, updated_at timestamp NOT NULL
        ) ON COMMIT DROP;
        CREATE TEMP TABLE novel_project (id bigint PRIMARY KEY) ON COMMIT DROP;
        CREATE UNIQUE INDEX idx_chapter_project_number_unique ON chapter(project_id, chapter_number);
        CREATE TEMP TABLE source_snapshot_chapter (
            id bigint, source_chapter_id bigint, root_project_id bigint, chapter_number integer
        ) ON COMMIT DROP;
        INSERT INTO novel_project VALUES (1);
        INSERT INTO chapter (id,project_id,chapter_number,title,content,word_count,status,created_at,updated_at)
            VALUES (11,1,1,'新标题','新正文',3,'draft',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP);
        INSERT INTO source_snapshot_chapter VALUES (1,10,1,1),(2,10,1,1),(3,11,1,1),(4,20,999,2);
    "#).await.unwrap();
    let current = chapter::Entity::find_by_id(11)
        .one(&tx)
        .await
        .unwrap()
        .unwrap();
    Migration.up(&SchemaManager::new(&tx)).await.unwrap();
    let restored = chapter::Entity::find_by_id(10)
        .one(&tx)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(restored.project_id, 1);
    assert_eq!(restored.chapter_number, 1);
    assert_eq!(restored.status, chapter::STATUS_DELETED);
    assert_eq!(restored.content, "");
    assert_eq!(restored.word_count, 0);
    assert_eq!(restored.title, None);
    assert_eq!(restored.analysis_data, None);
    assert!(
        chapter::Entity::find_by_id(20)
            .one(&tx)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(
        chapter::Entity::find_current().all(&tx).await.unwrap(),
        vec![current]
    );
    tx.execute_unprepared("SET CONSTRAINTS fk_source_snapshot_chapter_identity IMMEDIATE")
        .await
        .unwrap();
    let probe = tx.begin().await.unwrap();
    let error = chapter::Entity::delete_by_id(10)
        .exec(&probe)
        .await
        .unwrap_err();
    assert!(
        error
            .to_string()
            .contains("fk_source_snapshot_chapter_identity")
    );
    probe.rollback().await.unwrap();
    tx.execute_unprepared("SET CONSTRAINTS fk_source_snapshot_chapter_identity DEFERRED")
        .await
        .unwrap();
    chapter::Entity::delete_many()
        .filter(chapter::Column::ProjectId.eq(1))
        .exec(&tx)
        .await
        .unwrap();
    tx.execute_unprepared("DELETE FROM source_snapshot_chapter; SET CONSTRAINTS fk_source_snapshot_chapter_identity IMMEDIATE")
        .await.unwrap();
    tx.rollback().await.unwrap();
}
