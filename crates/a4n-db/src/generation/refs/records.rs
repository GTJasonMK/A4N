use a4n_core::models::{
    generation::{
        generation_spec, generation_spec_entity_ref, generation_spec_media_ref,
        generation_spec_text_anchor,
    },
    media::media_asset,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, Set};
use std::collections::HashMap;

use crate::generation::{GenerationDbError, validation::validate_positive_id};

use super::rules::{
    normalize_entity_kind, normalize_entity_role, normalize_media_ref_role,
    normalize_text_anchor_role,
};

pub(crate) async fn insert_generation_text_anchor_refs<C>(
    db: &C,
    generation_spec_id: i64,
    refs: &[(i64, String)],
) -> Result<(), GenerationDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(generation_spec_id, "generation_spec_id")?;
    let rows = refs
        .iter()
        .map(|(text_anchor_id, role)| {
            validate_positive_id(*text_anchor_id, "text_anchor_id")?;
            let role = normalize_text_anchor_role(role)?;
            Ok(generation_spec_text_anchor::ActiveModel {
                generation_spec_id: Set(generation_spec_id),
                text_anchor_id: Set(*text_anchor_id),
                role: Set(role.to_string()),
            })
        })
        .collect::<Result<Vec<_>, GenerationDbError>>()?;
    if rows.is_empty() {
        return Ok(());
    }
    let inserted = generation_spec_text_anchor::Entity::insert_many(rows)
        .exec_without_returning(db)
        .await?;
    if inserted != refs.len() as u64 {
        return Err(GenerationDbError::InvalidArgument(format!(
            "文本锚点引用批量写入影响 {} 行，预期 {} 行",
            inserted,
            refs.len()
        )));
    }
    Ok(())
}

pub(crate) async fn insert_generation_entity_refs<C>(
    db: &C,
    generation_spec_id: i64,
    refs: &[(String, i64, String)],
) -> Result<(), GenerationDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(generation_spec_id, "generation_spec_id")?;
    let rows = refs
        .iter()
        .map(|(entity_kind, entity_id, role)| {
            validate_positive_id(*entity_id, "entity_id")?;
            let entity_kind = normalize_entity_kind(entity_kind)?;
            let role = normalize_entity_role(role)?;
            Ok(generation_spec_entity_ref::ActiveModel {
                generation_spec_id: Set(generation_spec_id),
                entity_kind: Set(entity_kind.to_string()),
                entity_id: Set(*entity_id),
                role: Set(role.to_string()),
            })
        })
        .collect::<Result<Vec<_>, GenerationDbError>>()?;
    if rows.is_empty() {
        return Ok(());
    }
    let inserted = generation_spec_entity_ref::Entity::insert_many(rows)
        .exec_without_returning(db)
        .await?;
    if inserted != refs.len() as u64 {
        return Err(GenerationDbError::InvalidArgument(format!(
            "实体引用批量写入影响 {} 行，预期 {} 行",
            inserted,
            refs.len()
        )));
    }
    Ok(())
}

pub(crate) async fn insert_generation_media_refs<C>(
    db: &C,
    spec: &generation_spec::Model,
    refs: &[(i64, String)],
) -> Result<(), GenerationDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(spec.id, "generation_spec_id")?;
    let mut normalized = Vec::with_capacity(refs.len());
    for (media_asset_id, role) in refs {
        validate_positive_id(*media_asset_id, "media_asset_id")?;
        normalized.push((*media_asset_id, normalize_media_ref_role(role)?.to_string()));
    }
    if normalized.is_empty() {
        return Ok(());
    }
    let assets_by_id = media_asset::Entity::find()
        .filter(media_asset::Column::Id.is_in(normalized.iter().map(|(asset_id, _)| *asset_id)))
        .all(db)
        .await?
        .into_iter()
        .map(|asset| (asset.id, asset))
        .collect::<HashMap<_, _>>();
    let rows = normalized
        .into_iter()
        .map(|(media_asset_id, role)| {
            let asset = assets_by_id.get(&media_asset_id).ok_or_else(|| {
                GenerationDbError::NotFound(format!("媒体资产不存在: {media_asset_id}"))
            })?;
            if asset.project_id != spec.project_id || asset.root_project_id != spec.root_project_id
            {
                return Err(GenerationDbError::InvalidArgument(format!(
                    "生成意图 {} 不能引用跨项目媒体资产 {}",
                    spec.id, asset.id
                )));
            }
            if asset.status != "completed" || asset.file_path.is_none() {
                return Err(GenerationDbError::InvalidArgument(format!(
                    "媒体引用 {} 不是可用的已完成资产",
                    asset.id
                )));
            }
            Ok(generation_spec_media_ref::ActiveModel {
                generation_spec_id: Set(spec.id),
                media_asset_id: Set(asset.id),
                role: Set(role),
            })
        })
        .collect::<Result<Vec<_>, GenerationDbError>>()?;
    let expected = rows.len() as u64;
    let inserted = generation_spec_media_ref::Entity::insert_many(rows)
        .exec_without_returning(db)
        .await?;
    if inserted != expected {
        return Err(GenerationDbError::InvalidArgument(format!(
            "媒体引用批量写入影响 {} 行，预期 {} 行",
            inserted, expected
        )));
    }
    Ok(())
}
