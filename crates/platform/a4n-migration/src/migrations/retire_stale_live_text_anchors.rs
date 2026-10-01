use a4n_db_models::models::project::{profile_evidence, text_anchor};
use sea_orm::{ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, QueryFilter, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        let db = manager.get_connection();
        let rows = db.query_all(Statement::from_string(DbBackend::Postgres, r#"
            SELECT anchor.id, anchor.source_version_hash, chapter.content
            FROM text_anchor anchor
            LEFT JOIN chapter ON chapter.id = anchor.source_id AND chapter.project_id = anchor.project_id
            WHERE anchor.source_kind = 'chapter_content' AND anchor.source_snapshot_id IS NULL
        "#)).await?;
        let mut stale_ids = Vec::new();
        for row in rows {
            let content = row.try_get::<Option<String>>("", "content")?;
            let version = row.try_get::<String>("", "source_version_hash")?;
            if content
                .as_deref()
                .is_none_or(|content| a4n_domain::source_text_hash(content) != version)
            {
                stale_ids.push(row.try_get::<i64>("", "id")?);
            }
        }
        if !stale_ids.is_empty() {
            profile_evidence::Entity::delete_many()
                .filter(profile_evidence::Column::TextAnchorId.is_in(stale_ids.iter().copied()))
                .exec(db)
                .await?;
            text_anchor::Entity::delete_many()
                .filter(text_anchor::Column::Id.is_in(stale_ids))
                .exec(db)
                .await?;
        }
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "失效的实时正文锚点已清理，不能恢复为当前证据".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::TransactionTrait;

    #[tokio::test]
    async fn retirement_preserves_current_and_frozen_anchors() {
        let db = crate::test_support::connect_test_database().await;
        let tx = db.begin().await.unwrap();
        tx.execute_unprepared(r#"
            CREATE TEMP TABLE chapter (id bigint, project_id bigint, content text) ON COMMIT DROP;
            CREATE TEMP TABLE text_anchor (id bigint, project_id bigint, source_kind text, source_id bigint, source_snapshot_id bigint, source_version_hash text) ON COMMIT DROP;
            CREATE TEMP TABLE profile_evidence (text_anchor_id bigint) ON COMMIT DROP;
            INSERT INTO chapter VALUES (1,1,'新版正文');
            INSERT INTO profile_evidence VALUES (1),(2),(3),(4);
        "#).await.unwrap();
        tx.execute(Statement::from_sql_and_values(DbBackend::Postgres,
            "INSERT INTO text_anchor VALUES (1,1,'chapter_content',1,NULL,$1), (2,1,'chapter_content',1,NULL,$2), (3,1,'chapter_content',1,9,$1), (4,1,'chapter_content',2,NULL,$1)",
            [a4n_domain::source_text_hash("旧版正文").into(), a4n_domain::source_text_hash("新版正文").into()],
        )).await.unwrap();
        Migration.up(&SchemaManager::new(&tx)).await.unwrap();
        for sql in [
            "SELECT id FROM text_anchor ORDER BY id",
            "SELECT text_anchor_id AS id FROM profile_evidence ORDER BY id",
        ] {
            let ids = tx
                .query_all(Statement::from_string(DbBackend::Postgres, sql))
                .await
                .unwrap()
                .into_iter()
                .map(|row| row.try_get::<i64>("", "id").unwrap())
                .collect::<Vec<_>>();
            assert_eq!(ids, vec![2, 3]);
        }
        tx.rollback().await.unwrap();
    }
}
