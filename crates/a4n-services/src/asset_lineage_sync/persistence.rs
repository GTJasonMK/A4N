use std::collections::BTreeMap;

use a4n_core::error::AppError;
use a4n_core::models::{
    generation::{generation_spec, generation_spec_media_ref, prompt_artifact},
    media::{media_asset, media_binding, media_provenance},
};
use chrono::{NaiveDateTime, Utc};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter,
    QueryOrder, Set,
};
use serde_json::Value;

use super::{AssetLineageBackfillSummary, FamilyLineageContext};

const RENDERER_VERSION: &str = "legacy_asset_lineage_sync_v1";

#[derive(Debug)]
pub(super) struct GenerationSpecUpsertInput {
    pub(super) canonical_key: String,
    pub(super) project_id: i64,
    pub(super) root_project_id: i64,
    pub(super) source_snapshot_id: i64,
    pub(super) source_chapter_id: Option<i64>,
    pub(super) source_snapshot_chapter_id: Option<i64>,
    pub(super) target_kind: String,
    pub(super) target_entity_kind: String,
    pub(super) target_entity_id: Option<i64>,
    pub(super) status: String,
    pub(super) source_version_hash: String,
    pub(super) params_json: Value,
    pub(super) created_at: NaiveDateTime,
    pub(super) updated_at: NaiveDateTime,
}

#[derive(Debug, Clone)]
pub(super) struct MediaUpsertInput {
    pub(super) canonical_key: String,
    pub(super) project_id: i64,
    pub(super) root_project_id: i64,
    pub(super) source_snapshot_id: Option<i64>,
    pub(super) source_chapter_id: Option<i64>,
    pub(super) source_snapshot_chapter_id: Option<i64>,
    pub(super) media_kind: String,
    pub(super) origin: String,
    pub(super) file_path: Option<String>,
    pub(super) thumbnail_path: Option<String>,
    pub(super) width: Option<i32>,
    pub(super) height: Option<i32>,
    pub(super) duration_secs: Option<f32>,
    pub(super) fps: Option<f32>,
    pub(super) file_size: Option<i64>,
    pub(super) status: String,
    pub(super) error_message: Option<String>,
    pub(super) generation_spec_id: Option<i64>,
    pub(super) prompt_artifact_id: Option<i64>,
    pub(super) provider_job_id: Option<String>,
    pub(super) source_media_asset_id: Option<i64>,
    pub(super) source_external_path: Option<String>,
    pub(super) imported_from_user: bool,
    pub(super) request_snapshot_json: Value,
    pub(super) created_at: NaiveDateTime,
}

