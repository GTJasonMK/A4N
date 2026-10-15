use sea_orm::{ConnectionTrait, DbBackend};
use sea_orm_migration::prelude::*;

const CREATE_INDEX_SQL: &str = "CREATE INDEX IF NOT EXISTS idx_generated_video_project_chapter ON generated_video (project_id, chapter_id)";
const DROP_INDEX_SQL: &str = "DROP INDEX IF EXISTS idx_generated_video_project_chapter";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "Generated video project chapter index requires PostgreSQL".into(),
            ));
        }
        manager
            .get_connection()
            .execute_unprepared(CREATE_INDEX_SQL)
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "Generated video project chapter index requires PostgreSQL".into(),
            ));
        }
        manager
            .get_connection()
            .execute_unprepared(DROP_INDEX_SQL)
            .await?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::CREATE_INDEX_SQL;

    #[test]
    fn creates_project_chapter_filter_index_for_generated_videos() {
        assert!(CREATE_INDEX_SQL.contains("idx_generated_video_project_chapter"));
        assert!(CREATE_INDEX_SQL.contains("generated_video (project_id, chapter_id)"));
    }
}
