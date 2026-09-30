use crate::canvas::CanvasDbError;
use a4n_db_models::models::generation::{generation_spec, prompt_artifact};
use a4n_domain::canvas::{
    CanvasPromptArtifactRecord, CanvasPromptArtifactUpsertInput,
    FreeformCanvasGenerationSpecUpsertInput,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set,
};

const CANVAS_RENDERER_VERSION: &str = "canvas_workflow_v1";

pub async fn upsert_freeform_canvas_generation_spec_record<C>(
    db: &C,
    input: FreeformCanvasGenerationSpecUpsertInput,
) -> Result<generation_spec::Model, CanvasDbError>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    let source_version_hash = generation_spec::freeform_source_version_hash(&input.params_json)
        .ok_or_else(|| {
            CanvasDbError::InvalidArgument(
                "自由画布生成意图 params_json.source_material 必须是 JSON 对象".to_string(),
            )
        })?;
    let existing = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.eq(input.project_id))
        .filter(
            generation_spec::Column::SourceContextKind.eq(generation_spec::SOURCE_CONTEXT_FREEFORM),
        )
        .filter(generation_spec::Column::TargetEntityKind.eq(input.target_entity_kind.as_str()))
        .filter(generation_spec::Column::TargetEntityId.eq(input.target_entity_id))
        .order_by_asc(generation_spec::Column::Id)
        .one(db)
        .await?;

    if let Some(existing) = existing {
        let mut active: generation_spec::ActiveModel = existing.into();
        active.root_project_id = Set(input.project_id);
        active.source_context_kind =
            Set(Some(generation_spec::SOURCE_CONTEXT_FREEFORM.to_string()));
        active.source_snapshot_id = Set(None);
        active.source_chapter_id = Set(None);
        active.source_snapshot_chapter_id = Set(None);
        active.canvas_text_asset_id = Set(input.canvas_text_asset_id);
        active.target_kind = Set(input.target_kind);
        active.target_entity_kind = Set(input.target_entity_kind);
        active.target_entity_id = Set(Some(input.target_entity_id));
        active.status = Set(input.status);
        active.source_version_hash = Set(source_version_hash);
        active.params_json = Set(input.params_json);
        active.updated_at = Set(now);
        return Ok(active.update(db).await?);
    }

    Ok(generation_spec::ActiveModel {
        project_id: Set(input.project_id),
        root_project_id: Set(input.project_id),
        source_context_kind: Set(Some(generation_spec::SOURCE_CONTEXT_FREEFORM.to_string())),
        source_snapshot_id: Set(None),
        source_chapter_id: Set(None),
        source_snapshot_chapter_id: Set(None),
        canvas_text_asset_id: Set(input.canvas_text_asset_id),
        target_kind: Set(input.target_kind),
        target_entity_kind: Set(input.target_entity_kind),
        target_entity_id: Set(Some(input.target_entity_id)),
        status: Set(input.status),
        source_version_hash: Set(source_version_hash),
        params_json: Set(input.params_json),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?)
}

pub async fn mark_canvas_generation_spec_status_record<C>(
    db: &C,
    generation_spec_id: i64,
    status: &str,
) -> Result<(), CanvasDbError>
where
    C: ConnectionTrait,
{
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

pub async fn upsert_canvas_prompt_artifact_record<C>(
    db: &C,
    input: CanvasPromptArtifactUpsertInput,
) -> Result<CanvasPromptArtifactRecord, CanvasDbError>
where
    C: ConnectionTrait,
{
    let now = Utc::now().naive_utc();
    let existing = prompt_artifact::Entity::find()
        .filter(prompt_artifact::Column::GenerationSpecId.eq(input.generation_spec_id))
        .order_by_asc(prompt_artifact::Column::Id)
        .one(db)
        .await?;
    if let Some(existing) = existing {
        if existing.edited_by_user {
            return Ok(canvas_prompt_artifact_record(existing));
        }
        let mut active: prompt_artifact::ActiveModel = existing.into();
        active.prompt_kind = Set(input.prompt_kind);
        active.prompt_text = Set(input.prompt_text);
        active.renderer_version = Set(CANVAS_RENDERER_VERSION.to_string());
        active.edited_by_user = Set(false);
        active.updated_at = Set(now);
        return Ok(canvas_prompt_artifact_record(active.update(db).await?));
    }

    Ok(canvas_prompt_artifact_record(
        prompt_artifact::ActiveModel {
            generation_spec_id: Set(input.generation_spec_id),
            prompt_kind: Set(input.prompt_kind),
            prompt_text: Set(input.prompt_text),
            renderer_version: Set(CANVAS_RENDERER_VERSION.to_string()),
            edited_by_user: Set(false),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(db)
        .await?,
    ))
}

fn canvas_prompt_artifact_record(row: prompt_artifact::Model) -> CanvasPromptArtifactRecord {
    CanvasPromptArtifactRecord {
        id: row.id,
        generation_spec_id: row.generation_spec_id,
        prompt_kind: row.prompt_kind,
        prompt_text: row.prompt_text,
    }
}