pub(super) async fn upsert_generation_spec<C>(
    db: &C,
    family: &FamilyLineageContext,
    input: GenerationSpecUpsertInput,
    summary: &mut AssetLineageBackfillSummary,
) -> Result<generation_spec::Model, AppError>
where
    C: ConnectionTrait,
{
    let existing = find_generation_spec_by_key(db, family, &input.canonical_key).await?;
    if let Some(existing) = existing {
        let mut active: generation_spec::ActiveModel = existing.clone().into();
        let mut changed = false;
        set_if_changed(
            &mut active.project_id,
            existing.project_id,
            input.project_id,
            &mut changed,
        );
        set_if_changed(
            &mut active.root_project_id,
            existing.root_project_id,
            input.root_project_id,
            &mut changed,
        );
        set_if_changed(
            &mut active.source_snapshot_id,
            existing.source_snapshot_id,
            Some(input.source_snapshot_id),
            &mut changed,
        );
        set_if_changed(
            &mut active.source_chapter_id,
            existing.source_chapter_id,
            input.source_chapter_id,
            &mut changed,
        );
        set_if_changed(
            &mut active.source_snapshot_chapter_id,
            existing.source_snapshot_chapter_id,
            input.source_snapshot_chapter_id,
            &mut changed,
        );
        set_string_if_changed(
            &mut active.target_kind,
            &existing.target_kind,
            input.target_kind,
            &mut changed,
        );
        set_string_if_changed(
            &mut active.target_entity_kind,
            &existing.target_entity_kind,
            input.target_entity_kind,
            &mut changed,
        );
        set_if_changed(
            &mut active.target_entity_id,
            existing.target_entity_id,
            input.target_entity_id,
            &mut changed,
        );
        set_string_if_changed(
            &mut active.status,
            &existing.status,
            input.status,
            &mut changed,
        );
        set_string_if_changed(
            &mut active.source_version_hash,
            &existing.source_version_hash,
            input.source_version_hash,
            &mut changed,
        );
        set_value_if_changed(
            &mut active.params_json,
            &existing.params_json,
            input.params_json,
            &mut changed,
        );
        if changed {
            active.updated_at = Set(Utc::now().naive_utc());
            summary.updated_generation_specs += 1;
            return active.update(db).await.map_err(AppError::Database);
        }
        return Ok(existing);
    }

    let spec = generation_spec::ActiveModel {
        project_id: Set(input.project_id),
        root_project_id: Set(input.root_project_id),
        source_context_kind: Set(Some("snapshot_backed".to_string())),
        source_snapshot_id: Set(Some(input.source_snapshot_id)),
        source_chapter_id: Set(input.source_chapter_id),
        source_snapshot_chapter_id: Set(input.source_snapshot_chapter_id),
        target_kind: Set(input.target_kind),
        target_entity_kind: Set(input.target_entity_kind),
        target_entity_id: Set(input.target_entity_id),
        status: Set(input.status),
        source_version_hash: Set(input.source_version_hash),
        params_json: Set(input.params_json),
        created_at: Set(input.created_at),
        updated_at: Set(input.updated_at),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::Database)?;
    summary.created_generation_specs += 1;
    Ok(spec)
}

pub(super) async fn upsert_prompt_artifact<C>(
    db: &C,
    generation_spec_id: i64,
    prompt_kind: &str,
    prompt_text: &str,
    created_at: NaiveDateTime,
    updated_at: NaiveDateTime,
    summary: &mut AssetLineageBackfillSummary,
) -> Result<prompt_artifact::Model, AppError>
where
    C: ConnectionTrait,
{
    let existing = prompt_artifact::Entity::find()
        .filter(prompt_artifact::Column::GenerationSpecId.eq(generation_spec_id))
        .order_by_asc(prompt_artifact::Column::Id)
        .one(db)
        .await
        .map_err(AppError::Database)?;
    if let Some(existing) = existing {
        let mut active: prompt_artifact::ActiveModel = existing.clone().into();
        let mut changed = false;
        set_string_if_changed(
            &mut active.prompt_kind,
            &existing.prompt_kind,
            prompt_kind.to_string(),
            &mut changed,
        );
        set_string_if_changed(
            &mut active.prompt_text,
            &existing.prompt_text,
            prompt_text.to_string(),
            &mut changed,
        );
        set_string_if_changed(
            &mut active.renderer_version,
            &existing.renderer_version,
            RENDERER_VERSION.to_string(),
            &mut changed,
        );
        if changed {
            active.updated_at = Set(Utc::now().naive_utc());
            summary.updated_prompt_artifacts += 1;
            return active.update(db).await.map_err(AppError::Database);
        }
        return Ok(existing);
    }

    let artifact = prompt_artifact::ActiveModel {
        generation_spec_id: Set(generation_spec_id),
        prompt_kind: Set(prompt_kind.to_string()),
        prompt_text: Set(prompt_text.to_string()),
        renderer_version: Set(RENDERER_VERSION.to_string()),
        edited_by_user: Set(false),
        created_at: Set(created_at),
        updated_at: Set(updated_at),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::Database)?;
    summary.created_prompt_artifacts += 1;
    Ok(artifact)
}

