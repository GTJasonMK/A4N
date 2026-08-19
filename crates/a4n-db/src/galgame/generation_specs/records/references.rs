use crate::galgame::{GALGAME_SPEC_REF_INSERT_CHUNK_SIZE, GalgameGenerationSpecRefsRecordInput};
use a4n_core::models::generation::{generation_spec_entity_ref, generation_spec_text_anchor};
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, Set};

pub(super) async fn replace_galgame_generation_spec_ref_records<C>(
    db: &C,
    input: GalgameGenerationSpecRefsRecordInput,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    if !input.text_anchor_spec_ids.is_empty() {
        generation_spec_text_anchor::Entity::delete_many()
            .filter(
                generation_spec_text_anchor::Column::GenerationSpecId
                    .is_in(input.text_anchor_spec_ids.iter().copied()),
            )
            .exec(db)
            .await?;
        for chunk in input
            .text_anchor_refs
            .chunks(GALGAME_SPEC_REF_INSERT_CHUNK_SIZE)
        {
            generation_spec_text_anchor::Entity::insert_many(chunk.iter().map(|reference| {
                generation_spec_text_anchor::ActiveModel {
                    generation_spec_id: Set(reference.generation_spec_id),
                    text_anchor_id: Set(reference.text_anchor_id),
                    role: Set("context".to_string()),
                }
            }))
            .exec(db)
            .await?;
        }
    }

    if !input.entity_ref_spec_ids.is_empty() {
        generation_spec_entity_ref::Entity::delete_many()
            .filter(
                generation_spec_entity_ref::Column::GenerationSpecId
                    .is_in(input.entity_ref_spec_ids.iter().copied()),
            )
            .exec(db)
            .await?;
        for chunk in input.entity_refs.chunks(GALGAME_SPEC_REF_INSERT_CHUNK_SIZE) {
            generation_spec_entity_ref::Entity::insert_many(chunk.iter().map(|reference| {
                generation_spec_entity_ref::ActiveModel {
                    generation_spec_id: Set(reference.generation_spec_id),
                    entity_kind: Set("character".to_string()),
                    entity_id: Set(reference.profile_id),
                    role: Set("subject".to_string()),
                }
            }))
            .exec(db)
            .await?;
        }
    }
    Ok(())
}
