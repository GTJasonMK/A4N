use std::collections::BTreeMap;

use a4n_db_models::models::chapter::chapter;
use sea_orm::{ActiveModelTrait, ConnectionTrait, DbBackend, Set, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let rows = db.query_all(Statement::from_string(DbBackend::Postgres, r#"
            SELECT w.chapter_id, w.project_id, w.input_fingerprint_json->'source_fingerprint' AS source
            FROM workflow_run w
            JOIN novel_project p ON p.id = w.project_id
            LEFT JOIN chapter c ON c.id=w.chapter_id
            WHERE w.chapter_id IS NOT NULL AND c.id IS NULL
              AND w.input_fingerprint_json::jsonb ? 'source_fingerprint'
        "#)).await?;
        let mut identities = BTreeMap::new();
        for row in rows {
            let id: i64 = row.try_get("", "chapter_id")?;
            let project_id: Option<i64> = row.try_get("", "project_id")?;
            let value: serde_json::Value = row.try_get("", "source")?;
            let integer = |field: &str| {
                value
                    .get(field)
                    .and_then(serde_json::Value::as_i64)
                    .ok_or_else(|| {
                        DbErr::Custom(format!("工作流的章节 {id} 来源指纹缺少整数 {field}"))
                    })
            };
            let source_id = integer("chapter_id")?;
            let source_project = integer("project_id")?;
            let source_number = <i32 as TryFrom<i64>>::try_from(integer("chapter_number")?)
                .map_err(|_| DbErr::Custom(format!("工作流的章节 {id} 来源序号越界")))?;
            if source_id != id
                || Some(source_project) != project_id
                || id <= 0
                || source_project <= 0
                || source_number <= 0
            {
                return Err(DbErr::Custom(format!("工作流的章节 {id} 来源身份不一致")));
            }
            let identity = (source_project, source_number);
            if identities
                .insert(id, identity)
                .is_some_and(|previous| previous != identity)
            {
                return Err(DbErr::Custom(format!(
                    "工作流的章节 {id} 存在冲突的来源身份"
                )));
            }
        }
        let now = sea_orm::prelude::DateTimeUtc::from(std::time::SystemTime::now()).naive_utc();
        for (id, (project_id, number)) in identities {
            chapter::ActiveModel {
                id: Set(id),
                project_id: Set(project_id),
                chapter_number: Set(number),
                outline_id: Set(None),
                title: Set(None),
                content: Set(String::new()),
                word_count: Set(0),
                status: Set(chapter::STATUS_DELETED.into()),
                summary: Set(None),
                style_tags: Set(None),
                analysis_data: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
            }
            .insert(db)
            .await?;
        }
        db.execute_unprepared("SELECT setval(pg_get_serial_sequence('chapter', 'id'), GREATEST((SELECT MAX(id) FROM chapter), nextval(pg_get_serial_sequence('chapter', 'id'))))").await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "工作流保留的已删除章节身份不能重新删除".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{EntityTrait, TransactionTrait};

    #[tokio::test]
    async fn history_restores_only_proven_deleted_identity_and_preserves_live_content() {
        let db = crate::test_support::connect_test_database().await;
        let tx = db.begin().await.unwrap();
        tx.execute_unprepared(r#"
            CREATE TEMP TABLE chapter (
                id bigserial PRIMARY KEY, project_id bigint NOT NULL, outline_id bigint,
                chapter_number integer NOT NULL, title text, content text NOT NULL, word_count integer NOT NULL,
                status text NOT NULL, summary text, style_tags jsonb, analysis_data jsonb,
                created_at timestamp NOT NULL, updated_at timestamp NOT NULL
            ) ON COMMIT DROP;
            CREATE TEMP TABLE novel_project (id bigint PRIMARY KEY) ON COMMIT DROP;
            CREATE TEMP TABLE workflow_run (chapter_id bigint, project_id bigint, input_fingerprint_json jsonb) ON COMMIT DROP;
            INSERT INTO novel_project VALUES (90);
            INSERT INTO chapter (id,project_id,chapter_number,content,word_count,status,created_at,updated_at)
                VALUES (20,90,1,'current',7,'draft',CURRENT_TIMESTAMP,CURRENT_TIMESTAMP);
        "#).await.unwrap();
        let source = serde_json::json!({"source_fingerprint":{
            "project_id":90,"chapter_id":19,"chapter_number":1,"title":"old title",
            "content_sha256":a4n_domain::source_text_hash("old content")
        }});
        tx.execute(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO workflow_run VALUES (19,90,$1),(19,90,$1)",
            [source.into()],
        ))
        .await
        .unwrap();
        Migration.up(&SchemaManager::new(&tx)).await.unwrap();
        let retired = chapter::Entity::find_by_id(19)
            .one(&tx)
            .await
            .unwrap()
            .unwrap();
        assert_eq!((retired.project_id, retired.chapter_number), (90, 1));
        assert_eq!(retired.status, chapter::STATUS_DELETED);
        assert!(retired.content.is_empty() && retired.title.is_none());
        let current = chapter::Entity::find_current().all(&tx).await.unwrap();
        assert_eq!(current.len(), 1);
        assert_eq!(current[0].content, "current");
        let orphan_project = serde_json::json!({"source_fingerprint":{
            "project_id":91,"chapter_id":23,"chapter_number":2,"title":null,
            "content_sha256":a4n_domain::source_text_hash("orphan")
        }});
        tx.execute(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO workflow_run VALUES (23,91,$1)",
            [orphan_project.into()],
        ))
        .await
        .unwrap();
        Migration.up(&SchemaManager::new(&tx)).await.unwrap();
        assert!(
            chapter::Entity::find_by_id(23)
                .one(&tx)
                .await
                .unwrap()
                .is_none()
        );
        let bad = serde_json::json!({"source_fingerprint":{
            "project_id":90,"chapter_id":22,"chapter_number":2,"title":null,
            "content_sha256":a4n_domain::source_text_hash("other"),"updated_at":"2026-01-01T00:00:00"
        }});
        tx.execute(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "INSERT INTO workflow_run VALUES (21,90,$1)",
            [bad.into()],
        ))
        .await
        .unwrap();
        assert!(
            Migration
                .up(&SchemaManager::new(&tx))
                .await
                .unwrap_err()
                .to_string()
                .contains("来源身份不一致")
        );
        assert!(
            chapter::Entity::find_by_id(21)
                .one(&tx)
                .await
                .unwrap()
                .is_none()
        );
        tx.rollback().await.unwrap();
    }
}