pub(super) async fn upsert_media_asset<C>(
    db: &C,
    family: &FamilyLineageContext,
    input: MediaUpsertInput,
    summary: &mut AssetLineageBackfillSummary,
) -> Result<media_asset::Model, AppError>
where
    C: ConnectionTrait,
{
    let existing = find_media_asset_by_key(db, family, &input.canonical_key).await?;
    if let Some((existing_asset, existing_provenance)) = existing {
        let mut active: media_asset::ActiveModel = existing_asset.clone().into();
        let mut changed = false;
        set_if_changed(
            &mut active.project_id,
            existing_asset.project_id,
            input.project_id,
            &mut changed,
        );
        set_if_changed(
            &mut active.root_project_id,
            existing_asset.root_project_id,
            input.root_project_id,
            &mut changed,
        );
        set_if_changed(
            &mut active.source_snapshot_id,
            existing_asset.source_snapshot_id,
            input.source_snapshot_id,
            &mut changed,
        );
        set_if_changed(
            &mut active.source_chapter_id,
            existing_asset.source_chapter_id,
            input.source_chapter_id,
            &mut changed,
        );
        set_if_changed(
            &mut active.source_snapshot_chapter_id,
            existing_asset.source_snapshot_chapter_id,
            input.source_snapshot_chapter_id,
            &mut changed,
        );
        set_string_if_changed(
            &mut active.media_kind,
            &existing_asset.media_kind,
            input.media_kind.clone(),
            &mut changed,
        );
        set_string_if_changed(
            &mut active.origin,
            &existing_asset.origin,
            input.origin.clone(),
            &mut changed,
        );
        set_if_changed(
            &mut active.file_path,
            existing_asset.file_path.clone(),
            input.file_path.clone(),
            &mut changed,
        );
        set_if_changed(
            &mut active.thumbnail_path,
            existing_asset.thumbnail_path.clone(),
            input.thumbnail_path.clone(),
            &mut changed,
        );
        set_if_changed(
            &mut active.width,
            existing_asset.width,
            input.width,
            &mut changed,
        );
        set_if_changed(
            &mut active.height,
            existing_asset.height,
            input.height,
            &mut changed,
        );
        set_if_changed(
            &mut active.duration_secs,
            existing_asset.duration_secs,
            input.duration_secs,
            &mut changed,
        );
        set_if_changed(&mut active.fps, existing_asset.fps, input.fps, &mut changed);
        set_if_changed(
            &mut active.file_size,
            existing_asset.file_size,
            input.file_size,
            &mut changed,
        );
        set_string_if_changed(
            &mut active.status,
            &existing_asset.status,
            input.status.clone(),
            &mut changed,
        );
        set_if_changed(
            &mut active.error_message,
            existing_asset.error_message.clone(),
            input.error_message.clone(),
            &mut changed,
        );
        let asset = if changed {
            active.updated_at = Set(Utc::now().naive_utc());
            summary.updated_media_assets += 1;
            active.update(db).await.map_err(AppError::Database)?
        } else {
            existing_asset
        };
        update_media_provenance_if_needed(db, existing_provenance, &input, summary).await?;
        return Ok(asset);
    }

    let now = Utc::now().naive_utc();
    let asset = media_asset::ActiveModel {
        project_id: Set(input.project_id),
        root_project_id: Set(input.root_project_id),
        source_snapshot_id: Set(input.source_snapshot_id),
        source_chapter_id: Set(input.source_chapter_id),
        source_snapshot_chapter_id: Set(input.source_snapshot_chapter_id),
        media_kind: Set(input.media_kind),
        origin: Set(input.origin),
        file_path: Set(input.file_path),
        thumbnail_path: Set(input.thumbnail_path),
        width: Set(input.width),
        height: Set(input.height),
        duration_secs: Set(input.duration_secs),
        fps: Set(input.fps),
        file_size: Set(input.file_size),
        checksum: Set(None),
        status: Set(input.status),
        error_message: Set(input.error_message),
        created_at: Set(input.created_at),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::Database)?;
    media_provenance::ActiveModel {
        media_asset_id: Set(asset.id),
        generation_spec_id: Set(input.generation_spec_id),
        prompt_artifact_id: Set(input.prompt_artifact_id),
        provider_job_id: Set(input.provider_job_id),
        source_media_asset_id: Set(input.source_media_asset_id),
        source_external_path: Set(input.source_external_path),
        imported_from_user: Set(input.imported_from_user),
        request_snapshot_json: Set(Some(input.request_snapshot_json)),
        created_at: Set(now),
    }
    .insert(db)
    .await
    .map_err(AppError::Database)?;
    summary.created_media_assets += 1;
    summary.created_media_provenance += 1;
    Ok(asset)
}

