//! Canonicalize user-imported media records to the shared external import model.

use sea_orm::{ConnectionTrait, DbBackend};
use sea_orm_migration::prelude::*;

const MIGRATION_NAME: &str = "external_import_model_identity_cleanup";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(format!(
                "A4N {MIGRATION_NAME} migration only supports PostgreSQL"
            )));
        }
        manager.get_connection().execute_unprepared(UP_SQL).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "external_import_model_identity_cleanup 不可逆：旧导入模型别名不会恢复".to_string(),
        ))
    }
}

const UP_SQL: &str = r#"
DO $$
BEGIN
    IF to_regclass(current_schema() || '.generated_video') IS NULL THEN
        RAISE EXCEPTION
            'external_import_model_identity_cleanup: generated_video table is missing';
    END IF;

    UPDATE generated_video
    SET video_type = 'reference_media_video',
        workflow_mode = 'reference_media',
        model = 'external-import',
        updated_at = NOW()
    WHERE video_type = 'reference_media';

    UPDATE generated_video
    SET model = 'external-import',
        updated_at = NOW()
    WHERE model IN (
        'external-reference-media-import',
        'external-assembly-insert'
    );
END
$$;
"#;
