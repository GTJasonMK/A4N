use super::*;

pub async fn mark_galgame_generation_spec_status_record(
    db: &DatabaseConnection,
    generation_spec_id: i64,
    status: &str,
) -> Result<(), GalgameGenerationRecordError> {
    validate_galgame_generation_id(generation_spec_id, "generation_spec_id")?;
    let status = require_galgame_generation_text(status, "status")?;
    generation_spec::ActiveModel {
        id: Set(generation_spec_id),
        status: Set(status.to_string()),
        updated_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .update(db)
    .await?;
    Ok(())
}

pub async fn galgame_prompt_artifact_by_generation_spec(
    db: &DatabaseConnection,
    generation_spec_id: i64,
) -> Result<Option<prompt_artifact::Model>, GalgameGenerationRecordError> {
    validate_galgame_generation_id(generation_spec_id, "generation_spec_id")?;
    prompt_artifact::Entity::find()
        .filter(prompt_artifact::Column::GenerationSpecId.eq(generation_spec_id))
        .order_by_asc(prompt_artifact::Column::Id)
        .one(db)
        .await
        .map_err(GalgameGenerationRecordError::Database)
}

pub async fn upsert_galgame_prompt_artifact_record(
    db: &DatabaseConnection,
    input: GalgamePromptArtifactUpsertInput,
) -> Result<prompt_artifact::Model, GalgameGenerationRecordError> {
    validate_galgame_generation_id(input.generation_spec_id, "generation_spec_id")?;
    require_galgame_generation_text(&input.prompt_kind, "prompt_kind")?;
    require_galgame_generation_text(&input.prompt_text, "prompt_text")?;
    require_galgame_generation_text(&input.renderer_version, "renderer_version")?;
    let existing = galgame_prompt_artifact_by_generation_spec(db, input.generation_spec_id).await?;
    if let Some(existing) = existing {
        if existing.edited_by_user {
            return Ok(existing);
        }
        let mut active: prompt_artifact::ActiveModel = existing.into();
        active.prompt_kind = Set(input.prompt_kind);
        active.prompt_text = Set(input.prompt_text);
        active.renderer_version = Set(input.renderer_version);
        active.edited_by_user = Set(false);
        active.updated_at = Set(Utc::now().naive_utc());
        return active
            .update(db)
            .await
            .map_err(GalgameGenerationRecordError::Database);
    }
    let now = Utc::now().naive_utc();
    prompt_artifact::ActiveModel {
        generation_spec_id: Set(input.generation_spec_id),
        prompt_kind: Set(input.prompt_kind),
        prompt_text: Set(input.prompt_text),
        renderer_version: Set(input.renderer_version),
        edited_by_user: Set(false),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(GalgameGenerationRecordError::Database)
}
