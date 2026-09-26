use crate::galgame::GalgameAssetPlanSpecRecordError;
use a4n_db_models::models::generation::generation_spec;
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder,
};
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};

pub async fn galgame_asset_generation_specs(
    db: &DatabaseConnection,
    project_id: i64,
    target_kinds: &[&str],
    generation_spec_ids: &[i64],
    limit: Option<usize>,
) -> Result<Vec<generation_spec::Model>, GalgameAssetPlanSpecRecordError> {
    validate_galgame_asset_generation_spec_query(
        project_id,
        target_kinds,
        generation_spec_ids,
        limit,
    )?;
    let target_kinds = target_kinds
        .iter()
        .map(|value| value.trim().to_string())
        .collect::<Vec<_>>();
    let mut query = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.eq(project_id))
        .filter(generation_spec::Column::TargetKind.is_in(target_kinds))
        .order_by_asc(generation_spec::Column::Id);
    if !generation_spec_ids.is_empty() {
        query = query.filter(generation_spec::Column::Id.is_in(generation_spec_ids.to_vec()));
    }
    let mut specs = query.all(db).await?;
    if !generation_spec_ids.is_empty() && specs.len() != generation_spec_ids.len() {
        let found = specs.iter().map(|spec| spec.id).collect::<HashSet<_>>();
        let missing = generation_spec_ids
            .iter()
            .filter(|id| !found.contains(id))
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        return Err(GalgameAssetPlanSpecRecordError::InvalidArgument(format!(
            "部分 Galgame 资产生成意图不存在或不属于当前项目: {}",
            missing.join(", ")
        )));
    }
    if let Some(limit) = limit {
        specs.truncate(limit);
    }
    Ok(specs)
}

pub(in crate::galgame) fn validate_galgame_asset_generation_spec_query(
    project_id: i64,
    target_kinds: &[&str],
    generation_spec_ids: &[i64],
    limit: Option<usize>,
) -> Result<(), GalgameAssetPlanSpecRecordError> {
    if project_id <= 0 {
        return Err(GalgameAssetPlanSpecRecordError::InvalidArgument(
            "project_id 必须是正整数".to_string(),
        ));
    }
    if target_kinds.is_empty() {
        return Err(GalgameAssetPlanSpecRecordError::InvalidArgument(
            "target_kinds 不能为空".to_string(),
        ));
    }
    if let Some(target_kind) = target_kinds.iter().find(|value| value.trim().is_empty()) {
        return Err(GalgameAssetPlanSpecRecordError::InvalidArgument(format!(
            "target_kinds 不能包含空值: {target_kind:?}"
        )));
    }
    if let Some(spec_id) = generation_spec_ids.iter().find(|id| **id <= 0) {
        return Err(GalgameAssetPlanSpecRecordError::InvalidArgument(format!(
            "generation_spec_ids 必须全部是正整数: {spec_id}"
        )));
    }
    let unique_ids = generation_spec_ids.iter().copied().collect::<HashSet<_>>();
    if unique_ids.len() != generation_spec_ids.len() {
        return Err(GalgameAssetPlanSpecRecordError::InvalidArgument(
            "generation_spec_ids 不能包含重复值".to_string(),
        ));
    }
    if limit == Some(0) {
        return Err(GalgameAssetPlanSpecRecordError::InvalidArgument(
            "limit 必须大于 0".to_string(),
        ));
    }
    Ok(())
}

pub(super) async fn galgame_generation_specs_by_canonical_key<C>(
    db: &C,
    project_id: i64,
) -> Result<BTreeMap<(Option<i64>, String), generation_spec::Model>, DbErr>
where
    C: ConnectionTrait,
{
    let rows = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.eq(project_id))
        .order_by_asc(generation_spec::Column::Id)
        .all(db)
        .await?;
    let mut specs_by_canonical_key = BTreeMap::new();
    for row in rows {
        let Some(canonical_key) = row.params_json.get("canonical_key").and_then(Value::as_str)
        else {
            continue;
        };
        specs_by_canonical_key
            .entry((row.source_snapshot_id, canonical_key.to_string()))
            .or_insert(row);
    }
    Ok(specs_by_canonical_key)
}
