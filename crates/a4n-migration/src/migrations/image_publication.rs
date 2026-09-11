//! 图片文件发布意图。

use a4n_db_models::models::media::image_publication;
use sea_orm::{ConnectionTrait, DbBackend, EntityName, Schema, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

fn image_publication_table() -> TableCreateStatement {
    let schema = Schema::new(DbBackend::Postgres);
    let mut table = schema.create_table_from_entity(image_publication::Entity);
    table.if_not_exists();
    table
}

fn image_publication_claim_index() -> IndexCreateStatement {
    Index::create()
        .name("idx_image_publication_recovery")
        .table(Alias::new(image_publication::Entity.table_name()))
        .col(image_publication::Column::Status)
        .col(image_publication::Column::UpdatedAt)
        .if_not_exists()
        .to_owned()
}

fn image_publication_project_index() -> IndexCreateStatement {
    Index::create()
        .name("idx_image_publication_project")
        .table(Alias::new(image_publication::Entity.table_name()))
        .col(image_publication::Column::ProjectId)
        .col(image_publication::Column::Status)
        .if_not_exists()
        .to_owned()
}

fn image_publication_constraints() -> Statement {
    Statement::from_string(
        DbBackend::Postgres,
        r#"DO $$
BEGIN
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'ck_image_publication_status'
    ) THEN
        ALTER TABLE image_publication
            ADD CONSTRAINT ck_image_publication_status
            CHECK (status IN ('pending', 'published', 'reclaiming', 'reclaimed'));
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'ck_image_publication_version_positive'
    ) THEN
        ALTER TABLE image_publication
            ADD CONSTRAINT ck_image_publication_version_positive
            CHECK (version > 0);
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'ck_image_publication_file_facts'
    ) THEN
        ALTER TABLE image_publication
            ADD CONSTRAINT ck_image_publication_file_facts
            CHECK (
                expected_file_size >= 0
                AND (file_size IS NULL OR file_size >= 0)
                AND length(expected_sha256) = 64
                AND (file_sha256 IS NULL OR length(file_sha256) = 64)
            );
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'ck_image_publication_paths_distinct'
    ) THEN
        ALTER TABLE image_publication
            ADD CONSTRAINT ck_image_publication_paths_distinct
            CHECK (staging_path <> final_path);
    END IF;
    IF NOT EXISTS (
        SELECT 1 FROM pg_constraint
        WHERE conname = 'ck_image_publication_published_identity'
    ) THEN
        ALTER TABLE image_publication
            ADD CONSTRAINT ck_image_publication_published_identity
            CHECK (
                status <> 'published'
                OR (
                    image_id IS NOT NULL
                    AND file_size IS NOT NULL
                    AND file_sha256 IS NOT NULL
                )
            );
    END IF;
END $$;"#,
    )
}

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N image publication migration only supports PostgreSQL".to_string(),
            ));
        }
        manager.create_table(image_publication_table()).await?;
        manager
            .create_index(image_publication_claim_index())
            .await?;
        manager
            .create_index(image_publication_project_index())
            .await?;
        manager
            .get_connection()
            .execute(image_publication_constraints())
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .drop_table(
                Table::drop()
                    .table(Alias::new(image_publication::Entity.table_name()))
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm_migration::prelude::PostgresQueryBuilder;

    #[test]
    fn publication_schema_uses_one_stable_table_and_recovery_indexes() {
        assert_eq!(image_publication::Entity.table_name(), "image_publication");
        for statement in [
            image_publication_table().to_string(PostgresQueryBuilder),
            image_publication_claim_index().to_string(PostgresQueryBuilder),
            image_publication_project_index().to_string(PostgresQueryBuilder),
            image_publication_constraints().to_string(),
        ] {
            assert!(statement.contains("image_publication"), "{statement}");
        }
    }
}
