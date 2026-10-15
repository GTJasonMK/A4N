use sea_orm::{DbBackend, sea_query::IndexCreateStatement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

fn project_chapter_index() -> IndexCreateStatement {
    Index::create()
        .name("idx_generated_image_project_chapter")
        .table(Alias::new("generated_image"))
        .col(Alias::new("project_id"))
        .col(Alias::new("chapter_id"))
        .if_not_exists()
        .to_owned()
}

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "Generated image project chapter index requires PostgreSQL".into(),
            ));
        }
        manager.create_index(project_chapter_index()).await
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "Generated image project chapter index requires PostgreSQL".into(),
            ));
        }
        manager
            .drop_index(
                Index::drop()
                    .name("idx_generated_image_project_chapter")
                    .table(Alias::new("generated_image"))
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}

#[cfg(test)]
mod tests {
    use sea_orm_migration::prelude::PostgresQueryBuilder;

    use super::*;

    #[test]
    fn creates_project_chapter_filter_index_for_generated_images() {
        let statement = project_chapter_index().to_string(PostgresQueryBuilder);

        assert!(statement.contains("idx_generated_image_project_chapter"));
        assert!(statement.contains("generated_image"));
        assert!(statement.contains("project_id"));
        assert!(statement.contains("chapter_id"));
    }
}
