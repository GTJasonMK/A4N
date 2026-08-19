use a4n_core::models::generation::{
    generation_spec, generation_spec_entity_ref, generation_spec_media_ref,
    generation_spec_text_anchor,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter};
use serde_json::Value;

use crate::generation::{GenerationDbError, validation::validate_positive_id};

use super::rules::normalize_media_ref_role;

pub(crate) async fn validate_required_prompt_refs<C>(
    db: &C,
    spec: &generation_spec::Model,
) -> Result<(), GenerationDbError>
where
    C: ConnectionTrait,
{
    validate_positive_id(spec.id, "generation_spec_id")?;
    let anchor_count = generation_spec_text_anchor::Entity::find()
        .filter(generation_spec_text_anchor::Column::GenerationSpecId.eq(spec.id))
        .count(db)
        .await?;
    if anchor_count == 0 {
        return Err(GenerationDbError::InvalidArgument(format!(
            "生成意图 {} 缺少必需文本锚点",
            spec.id
        )));
    }

    if matches!(
        spec.target_kind.as_str(),
        "character_material_image" | "object_material_image" | "scene_material_image"
    ) {
        let subject_count = generation_spec_entity_ref::Entity::find()
            .filter(generation_spec_entity_ref::Column::GenerationSpecId.eq(spec.id))
            .filter(generation_spec_entity_ref::Column::Role.eq("subject"))
            .count(db)
            .await?;
        if subject_count == 0 {
            return Err(GenerationDbError::InvalidArgument(format!(
                "素材生成意图 {} 缺少 subject 实体引用",
                spec.id
            )));
        }
    }

    if let Some(required_roles) = spec
        .params_json
        .get("required_media_roles")
        .and_then(Value::as_array)
    {
        for role in required_roles {
            let role = role.as_str().ok_or_else(|| {
                GenerationDbError::InvalidArgument(
                    "params_json.required_media_roles 必须是字符串数组".to_string(),
                )
            })?;
            let role = normalize_media_ref_role(role)?;
            let count = generation_spec_media_ref::Entity::find()
                .filter(generation_spec_media_ref::Column::GenerationSpecId.eq(spec.id))
                .filter(generation_spec_media_ref::Column::Role.eq(role))
                .count(db)
                .await?;
            if count == 0 {
                return Err(GenerationDbError::InvalidArgument(format!(
                    "生成意图 {} 缺少必需媒体引用: {}",
                    spec.id, role
                )));
            }
        }
    }
    Ok(())
}
