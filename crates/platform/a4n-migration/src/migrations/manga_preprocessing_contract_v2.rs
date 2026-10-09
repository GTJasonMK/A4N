//! Invalidates chapter outputs created with the earlier preprocessing schema.

use sea_orm::ConnectionTrait;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(r#"
                UPDATE manga_pipeline_artifact AS artifact
                SET stale = TRUE
                WHERE artifact.stage IN ('preprocessing', 'page_slots', 'page_content', 'adaptation', 'storyboard', 'reference_discovery', 'reference_bindings', 'drawing_prompt', 'image')
                  AND EXISTS (
                      SELECT 1
                      FROM manga_pipeline_artifact AS preprocessing
                      WHERE preprocessing.project_id = artifact.project_id
                        AND preprocessing.chapter_id = artifact.chapter_id
                        AND preprocessing.stage = 'preprocessing'
                        AND preprocessing.payload ->> 'schema_version' = 'manga_preprocessing_v1'
                  )
            "#)
            .await
            .map(|_| ())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "manga_preprocessing_contract_v2 不可逆：旧预处理产物不能恢复到当前合同".into(),
        ))
    }
}
