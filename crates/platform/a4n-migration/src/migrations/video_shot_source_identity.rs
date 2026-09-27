//! 一次性补齐可证明的镜头来源；未知来源保留为空映射，运行时明确标记过时。

use a4n_domain::{
    chapter_visual_plan::ChapterVisualPlanV3,
    shooting_script::{
        compat::v1::ChapterShootingScriptV1, project_visual_plan_for_shooting_script,
    },
};
use sea_orm::{ConnectionTrait, DbBackend, DbErr, Statement};
use sea_orm_migration::prelude::*;
use serde_json::{Value, json};

#[derive(DeriveMigrationName)]
pub struct Migration;

#[sea_orm_migration::async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        if manager.get_database_backend() != DbBackend::Postgres {
            return Err(DbErr::Custom("镜头来源迁移需要 PostgreSQL".into()));
        }
        migrate_metadata(manager.get_connection()).await
    }

    async fn down(&self, _manager: &SchemaManager) -> Result<(), DbErr> {
        Err(DbErr::Custom(
            "镜头来源迁移不可逆：不能丢弃已证明的逐镜头来源".into(),
        ))
    }
}

async fn migrate_metadata<C: ConnectionTrait>(db: &C) -> Result<(), DbErr> {
    for table in ["chapter_video_prompt", "generated_video", "generated_image"] {
        let rows = db.query_all(Statement::from_string(DbBackend::Postgres, format!(r#"
            SELECT asset.id, asset.metadata_json, script.script_json, plan.plan_json
            FROM {table} AS asset
            LEFT JOIN chapter_shooting_script AS script ON
                asset.metadata_json #>> '{{video_execution_identity,shooting_script_identity,record_id}}' = script.id::text
                AND asset.metadata_json #>> '{{video_execution_identity,shooting_script_identity,revision}}' = script.revision::text
                AND asset.metadata_json #>> '{{video_execution_identity,shooting_script_identity,source_revision_key}}' = script.source_revision_key
                AND asset.metadata_json #>> '{{video_execution_identity,shooting_script_identity,source_content_sha256}}' = script.source_content_sha256
                AND asset.metadata_json #>> '{{video_execution_identity,shooting_script_identity,script_sha256}}' = script.script_sha256
            LEFT JOIN chapter_visual_plan AS plan ON plan.id = script.visual_plan_id
                AND plan.plan_sha256 = script.visual_plan_sha256 AND plan.plan_version = script.visual_plan_version
            WHERE jsonb_typeof(asset.metadata_json::jsonb -> 'video_execution_identity') = 'object'
                AND (
                    asset.metadata_json::jsonb -> 'video_execution_identity' ?| ARRAY['projection_sha256', 'segment_plan_sha256', 'unit_plan_sha256']
                    OR (jsonb_typeof(asset.metadata_json::jsonb #> '{{video_execution_identity,shooting_script_identity}}') = 'object'
                        AND jsonb_typeof(asset.metadata_json::jsonb #> '{{video_execution_identity,shooting_script_identity,shot_fingerprints}}') IS DISTINCT FROM 'object')
                )
        "#))).await?;
        for row in rows {
            let id: i64 = row.try_get("", "id")?;
            let original: Value = row.try_get("", "metadata_json")?;
            let mut metadata = original.clone();
            let identity = metadata["video_execution_identity"]
                .as_object_mut()
                .ok_or_else(|| DbErr::Custom(format!("{table} #{id} 的执行身份不是对象")))?;
            for field in [
                "projection_sha256",
                "segment_plan_sha256",
                "unit_plan_sha256",
            ] {
                identity.remove(field);
            }
            if let Some(source) = identity
                .get_mut("shooting_script_identity")
                .and_then(Value::as_object_mut)
                && source.get("shot_fingerprints").is_none_or(Value::is_null)
            {
                let script_json: Option<Value> = row.try_get("", "script_json")?;
                let plan_json: Option<Value> = row.try_get("", "plan_json")?;
                let fingerprints = match (script_json, plan_json) {
                    (Some(script), Some(plan)) => {
                        let parse_error =
                            |error| DbErr::Custom(format!("{table} #{id} 的来源合同无效：{error}"));
                        let script: ChapterShootingScriptV1 =
                            serde_json::from_value(script).map_err(parse_error)?;
                        let plan: ChapterVisualPlanV3 =
                            serde_json::from_value(plan).map_err(parse_error)?;
                        let director = project_visual_plan_for_shooting_script(&plan, &[])
                            .map_err(|error| DbErr::Custom(error.to_string()))?;
                        script
                            .shot_content_fingerprints(&director)
                            .map_err(parse_error)?
                    }
                    _ => Default::default(),
                };
                source.insert("shot_fingerprints".into(), json!(fingerprints));
            }
            let updated = db.execute(Statement::from_sql_and_values(DbBackend::Postgres,
                format!("UPDATE {table} SET metadata_json = $1 WHERE id = $2 AND metadata_json::jsonb = $3::jsonb"),
                [metadata.into(), id.into(), original.into()],
            )).await?;
            if updated.rows_affected() != 1 {
                return Err(DbErr::Custom(format!(
                    "{table} #{id} 在来源迁移期间被修改，请重试"
                )));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests;
