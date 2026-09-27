//! 将严格转场枚举启用前保存的自然语言值一次性规范化为当前合同。

use a4n_domain::shooting_script::{ShootingTransitionTypeV1, compat::v1::ChapterShootingScriptV1};
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

pub(crate) async fn canonicalize_stored_scripts(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
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
        let changed = normalize_transition_types(&mut script_json).map_err(|value| {
            DbErr::Custom(format!(
                "章节拍摄剧本 {record_id} 含未知 transition_type: {value}"
            ))
        })?;

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
        if changed == 0 {
            continue;
        }
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

pub(crate) async fn canonicalize_shooting_script_checkpoints(
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
        if normalize_transition_types(&mut checkpoint_json).map_err(|value| {
            DbErr::Custom(format!(
                "拍摄剧本 checkpoint {checkpoint_id} 含未知 transition_type: {value}"
            ))
        })? == 0
        {
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

pub(crate) fn normalize_transition_types(value: &mut Value) -> Result<usize, String> {
    match value {
        Value::Array(items) => items.iter_mut().try_fold(0, |changed, item| {
            normalize_transition_types(item).map(|item_changed| changed + item_changed)
        }),
        Value::Object(fields) => fields.iter_mut().try_fold(0, |changed, (key, value)| {
            if key == "transition_type"
                && let Value::String(raw) = value
            {
                let canonical = canonical_transition_type(raw).ok_or_else(|| raw.clone())?;
                if raw != canonical.as_str() {
                    *raw = canonical.as_str().to_string();
                    return Ok(changed + 1);
                }
                return Ok(changed);
            }
            normalize_transition_types(value).map(|item_changed| changed + item_changed)
        }),
        _ => Ok(0),
    }
}

fn canonical_transition_type(value: &str) -> Option<ShootingTransitionTypeV1> {
    match value.trim() {
        "continuous" => Some(ShootingTransitionTypeV1::Continuous),
        "hard_cut" => Some(ShootingTransitionTypeV1::HardCut),
        "match_cut" => Some(ShootingTransitionTypeV1::MatchCut),
        "whip_pan" => Some(ShootingTransitionTypeV1::WhipPan),
        "action_wipe" => Some(ShootingTransitionTypeV1::ActionWipe),
        "sound_bridge" => Some(ShootingTransitionTypeV1::SoundBridge),
        "dissolve" => Some(ShootingTransitionTypeV1::Dissolve),
        "fade" => Some(ShootingTransitionTypeV1::Fade),
        "focus_transition" => Some(ShootingTransitionTypeV1::FocusTransition),
        "morph" => Some(ShootingTransitionTypeV1::Morph),
        "continuity"
        | "continuous_action"
        | "action_continuity"
        | "continuity_cut"
        | "dialogue_continuity"
        | "direct_continuity"
        | "spatial_continuity"
        | "直接连戏"
        | "连续动作转场"
        | "时间顺接"
        | "连续时间推进"
        | "时间推进"
        | "同场连续"
        | "同场景时间推进"
        | "连续转场"
        | "同场景连续转场"
        | "连续时空转场"
        | "时空连续转场" => Some(ShootingTransitionTypeV1::Continuous),
        "cut" | "direct_cut" | "TIME_CUT" | "time_cut" => Some(ShootingTransitionTypeV1::HardCut),
        "match on action" | "cut_on_glance" | "cause_effect" => {
            Some(ShootingTransitionTypeV1::MatchCut)
        }
        "sound_cut" => Some(ShootingTransitionTypeV1::SoundBridge),
        "主观回忆"
        | "回忆延展至蒙太奇"
        | "时间推进与场景转换"
        | "意识蒙太奇转场"
        | "time_ellipsis"
        | "time_jump"
        | "montage_to_scene"
        | "意识闪回" => Some(ShootingTransitionTypeV1::Dissolve),
        "意识回归" => Some(ShootingTransitionTypeV1::FocusTransition),
        _ => None,
    }
}
