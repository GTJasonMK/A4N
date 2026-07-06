//! Materialize AssembleNode clip order on canvas edges.

use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N canvas assemble clip order migration only supports PostgreSQL".to_string(),
            ));
        }

        execute_pg_sql(manager, MATERIALIZE_ASSEMBLE_CLIP_ORDER_SQL).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N canvas assemble clip order migration only supports PostgreSQL".to_string(),
            ));
        }

        execute_pg_sql(manager, REMOVE_MATERIALIZED_ASSEMBLE_CLIP_ORDER_SQL).await?;
        Ok(())
    }
}

const MATERIALIZE_ASSEMBLE_CLIP_ORDER_SQL: &str = r#"
WITH ordered_edges AS (
    SELECT
        edge.id,
        row_number() OVER (
            PARTITION BY edge.target_node_id
            ORDER BY edge.id
        ) AS order_index
    FROM canvas_edge edge
    JOIN canvas_node source_node ON source_node.id = edge.source_node_id
    JOIN canvas_node target_node ON target_node.id = edge.target_node_id
    WHERE source_node.node_kind = 'VideoNode'
      AND target_node.node_kind = 'AssembleNode'
      AND edge.source_port = 'video'
      AND edge.target_port = 'clip'
      AND edge.role = 'assembly_clip'
      AND edge.config_json->>'order_index' IS NULL
)
UPDATE canvas_edge edge
SET config_json = jsonb_set(
    COALESCE(edge.config_json::jsonb, '{}'::jsonb),
    '{order_index}',
    to_jsonb(ordered_edges.order_index),
    true
)::json
FROM ordered_edges
WHERE edge.id = ordered_edges.id
"#;

const REMOVE_MATERIALIZED_ASSEMBLE_CLIP_ORDER_SQL: &str = r#"
UPDATE canvas_edge edge
SET config_json = (edge.config_json::jsonb - 'order_index')::json
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
