//! 用章节拍摄剧本硬切旧共享导演计划和视频镜头分析链。

use a4n_db_models::models::chapter::chapter_shooting_script;
use sea_orm::{DbBackend, EntityName, Schema};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_postgres(manager)?;
        let schema = Schema::new(DbBackend::Postgres);
        let mut table = schema.create_table_from_entity(chapter_shooting_script::Entity);
        table.if_not_exists();
        manager.create_table(table).await?;
        manager
            .create_index(
                Index::create()
                    .name("uidx_chapter_shooting_script_source_revision")
                    .table(Alias::new(chapter_shooting_script::Entity.table_name()))
                    .col(chapter_shooting_script::Column::RootProjectId)
                    .col(chapter_shooting_script::Column::SourceChapterId)
                    .col(chapter_shooting_script::Column::SourceRevisionKey)
                    .unique()
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;
        manager
            .create_index(
                Index::create()
                    .name("idx_chapter_shooting_script_snapshot_chapter")
                    .table(Alias::new(chapter_shooting_script::Entity.table_name()))
                    .col(chapter_shooting_script::Column::SourceSnapshotChapterId)
                    .if_not_exists()
                    .to_owned(),
            )
            .await?;

        manager
            .get_connection()
            .execute_unprepared(
                r#"
DELETE FROM generation_run_target
WHERE target_kind = 'chapter_video_prompt'
  AND target_id IN (SELECT id FROM chapter_video_prompt);

DELETE FROM chapter_video_prompt;
"#,
            )
            .await?;
        let legacy_table = ["chapter_", "directing_", "plan"].concat();
        manager
            .get_connection()
            .execute_unprepared(&format!("DROP TABLE IF EXISTS {legacy_table}"))
            .await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        ensure_postgres(manager)?;
        manager
            .drop_table(
                Table::drop()
                    .table(Alias::new(chapter_shooting_script::Entity.table_name()))
                    .if_exists()
                    .to_owned(),
            )
            .await
    }
}

fn ensure_postgres(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    if manager.get_database_backend() != DbBackend::Postgres {
        return Err(DbErr::Custom(
            "A4N chapter_shooting_script migration only supports PostgreSQL".to_string(),
        ));
    }
    Ok(())
}
