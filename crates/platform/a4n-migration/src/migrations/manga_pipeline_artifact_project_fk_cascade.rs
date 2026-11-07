use sea_orm::{ConnectionTrait, DbBackend, DbErr};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "manga_pipeline_artifact_project_fk_cascade requires PostgreSQL".into(),
            ));
        }

        manager
            .get_connection()
            .execute_unprepared(
                r#"
ALTER TABLE manga_pipeline_artifact
    DROP CONSTRAINT IF EXISTS "fk-manga_pipeline_artifact-project_id",
    DROP CONSTRAINT IF EXISTS fk_manga_pipeline_artifact_project;
ALTER TABLE manga_pipeline_artifact
    ADD CONSTRAINT fk_manga_pipeline_artifact_project
    FOREIGN KEY (project_id) REFERENCES novel_project(id) ON DELETE CASCADE
"#,
            )
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "manga_pipeline_artifact_project_fk_cascade 不可逆：回退会重新阻止项目级联删除".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::{Statement, TransactionTrait};

    #[tokio::test]
    async fn fresh_and_upgraded_artifacts_have_one_cascading_project_foreign_key() {
        let db = crate::test_support::connect_test_database().await;
        let constraint_query = r#"
SELECT confdeltype::text AS delete_action
FROM pg_constraint
WHERE conrelid = 'manga_pipeline_artifact'::regclass
  AND confrelid = 'novel_project'::regclass
  AND contype = 'f'
"#;
        let fresh = db
            .query_all(Statement::from_string(
                DbBackend::Postgres,
                constraint_query,
            ))
            .await
            .unwrap();
        assert_eq!(fresh.len(), 1);
        assert_eq!(
            fresh[0].try_get::<String>("", "delete_action").unwrap(),
            "c"
        );

        let tx = db.begin().await.unwrap();
        tx.execute_unprepared(r#"
CREATE TEMP TABLE novel_project (id bigint PRIMARY KEY) ON COMMIT DROP;
CREATE TEMP TABLE manga_pipeline_artifact (
    id bigint PRIMARY KEY, project_id bigint NOT NULL,
    CONSTRAINT "fk-manga_pipeline_artifact-project_id" FOREIGN KEY (project_id) REFERENCES novel_project(id),
    CONSTRAINT fk_manga_pipeline_artifact_project FOREIGN KEY (project_id) REFERENCES novel_project(id) ON DELETE CASCADE
) ON COMMIT DROP;
INSERT INTO novel_project VALUES (1);
INSERT INTO manga_pipeline_artifact VALUES (10, 1);
"#).await.unwrap();
        let manager = SchemaManager::new(&tx);
        Migration.up(&manager).await.unwrap();
        Migration.up(&manager).await.unwrap();
        let repaired = tx
            .query_all(Statement::from_string(
                DbBackend::Postgres,
                constraint_query,
            ))
            .await
            .unwrap();
        assert_eq!(repaired.len(), 1);
        assert_eq!(
            repaired[0].try_get::<String>("", "delete_action").unwrap(),
            "c"
        );
        tx.execute_unprepared("DELETE FROM novel_project WHERE id = 1")
            .await
            .unwrap();
        let count = tx
            .query_one(Statement::from_string(
                DbBackend::Postgres,
                "SELECT COUNT(*) AS count FROM manga_pipeline_artifact",
            ))
            .await
            .unwrap()
            .unwrap()
            .try_get::<i64>("", "count")
            .unwrap();
        assert_eq!(count, 0);
        tx.rollback().await.unwrap();
    }
}
