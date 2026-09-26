use std::collections::BTreeMap;

use a4n_db_models::models::character::protagonist_profile;
use a4n_domain::character::{profile_current_state_fields, profile_reset_base};
use sea_orm::{ActiveModelTrait, ConnectionTrait, DbBackend, DbErr, Statement};
use serde_json::Value;

pub(super) async fn restore_snapshot_referenced_profiles<C: ConnectionTrait>(
    db: &C,
) -> Result<(), DbErr> {
    let rows = db.query_all(Statement::from_string(DbBackend::Postgres, r#"
        SELECT snapshot.root_project_id, snapshot.source_profile_id, snapshot.payload_json
        FROM source_snapshot_profile snapshot
        WHERE snapshot.profile_kind = 'character'
          AND NOT EXISTS (SELECT 1 FROM protagonist_profile profile WHERE profile.id = snapshot.source_profile_id)
        ORDER BY snapshot.root_project_id, snapshot.source_profile_id, snapshot.id
    "#)).await?;
    let now = sea_orm::prelude::DateTimeUtc::from(std::time::SystemTime::now()).naive_utc();
    let mut restored = BTreeMap::new();
    for row in rows {
        let root_project_id: i64 = row.try_get("", "root_project_id")?;
        let source_profile_id: i64 = row.try_get("", "source_profile_id")?;
        let payload: Value = row.try_get("", "payload_json")?;
        let mut profile: protagonist_profile::Model =
            serde_json::from_value(payload).map_err(|error| {
                DbErr::Custom(format!(
                    "角色 {source_profile_id} 的冻结档案不完整，无法恢复身份：{error}"
                ))
            })?;
        if profile.id != source_profile_id || profile.project_id != root_project_id {
            return Err(DbErr::Custom(format!(
                "角色 {source_profile_id} 的冻结档案身份不一致"
            )));
        }
        let text_discovered = profile.is_text_discovered();
        profile.base_attributes =
            profile_reset_base(profile.base_attributes.clone(), text_discovered)
                .map_err(DbErr::Custom)?;
        if text_discovered {
            profile.character_arc = None;
            profile.voice_fingerprint = None;
        }
        let fields = profile_current_state_fields(profile.base_attributes.clone());
        profile.gender = fields.gender;
        profile.age = fields.age;
        profile.appearance = fields.appearance;
        profile.background = fields.background;
        profile.personality = fields.personality;
        profile.current_state = fields.current_state;
        profile.last_synced_chapter = 0;
        profile.updated_at = now;
        if let Some(previous) = restored.insert(source_profile_id, profile.clone()) {
            if previous != profile {
                return Err(DbErr::Custom(format!(
                    "角色 {source_profile_id} 的冻结初始档案不一致，不能自动恢复"
                )));
            }
        }
    }
    for profile in restored.into_values() {
        let active: protagonist_profile::ActiveModel = profile.into();
        active.insert(db).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests;
