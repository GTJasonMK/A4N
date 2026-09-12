//! Remove the retired global LLM scenario output-budget setting.

use sea_orm::{ConnectionTrait, DbBackend};
use sea_orm_migration::prelude::*;

const DELETE_SQL: &str = "DELETE FROM system_settings WHERE key = 'llm_max_tokens';";

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N llm_max_tokens setting cleanup only supports PostgreSQL".to_string(),
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
            "llm_max_tokens_setting_cleanup 不可逆：已删除的场景级输出预算不会恢复".to_string(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::DELETE_SQL;

    #[test]
    fn deletes_only_the_retired_setting_key() {
        assert_eq!(
            DELETE_SQL,
            "DELETE FROM system_settings WHERE key = 'llm_max_tokens';"
        );
    }
}