async fn update_media_provenance_if_needed<C>(
    db: &C,
    existing: media_provenance::Model,
    input: &MediaUpsertInput,
    summary: &mut AssetLineageBackfillSummary,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let mut active: media_provenance::ActiveModel = existing.clone().into();
    let mut changed = false;
    set_if_changed(
        &mut active.generation_spec_id,
        existing.generation_spec_id,
        input.generation_spec_id,
        &mut changed,
    );
    set_if_changed(
        &mut active.prompt_artifact_id,
        existing.prompt_artifact_id,
        input.prompt_artifact_id,
        &mut changed,
    );
    set_if_changed(
        &mut active.provider_job_id,
        existing.provider_job_id,
        input.provider_job_id.clone(),
        &mut changed,
    );
    set_if_changed(
        &mut active.source_media_asset_id,
        existing.source_media_asset_id,
        input.source_media_asset_id,
        &mut changed,
    );
    set_if_changed(
        &mut active.source_external_path,
        existing.source_external_path,
        input.source_external_path.clone(),
        &mut changed,
    );
    set_if_changed(
        &mut active.imported_from_user,
        existing.imported_from_user,
        input.imported_from_user,
        &mut changed,
    );
    set_if_changed(
        &mut active.request_snapshot_json,
        existing.request_snapshot_json,
        Some(input.request_snapshot_json.clone()),
        &mut changed,
    );
    if changed {
        active.update(db).await.map_err(AppError::Database)?;
        summary.updated_media_provenance += 1;
    }
    Ok(())
}

pub(super) async fn sync_video_prompt_media_refs<C>(
    db: &C,
    generation_spec_id: i64,
    reference_image_ids_json: Option<&Value>,
    image_media_by_legacy_id: &BTreeMap<i64, i64>,
    summary: &mut AssetLineageBackfillSummary,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let Some(reference_ids) = reference_image_ids_json.and_then(Value::as_array) else {
        return Ok(());
    };
    for image_id in reference_ids.iter().filter_map(Value::as_i64) {
        if let Some(media_asset_id) = image_media_by_legacy_id.get(&image_id).copied() {
            ensure_generation_media_ref(
                db,
                generation_spec_id,
                media_asset_id,
                "reference",
                summary,
            )
            .await?;
        }
    }
    Ok(())
}

async fn ensure_generation_media_ref<C>(
    db: &C,
    generation_spec_id: i64,
    media_asset_id: i64,
    role: &str,
    summary: &mut AssetLineageBackfillSummary,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let exists = generation_spec_media_ref::Entity::find()
        .filter(generation_spec_media_ref::Column::GenerationSpecId.eq(generation_spec_id))
        .filter(generation_spec_media_ref::Column::MediaAssetId.eq(media_asset_id))
        .filter(generation_spec_media_ref::Column::Role.eq(role))
        .count(db)
        .await
        .map_err(AppError::Database)?;
    if exists > 0 {
        return Ok(());
    }
    generation_spec_media_ref::ActiveModel {
        generation_spec_id: Set(generation_spec_id),
        media_asset_id: Set(media_asset_id),
        role: Set(role.to_string()),
    }
    .insert(db)
    .await
    .map_err(AppError::Database)?;
    summary.created_generation_media_refs += 1;
    Ok(())
}

