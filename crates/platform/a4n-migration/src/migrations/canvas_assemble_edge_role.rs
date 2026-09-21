//! Normalize canvas AssembleNode clip edge roles.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N canvas assemble edge role migration only supports PostgreSQL".to_string(),
            ));
        }

        execute_pg_sql(manager, NORMALIZE_ASSEMBLE_EDGE_ROLE_SQL).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N canvas assemble edge role migration only supports PostgreSQL".to_string(),
            ));
        }

        execute_pg_sql(manager, RESTORE_ASSEMBLE_EDGE_ROLE_SQL).await?;
        Ok(())
    }
}

const NORMALIZE_ASSEMBLE_EDGE_ROLE_SQL: &str = r#"
UPDATE canvas_edge edge
SET role = 'assembly_clip'
FROM canvas_node source_node, canvas_node target_node
WHERE edge.source_node_id = source_node.id
  AND edge.target_node_id = target_node.id
  AND source_node.node_kind = 'VideoNode'
  AND target_node.node_kind = 'AssembleNode'
  AND edge.source_port = 'video'
  AND edge.target_port = 'clip'
  AND edge.role = 'source_video'
"#;

const RESTORE_ASSEMBLE_EDGE_ROLE_SQL: &str = r#"
UPDATE canvas_edge edge
SET role = 'source_video'
FROM canvas_node source_node, canvas_node target_node
WHERE edge.source_node_id = source_node.id
  AND edge.target_node_id = target_node.id
  AND source_node.node_kind = 'VideoNode'
  AND target_node.node_kind = 'AssembleNode'
  AND edge.source_port = 'video'
  AND edge.target_port = 'clip'
  AND edge.role = 'assembly_clip'
"#;

async fn execute_pg_sql(manager: &SchemaManager<'_>, sql: &str) -> Result<(), DbErr> {
    manager
        .get_connection()
        .execute(Statement::from_string(DbBackend::Postgres, sql.to_string()))
        .await?;
    Ok(())
}
