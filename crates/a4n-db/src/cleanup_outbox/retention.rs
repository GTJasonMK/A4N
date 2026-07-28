use super::contracts::{CLEANUP_AUDIT_TTL, CLEANUP_STATUS_COMPLETED};
use chrono::Utc;
use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};

const PURGE_COMPLETED_AUDITS_SQL: &str = r#"
WITH expired AS (
    SELECT id
    FROM project_cleanup_outbox
    WHERE status = $1
      AND completed_at IS NOT NULL
      AND completed_at <= $2
    ORDER BY completed_at, id
    FOR UPDATE SKIP LOCKED
    LIMIT $3
)
DELETE FROM project_cleanup_outbox AS cleanup
USING expired
WHERE cleanup.id = expired.id
RETURNING cleanup.id
"#;

/// 删除已完成且超过审计保留期的 cleanup outbox 记录。
pub async fn purge_completed_cleanup_audits<C>(db: &C, limit: usize) -> Result<u64, DbErr>
where
    C: ConnectionTrait,
{
    if db.get_database_backend() != DbBackend::Postgres {
        return Err(DbErr::Custom(
            "cleanup outbox 审计 TTL 仅支持 PostgreSQL".to_string(),
        ));
    }
    if limit == 0 {
        return Ok(0);
    }
    let limit = i64::try_from(limit)
        .map_err(|_| DbErr::Custom("cleanup outbox 审计 TTL limit 超出 i64 范围".to_string()))?;
    let cutoff = Utc::now().naive_utc() - CLEANUP_AUDIT_TTL;
    let rows = db
        .query_all(Statement::from_sql_and_values(
            DbBackend::Postgres,
            PURGE_COMPLETED_AUDITS_SQL,
            [CLEANUP_STATUS_COMPLETED.into(), cutoff.into(), limit.into()],
        ))
        .await?;
    Ok(rows.len() as u64)
}
