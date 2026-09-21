//! Remove the retired global web-search configuration.

use sea_orm::{ConnectionTrait, DbBackend};
use sea_orm_migration::prelude::*;

const DELETE_SQL: &str = "DELETE FROM system_settings WHERE key = 'web_search';";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N web_search global config cleanup only supports PostgreSQL".to_string(),
            ));
        }
        manager
            .get_connection()
            .execute_unprepared(DELETE_SQL)
            .await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "web_search_global_config_retirement 不可逆：已删除的全局联网搜索配置不会恢复"
                .to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::DELETE_SQL;

    #[test]
    fn deletes_only_the_retired_global_config_key() {
        assert_eq!(
            DELETE_SQL,
            "DELETE FROM system_settings WHERE key = 'web_search';"
        );
    }
}
