use a4n_core::models::{
    generation::{
        generation_run, generation_run_target, generation_spec, generation_spec_entity_ref,
        generation_spec_media_ref, generation_spec_text_anchor, llm_call_trace, prompt_artifact,
    },
    media::media_provenance,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait,
    QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait, sea_query::Expr,
};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

const RUN_STATUS_RUNNING: &str = "running";
const RUN_STATUS_COMPLETED: &str = "completed";
const RUN_STATUS_FAILED: &str = "failed";
const RUN_STATUS_CANCELLED: &str = "cancelled";
const RUN_STATUS_INTERRUPTED: &str = "interrupted";

mod validation;
use validation::*;
pub use validation::{
    canonical_json_bytes, canonical_json_sha256, canonical_json_text, verify_canonical_json_sha256,
};

mod types;
pub use types::*;
use types::{AttachGenerationRunTargetInput, CreateGenerationRunInput, INTERNAL_PROMPT_TOKENS};

mod runs;
pub use runs::*;

mod refs;
pub use refs::*;

mod prompts;
pub use prompts::*;

mod tasks;
pub use tasks::*;

mod task_lifecycle;
pub use task_lifecycle::*;

mod lifecycle_gate;
pub use lifecycle_gate::*;

#[cfg(test)]
mod tests;

pub(crate) async fn delete_generation_spec_records_by_ids<C>(
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