pub(super) async fn ensure_media_binding<C>(
    db: &C,
    media_asset_id: i64,
    target_kind: &str,
    target_id: i64,
    role: &str,
    summary: &mut AssetLineageBackfillSummary,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let exists = media_binding::Entity::find()
        .filter(media_binding::Column::MediaAssetId.eq(media_asset_id))
        .filter(media_binding::Column::TargetKind.eq(target_kind))
        .filter(media_binding::Column::TargetId.eq(target_id))
        .filter(media_binding::Column::Role.eq(role))
        .count(db)
        .await
        .map_err(AppError::Database)?;
    if exists > 0 {
        return Ok(());
    }
    media_binding::ActiveModel {
        media_asset_id: Set(media_asset_id),
        target_kind: Set(target_kind.to_string()),
        target_id: Set(target_id),
        role: Set(role.to_string()),
        created_at: Set(Utc::now().naive_utc()),
        ..Default::default()
    }
    .insert(db)
    .await
    .map_err(AppError::Database)?;
    summary.created_media_bindings += 1;
    Ok(())
}

async fn find_generation_spec_by_key<C>(
    db: &C,
    family: &FamilyLineageContext,
    canonical_key: &str,
) -> Result<Option<generation_spec::Model>, AppError>
where
    C: ConnectionTrait,
{
    let rows = generation_spec::Entity::find()
        .filter(generation_spec::Column::ProjectId.is_in(family.project_ids.iter().copied()))
        .order_by_asc(generation_spec::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    Ok(rows.into_iter().find(|row| {
        row.params_json.get("canonical_key").and_then(Value::as_str) == Some(canonical_key)
    }))
}

async fn find_media_asset_by_key<C>(
    db: &C,
    family: &FamilyLineageContext,
    canonical_key: &str,
) -> Result<Option<(media_asset::Model, media_provenance::Model)>, AppError>
where
    C: ConnectionTrait,
{
    let assets = media_asset::Entity::find()
        .filter(media_asset::Column::ProjectId.is_in(family.project_ids.iter().copied()))
        .order_by_asc(media_asset::Column::Id)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    if assets.is_empty() {
        return Ok(None);
    }
    let asset_by_id = assets
        .into_iter()
        .map(|asset| (asset.id, asset))
        .collect::<BTreeMap<_, _>>();
    let provenances = media_provenance::Entity::find()
        .filter(media_provenance::Column::MediaAssetId.is_in(asset_by_id.keys().copied()))
        .order_by_asc(media_provenance::Column::MediaAssetId)
        .all(db)
        .await
        .map_err(AppError::Database)?;
    for provenance in provenances {
        let matches = provenance
            .request_snapshot_json
            .as_ref()
            .and_then(|value| value.get("canonical_key"))
            .and_then(Value::as_str)
            == Some(canonical_key);
        if matches && let Some(asset) = asset_by_id.get(&provenance.media_asset_id) {
            return Ok(Some((asset.clone(), provenance)));
        }
    }
    Ok(None)
}

fn set_if_changed<T>(field: &mut sea_orm::ActiveValue<T>, current: T, next: T, changed: &mut bool)
where
    T: PartialEq + Into<sea_orm::Value>,
{
    if current != next {
        *field = Set(next);
        *changed = true;
    }
}

fn set_string_if_changed(
    field: &mut sea_orm::ActiveValue<String>,
    current: &str,
    next: String,
    changed: &mut bool,
) {
    if current != next {
        *field = Set(next);
        *changed = true;
    }
}

fn set_value_if_changed(
    field: &mut sea_orm::ActiveValue<Value>,
    current: &Value,
    next: Value,
    changed: &mut bool,
) {
    if current != &next {
        *field = Set(next);
        *changed = true;
    }
}
