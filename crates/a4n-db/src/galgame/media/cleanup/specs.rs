use a4n_core::models::{
    generation::{
        generation_spec, generation_spec_entity_ref, generation_spec_media_ref,
        generation_spec_text_anchor, prompt_artifact,
    },
    media::media_provenance,
};
use sea_orm::sea_query::Expr;
use sea_orm::{ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter};

pub async fn delete_generation_spec_records_by_ids<C>(
    db: &C,
    generation_spec_ids: Vec<i64>,
    prompt_artifact_ids: Vec<i64>,
) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    if generation_spec_ids.is_empty() {
        return Ok(());
    }
    let mut provenance_update = media_provenance::Entity::update_many()
        .col_expr(
            media_provenance::Column::GenerationSpecId,
            Expr::value(Option::<i64>::None),
        )
        .filter(media_provenance::Column::GenerationSpecId.is_in(generation_spec_ids.clone()));
    if !prompt_artifact_ids.is_empty() {
        provenance_update = provenance_update.col_expr(
            media_provenance::Column::PromptArtifactId,
            Expr::value(Option::<i64>::None),
        );
    }
    provenance_update.exec(db).await?;
    if !prompt_artifact_ids.is_empty() {
        media_provenance::Entity::update_many()
            .col_expr(
                media_provenance::Column::PromptArtifactId,
                Expr::value(Option::<i64>::None),
            )
            .filter(media_provenance::Column::PromptArtifactId.is_in(prompt_artifact_ids))
            .exec(db)
            .await?;
    }
    generation_spec_media_ref::Entity::delete_many()
        .filter(
            generation_spec_media_ref::Column::GenerationSpecId.is_in(generation_spec_ids.clone()),
        )
        .exec(db)
        .await?;
    generation_spec_text_anchor::Entity::delete_many()
        .filter(
            generation_spec_text_anchor::Column::GenerationSpecId
                .is_in(generation_spec_ids.clone()),
        )
        .exec(db)
        .await?;
    generation_spec_entity_ref::Entity::delete_many()
        .filter(
            generation_spec_entity_ref::Column::GenerationSpecId.is_in(generation_spec_ids.clone()),
        )
        .exec(db)
        .await?;
    prompt_artifact::Entity::delete_many()
        .filter(prompt_artifact::Column::GenerationSpecId.is_in(generation_spec_ids.clone()))
        .exec(db)
        .await?;
    generation_spec::Entity::delete_many()
        .filter(generation_spec::Column::Id.is_in(generation_spec_ids))
        .exec(db)
        .await?;
    Ok(())
}
