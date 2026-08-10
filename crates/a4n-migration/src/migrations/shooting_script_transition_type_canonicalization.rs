//! 将严格转场枚举启用前保存的自然语言值一次性规范化为当前合同。

use a4n_domain::shooting_script::ChapterShootingScriptV1;
use sea_orm::{ConnectionTrait, DbBackend, Statement};
use sea_orm_migration::prelude::*;
use serde_json::Value;
use sha2::{Digest, Sha256};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom(
                "A4N shooting script transition canonicalization only supports PostgreSQL"
                    .to_string(),
            ));
        }

        canonicalize_stored_scripts(manager).await?;
        canonicalize_shooting_script_checkpoints(manager).await?;
        Ok(())
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "shooting_script_transition_type_canonicalization 不可逆：历史自然语言转场值不能可靠恢复"
                .to_string(),
        ))
    }
}

async fn canonicalize_stored_scripts(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    let connection = manager.get_connection();
    let rows = connection
        .query_all(Statement::from_string(
            DbBackend::Postgres,
            "SELECT id, script_json FROM chapter_shooting_script ORDER BY id".to_string(),
        ))
        .await?;

    for row in rows {
        let record_id: i64 = row.try_get("", "id")?;
        let mut script_json: Value = row.try_get("", "script_json")?;
        if normalize_transition_types(&mut script_json) == 0 {
            continue;
        }

        let script: ChapterShootingScriptV1 =
            serde_json::from_value(script_json).map_err(|error| {
                DbErr::Custom(format!(
                    "章节拍摄剧本 {record_id} 的历史转场值规范化后仍无法解析：{error}"
                ))
            })?;
        script.validate().map_err(|error| {
            DbErr::Custom(format!(
                "章节拍摄剧本 {record_id} 的历史转场值规范化后校验失败：{error}"
            ))
        })?;
        let canonical_json = serde_json::to_value(&script).map_err(|error| {
            DbErr::Custom(format!("序列化章节拍摄剧本 {record_id} 失败：{error}"))
        })?;
        let canonical_bytes = serde_json::to_vec(&script).map_err(|error| {
            DbErr::Custom(format!(
                "序列化章节拍摄剧本 {record_id} 哈希输入失败：{error}"
            ))
        })?;
        let script_sha256 = format!("{:x}", Sha256::digest(canonical_bytes));

        connection
            .execute(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "UPDATE chapter_shooting_script SET script_json = $1, script_sha256 = $2, updated_at = NOW() WHERE id = $3",
                [canonical_json.into(), script_sha256.into(), record_id.into()],
            ))
            .await?;
    }
    Ok(())
}

async fn canonicalize_shooting_script_checkpoints(
    manager: &SchemaManager<'_>,
) -> Result<(), DbErr> {
    let connection = manager.get_connection();
    let rows = connection
        .query_all(Statement::from_string(
            DbBackend::Postgres,
            r#"
SELECT checkpoint.id, checkpoint.checkpoint_json
FROM workflow_checkpoint AS checkpoint
INNER JOIN workflow_run AS run ON run.id = checkpoint.workflow_run_id
WHERE run.workflow_kind = 'chapter_shooting_script'
  AND checkpoint.checkpoint_json IS NOT NULL
ORDER BY checkpoint.id
"#
            .to_string(),
        ))
        .await?;

    for row in rows {
        let checkpoint_id: i64 = row.try_get("", "id")?;
        let mut checkpoint_json: Value = row.try_get("", "checkpoint_json")?;
        if normalize_transition_types(&mut checkpoint_json) == 0 {
            continue;
        }
        connection
            .execute(Statement::from_sql_and_values(
                DbBackend::Postgres,
                "UPDATE workflow_checkpoint SET checkpoint_json = $1, updated_at = NOW() WHERE id = $2",
                [checkpoint_json.into(), checkpoint_id.into()],
            ))
            .await?;
    }
    Ok(())
}

fn normalize_transition_types(value: &mut Value) -> usize {
    match value {
        Value::Array(items) => items.iter_mut().map(normalize_transition_types).sum(),
        Value::Object(fields) => {
            let mut changed = 0;
            for (key, value) in fields {
                if key == "transition_type" {
                    let Value::String(raw) = value else {
                        continue;
                    };
                    let Some(canonical) = canonical_transition_type(raw) else {
                        continue;
                    };
                    if raw != canonical {
                        *raw = canonical.to_string();
                        changed += 1;
                    }
                } else {
                    changed += normalize_transition_types(value);
                }
            }
            changed
        }
        _ => 0,
    }
}

fn canonical_transition_type(value: &str) -> Option<&'static str> {
    match value.trim() {
        "continuous" | "hard_cut" | "match_cut" | "whip_pan" | "action_wipe" | "sound_bridge"
        | "dissolve" | "fade" | "focus_transition" | "morph" => None,
        "continuity"
        | "连续动作转场"
        | "时间顺接"
        | "连续时间推进"
        | "时间推进"
        | "同场连续"
        | "同场景时间推进"
        | "连续转场"
        | "同场景连续转场"
        | "连续时空转场"
        | "时空连续转场" => Some("continuous"),
        "主观回忆" | "回忆延展至蒙太奇" | "时间推进与场景转换" | "意识蒙太奇转场" => {
            Some("dissolve")
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn known_legacy_transition_types_are_canonicalized_recursively() {
        let mut value = json!({
            "transition_type": "连续动作转场",
            "nested": [
                { "transition_type": "主观回忆" },
                { "transition_type": "sound_bridge" }
            ]
        });

        assert_eq!(normalize_transition_types(&mut value), 2);
        assert_eq!(value["transition_type"], "continuous");
        assert_eq!(value["nested"][0]["transition_type"], "dissolve");
        assert_eq!(value["nested"][1]["transition_type"], "sound_bridge");
    }

    #[test]
    fn unknown_transition_type_is_not_silently_rewritten() {
        let mut value = json!({ "transition_type": "模型新造的转场" });
        assert_eq!(normalize_transition_types(&mut value), 0);
        assert_eq!(value["transition_type"], "模型新造的转场");
    }
}
