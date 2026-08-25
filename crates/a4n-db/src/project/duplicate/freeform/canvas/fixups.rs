use crate::project::duplicate::ProjectDuplicateError;
use crate::project::duplicate::freeform::remap::remap_freeform_metadata;
use crate::project::duplicate::freeform::{DeferredFreeformSpecTarget, FreeformCopyMaps};
use crate::project::duplicate::shared::remap_derived_required_id;
use a4n_core::models::generation::generation_spec;
use chrono::NaiveDateTime;
use sea_orm::sea_query::{Alias, CaseStatement, Expr};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};
use std::collections::HashMap;

pub(in crate::project::duplicate::freeform) async fn update_freeform_generation_spec_deferred_targets_record<
    C,
>(
    db: &C,
    targets: &[DeferredFreeformSpecTarget],
    maps: &FreeformCopyMaps,
    now: NaiveDateTime,
) -> Result<(), ProjectDuplicateError>
where
    C: ConnectionTrait,
{
    if targets.is_empty() {
        return Ok(());
    }
    let mut target_entity_id_case = CaseStatement::new();
    let mut spec_ids = Vec::with_capacity(targets.len());
    for target in targets {
        let target_entity_id = match target.target_entity_kind.as_str() {
            "canvas_node" => remap_derived_required_id(
                &maps.node_ids,
                target.old_target_entity_id,
                "generation_spec.target_entity_id",
            )?,
            "video_segment" => remap_derived_required_id(
                &maps.video_segment_ids,
                target.old_target_entity_id,
                "generation_spec.target_entity_id",
            )?,
            "assembly_timeline" => remap_derived_required_id(
                &maps.assembly_timeline_ids,
                target.old_target_entity_id,
                "generation_spec.target_entity_id",
            )?,
            "media_asset" => remap_derived_required_id(
                &maps.media_asset_ids,
                target.old_target_entity_id,
                "generation_spec.target_entity_id",
            )?,
            other => {
                return Err(ProjectDuplicateError::InvalidArgument(format!(
                    "自由视频项目生成意图暂不支持重映射 target_entity_kind={other}"
                )));
            }
        };
        target_entity_id_case = target_entity_id_case.case(
            generation_spec::Column::Id.eq(target.new_spec_id),
            Expr::value(Some(target_entity_id)),
        );
        spec_ids.push(target.new_spec_id);
    }
    let updated = generation_spec::Entity::update_many()
        .col_expr(
            generation_spec::Column::TargetEntityId,
            target_entity_id_case
                .finally(Expr::col(generation_spec::Column::TargetEntityId))
                .into(),
        )
        .col_expr(generation_spec::Column::UpdatedAt, Expr::value(now))
        .filter(generation_spec::Column::Id.is_in(spec_ids))
        .exec(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    if updated.rows_affected != targets.len() as u64 {
        return Err(ProjectDuplicateError::InvalidState {
            current: "复制后的生成意图目标数量与待回填记录不一致".to_string(),
            operation: "回填自由视频项目生成意图目标".to_string(),
        });
    }
    Ok(())
}

pub(in crate::project::duplicate::freeform) async fn update_freeform_generation_spec_metadata_record<
    C,
>(
    db: &C,
    spec_id_map: &HashMap<i64, i64>,
    maps: &FreeformCopyMaps,
    now: NaiveDateTime,
) -> Result<(), ProjectDuplicateError>
where
    C: ConnectionTrait,
{
    if spec_id_map.is_empty() {
        return Ok(());
    }
    let specs_by_id = generation_spec::Entity::find()
        .filter(generation_spec::Column::Id.is_in(spec_id_map.values().copied()))
        .all(db)
        .await
        .map_err(ProjectDuplicateError::Database)?
        .into_iter()
        .map(|spec| (spec.id, spec))
        .collect::<HashMap<_, _>>();
    let mut source_version_hash_case = CaseStatement::new();
    let mut params_json_case = CaseStatement::new();
    let mut spec_ids = Vec::with_capacity(spec_id_map.len());
    for new_spec_id in spec_id_map.values().copied() {
        let spec =
            specs_by_id
                .get(&new_spec_id)
                .ok_or_else(|| ProjectDuplicateError::InvalidState {
                    current: format!("复制后的生成意图不存在: {new_spec_id}"),
                    operation: "更新生成意图元数据".to_string(),
                })?;
        let params_json = remap_freeform_metadata(spec.params_json.clone(), maps)?;
        let source_version_hash = generation_spec::freeform_source_version_hash(&params_json)
            .ok_or_else(|| ProjectDuplicateError::InvalidState {
                current: format!(
                    "生成意图 {new_spec_id} 的 params_json.source_material 不是 JSON 对象"
                ),
                operation: "更新生成意图元数据".to_string(),
            })?;
        source_version_hash_case = source_version_hash_case.case(
            generation_spec::Column::Id.eq(new_spec_id),
            Expr::value(source_version_hash),
        );
        params_json_case = params_json_case.case(
            generation_spec::Column::Id.eq(new_spec_id),
            Expr::value(params_json).cast_as(Alias::new("json")),
        );
        spec_ids.push(new_spec_id);
    }
    let updated = generation_spec::Entity::update_many()
        .col_expr(
            generation_spec::Column::SourceVersionHash,
            source_version_hash_case
                .finally(Expr::col(generation_spec::Column::SourceVersionHash))
                .into(),
        )
        .col_expr(
            generation_spec::Column::ParamsJson,
            params_json_case
                .finally(Expr::col(generation_spec::Column::ParamsJson))
                .into(),
        )
        .col_expr(generation_spec::Column::UpdatedAt, Expr::value(now))
        .filter(generation_spec::Column::Id.is_in(spec_ids))
        .exec(db)
        .await
        .map_err(ProjectDuplicateError::Database)?;
    if updated.rows_affected != spec_id_map.len() as u64 {
        return Err(ProjectDuplicateError::InvalidState {
            current: "复制后的生成意图元数据数量与映射不一致".to_string(),
            operation: "更新自由视频项目生成意图元数据".to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use a4n_core::models::generation::generation_spec;
    use chrono::Utc;
    use sea_orm::{EntityTrait, Set};
    use serde_json::json;

    #[tokio::test]
    async fn batches_generation_spec_target_and_metadata_fixups() {
        let db = a4n_migration::test_support::connect_test_database().await;
        let source = crate::project::create_freeform_video_project_record(
            &db,
            1,
            crate::project::CreateFreeformVideoProjectParams {
                title: "回填生成意图源".to_string(),
                description: String::new(),
            },
        )
        .await
        .unwrap();
        let target = crate::project::create_freeform_video_project_record(
            &db,
            1,
            crate::project::CreateFreeformVideoProjectParams {
                title: "回填生成意图目标".to_string(),
                description: String::new(),
            },
        )
        .await
        .unwrap();
        let now = Utc::now().naive_utc();
        let rows = (0..20)
            .map(|index| {
                let params_json = json!({
                    "source_material": {"source_node_ids": [70_000 + index]}
                });
                generation_spec::ActiveModel {
                    project_id: Set(target.id),
                    root_project_id: Set(target.id),
                    source_context_kind: Set(Some(
                        generation_spec::SOURCE_CONTEXT_FREEFORM.to_string(),
                    )),
                    target_kind: Set("video".to_string()),
                    target_entity_kind: Set("media_asset".to_string()),
                    target_entity_id: Set(None),
                    status: Set("completed".to_string()),
                    source_version_hash: Set("stale".to_string()),
                    params_json: Set(params_json),
                    created_at: Set(now),
                    updated_at: Set(now),
                    ..Default::default()
                }
            })
            .collect::<Vec<_>>();
        let inserted = generation_spec::Entity::insert_many(rows)
            .exec_with_returning_many(&db)
            .await
            .unwrap();

        let mut maps = FreeformCopyMaps::default();
        let mut spec_id_map = HashMap::new();
        let mut targets = Vec::with_capacity(inserted.len());
        for (index, row) in inserted.iter().enumerate() {
            maps.media_asset_ids
                .insert(70_000 + index as i64, 80_000 + index as i64);
            maps.node_ids
                .insert(70_000 + index as i64, 90_000 + index as i64);
            spec_id_map.insert(50_000 + index as i64, row.id);
            targets.push(DeferredFreeformSpecTarget {
                new_spec_id: row.id,
                target_entity_kind: "media_asset".to_string(),
                old_target_entity_id: 70_000 + index as i64,
            });
        }

        update_freeform_generation_spec_deferred_targets_record(&db, &targets, &maps, now)
            .await
            .unwrap();
        update_freeform_generation_spec_metadata_record(&db, &spec_id_map, &maps, now)
            .await
            .unwrap();

        let stored = generation_spec::Entity::find()
            .filter(generation_spec::Column::ProjectId.eq(target.id))
            .all(&db)
            .await
            .unwrap();
        assert_eq!(stored.len(), 20);
        for (index, row) in stored.iter().enumerate() {
            assert_eq!(row.target_entity_id, Some(80_000 + index as i64));
            assert_eq!(
                row.params_json["source_material"]["source_node_ids"],
                json!([90_000 + index as i64])
            );
            assert_eq!(
                row.source_version_hash,
                generation_spec::freeform_source_version_hash(&row.params_json).unwrap()
            );
        }
        assert_ne!(source.id, target.id);
    }
}
