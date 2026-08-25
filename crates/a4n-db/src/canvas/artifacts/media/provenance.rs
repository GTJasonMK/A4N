use super::super::types::CanvasVideoMediaSource;
use crate::canvas::CanvasDbError;
use a4n_core::models::{
    ai::generated_image,
    generation::generation_spec,
    media::{media_asset, media_provenance},
};
use a4n_domain::{EXTERNAL_IMPORT_MODEL, GeneratedMediaReference};
use sea_orm::sea_query::{CaseStatement, Expr};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbBackend, EntityTrait, FromQueryResult,
    QueryFilter, QueryOrder, QuerySelect, Set,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

struct CanvasMediaProvenanceUpsertInput {
    generation_spec_id: Option<i64>,
    provider_job_id: Option<String>,
    source_external_path: String,
    imported_from_user: bool,
    request_snapshot_json: Value,
}

#[derive(Debug, FromQueryResult)]
struct CanvasGenerationSpecIdentity {
    id: i64,
    project_id: i64,
    root_project_id: i64,
    source_snapshot_id: Option<i64>,
    source_chapter_id: Option<i64>,
    source_snapshot_chapter_id: Option<i64>,
}

pub(super) async fn upsert_canvas_image_media_provenance<C>(
    db: &C,
    asset: &media_asset::Model,
    image: &generated_image::Model,
    generation_spec_id: Option<i64>,
) -> Result<(), CanvasDbError>
where
    C: ConnectionTrait,
{
    upsert_canvas_media_provenance(
        db,
        asset,
        CanvasMediaProvenanceUpsertInput {
            generation_spec_id,
            provider_job_id: None,
            source_external_path: GeneratedMediaReference::Image(image.id).to_string(),
            imported_from_user: image.model == EXTERNAL_IMPORT_MODEL,
            request_snapshot_json: serde_json::json!({
                "source": "video_canvas",
                "generated_image_id": image.id,
                "image_type": image.image_type,
                "metadata": image.metadata_json,
            }),
        },
    )
    .await
}

pub(super) async fn upsert_canvas_video_media_provenance<C>(
    db: &C,
    asset: &media_asset::Model,
    video: &CanvasVideoMediaSource,
    generation_spec_id: Option<i64>,
) -> Result<(), CanvasDbError>
where
    C: ConnectionTrait,
{
    upsert_canvas_media_provenance(
        db,
        asset,
        canvas_video_media_provenance_input(video, generation_spec_id),
    )
    .await
}

pub(super) async fn upsert_canvas_video_media_provenance_records<C>(
    db: &C,
    assets: &[media_asset::Model],
    videos: &[CanvasVideoMediaSource],
    generation_spec_ids: &[Option<i64>],
) -> Result<(), CanvasDbError>
where
    C: ConnectionTrait,
{
    if assets.len() != videos.len() || assets.len() != generation_spec_ids.len() {
        return Err(CanvasDbError::InvalidArgument(format!(
            "画布视频媒体来源与资产数量不一致: videos={}, assets={}, generation_specs={}",
            videos.len(),
            assets.len(),
            generation_spec_ids.len()
        )));
    }
    if assets.is_empty() {
        return Ok(());
    }

    let mut prepared_inputs = Vec::with_capacity(assets.len());
    let mut generation_spec_id_set = HashSet::new();
    for ((asset, video), generation_spec_id) in assets.iter().zip(videos).zip(generation_spec_ids) {
        let input = canvas_video_media_provenance_input(video, *generation_spec_id);
        if let Some(generation_spec_id) = input.generation_spec_id {
            generation_spec_id_set.insert(generation_spec_id);
        }
        prepared_inputs.push((asset, input));
    }
    let generation_specs_by_id =
        load_canvas_generation_spec_identities(db, &generation_spec_id_set)
            .await?
            .into_iter()
            .map(|spec| (spec.id, spec))
            .collect::<HashMap<_, _>>();
    let mut inputs_by_asset_id = BTreeMap::new();
    for (asset, input) in prepared_inputs {
        if let Some(generation_spec_id) = input.generation_spec_id {
            let spec = generation_specs_by_id
                .get(&generation_spec_id)
                .ok_or_else(|| {
                    CanvasDbError::NotFound(format!("生成意图不存在: {generation_spec_id}"))
                })?;
            validate_canvas_media_generation_spec(asset, spec)?;
        }
        inputs_by_asset_id.insert(asset.id, input);
    }
    let asset_ids = inputs_by_asset_id.keys().copied().collect::<Vec<_>>();
    let mut existing_ids = BTreeSet::new();
    let mut query = media_provenance::Entity::find()
        .select_only()
        .column(media_provenance::Column::MediaAssetId)
        .filter(media_provenance::Column::MediaAssetId.is_in(asset_ids.clone()))
        .order_by_asc(media_provenance::Column::MediaAssetId);
    if db.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    for asset_id in query.into_tuple::<i64>().all(db).await? {
        existing_ids.insert(asset_id);
    }

    let now = chrono::Utc::now().naive_utc();
    let mut inserts = Vec::new();
    for (asset_id, input) in &inputs_by_asset_id {
        if existing_ids.contains(asset_id) {
            continue;
        }
        inserts.push(media_provenance::ActiveModel {
            media_asset_id: Set(*asset_id),
            generation_spec_id: Set(input.generation_spec_id),
            prompt_artifact_id: Set(None),
            provider_job_id: Set(input.provider_job_id.clone()),
            source_media_asset_id: Set(None),
            source_external_path: Set(Some(input.source_external_path.clone())),
            imported_from_user: Set(input.imported_from_user),
            request_snapshot_json: Set(Some(input.request_snapshot_json.clone())),
            created_at: Set(now),
        });
    }
    if !inserts.is_empty() {
        let expected = inserts.len();
        let inserted = media_provenance::Entity::insert_many(inserts)
            .exec_without_returning(db)
            .await?;
        if inserted != expected as u64 {
            return Err(CanvasDbError::InvalidArgument(format!(
                "画布媒体来源批量插入影响 {} 行，预期 {} 行",
                inserted, expected
            )));
        }
    }

    if existing_ids.is_empty() {
        return Ok(());
    }
    let mut generation_spec_id_case = CaseStatement::new();
    let mut provider_job_id_case = CaseStatement::new();
    let mut source_external_path_case = CaseStatement::new();
    let mut imported_from_user_case = CaseStatement::new();
    let mut request_snapshot_json_case = CaseStatement::new();
    let mut created_at_case = CaseStatement::new();
    for asset_id in &existing_ids {
        let input = inputs_by_asset_id
            .get(asset_id)
            .expect("existing canvas provenance has a prepared input");
        let condition = media_provenance::Column::MediaAssetId.eq(*asset_id);
        generation_spec_id_case =
            generation_spec_id_case.case(condition.clone(), Expr::value(input.generation_spec_id));
        provider_job_id_case = provider_job_id_case.case(
            condition.clone(),
            Expr::value(input.provider_job_id.clone()),
        );
        source_external_path_case = source_external_path_case.case(
            condition.clone(),
            Expr::value(Some(input.source_external_path.clone())),
        );
        imported_from_user_case =
            imported_from_user_case.case(condition.clone(), Expr::value(input.imported_from_user));
        request_snapshot_json_case = request_snapshot_json_case.case(
            condition.clone(),
            Expr::value(Some(input.request_snapshot_json.clone())).cast_as("json"),
        );
        created_at_case = created_at_case.case(condition, Expr::value(now));
    }
    let updated = media_provenance::Entity::update_many()
        .col_expr(
            media_provenance::Column::GenerationSpecId,
            generation_spec_id_case
                .finally(Expr::col(media_provenance::Column::GenerationSpecId))
                .into(),
        )
        .col_expr(
            media_provenance::Column::PromptArtifactId,
            Expr::value(None::<i64>),
        )
        .col_expr(
            media_provenance::Column::ProviderJobId,
            provider_job_id_case
                .finally(Expr::col(media_provenance::Column::ProviderJobId))
                .into(),
        )
        .col_expr(
            media_provenance::Column::SourceMediaAssetId,
            Expr::value(None::<i64>),
        )
        .col_expr(
            media_provenance::Column::SourceExternalPath,
            source_external_path_case
                .finally(Expr::col(media_provenance::Column::SourceExternalPath))
                .into(),
        )
        .col_expr(
            media_provenance::Column::ImportedFromUser,
            imported_from_user_case
                .finally(Expr::col(media_provenance::Column::ImportedFromUser))
                .into(),
        )
        .col_expr(
            media_provenance::Column::RequestSnapshotJson,
            request_snapshot_json_case
                .finally(Expr::col(media_provenance::Column::RequestSnapshotJson))
                .into(),
        )
        .col_expr(
            media_provenance::Column::CreatedAt,
            created_at_case
                .finally(Expr::col(media_provenance::Column::CreatedAt))
                .into(),
        )
        .filter(media_provenance::Column::MediaAssetId.is_in(existing_ids.iter().copied()))
        .exec(db)
        .await?;
    if updated.rows_affected != existing_ids.len() as u64 {
        return Err(CanvasDbError::InvalidArgument(format!(
            "画布媒体来源批量更新影响 {} 行，预期 {} 行",
            updated.rows_affected,
            existing_ids.len()
        )));
    }
    Ok(())
}

fn canvas_video_media_provenance_input(
    video: &CanvasVideoMediaSource,
    generation_spec_id: Option<i64>,
) -> CanvasMediaProvenanceUpsertInput {
    CanvasMediaProvenanceUpsertInput {
        generation_spec_id,
        provider_job_id: video.provider_job_id.clone(),
        source_external_path: GeneratedMediaReference::Video(video.id).to_string(),
        imported_from_user: video.model == EXTERNAL_IMPORT_MODEL,
        request_snapshot_json: serde_json::json!({
            "source": "video_canvas",
            "generated_video_id": video.id,
            "video_type": video.video_type,
            "workflow_mode": video.workflow_mode,
            "metadata": video.metadata_json,
        }),
    }
}

async fn upsert_canvas_media_provenance<C>(
    db: &C,
    asset: &media_asset::Model,
    input: CanvasMediaProvenanceUpsertInput,
) -> Result<(), CanvasDbError>
where
    C: ConnectionTrait,
{
    if let Some(generation_spec_id) = input.generation_spec_id {
        let spec = load_canvas_generation_spec_identity(db, generation_spec_id).await?;
        validate_canvas_media_generation_spec(asset, &spec)?;
    }
    let existing = canvas_media_provenance_for_update(db, asset.id).await?;
    let model = media_provenance::ActiveModel {
        media_asset_id: Set(asset.id),
        generation_spec_id: Set(input.generation_spec_id),
        prompt_artifact_id: Set(None),
        provider_job_id: Set(input.provider_job_id),
        source_media_asset_id: Set(None),
        source_external_path: Set(Some(input.source_external_path)),
        imported_from_user: Set(input.imported_from_user),
        request_snapshot_json: Set(Some(input.request_snapshot_json)),
        created_at: Set(chrono::Utc::now().naive_utc()),
    };
    if existing.is_some() {
        model.update(db).await?;
    } else {
        model.insert(db).await?;
    }
    Ok(())
}

async fn load_canvas_generation_spec_identity<C>(
    db: &C,
    generation_spec_id: i64,
) -> Result<CanvasGenerationSpecIdentity, CanvasDbError>
where
    C: ConnectionTrait,
{
    let mut query = generation_spec::Entity::find_by_id(generation_spec_id);
    if db.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    let spec = query
        .select_only()
        .column(generation_spec::Column::Id)
        .column(generation_spec::Column::ProjectId)
        .column(generation_spec::Column::RootProjectId)
        .column(generation_spec::Column::SourceSnapshotId)
        .column(generation_spec::Column::SourceChapterId)
        .column(generation_spec::Column::SourceSnapshotChapterId)
        .into_model::<CanvasGenerationSpecIdentity>()
        .one(db)
        .await?
        .ok_or_else(|| CanvasDbError::NotFound(format!("生成意图不存在: {generation_spec_id}")))?;
    Ok(spec)
}

async fn load_canvas_generation_spec_identities<C>(
    db: &C,
    generation_spec_ids: &HashSet<i64>,
) -> Result<Vec<CanvasGenerationSpecIdentity>, CanvasDbError>
where
    C: ConnectionTrait,
{
    if generation_spec_ids.is_empty() {
        return Ok(Vec::new());
    }
    let mut query = generation_spec::Entity::find()
        .select_only()
        .column(generation_spec::Column::Id)
        .column(generation_spec::Column::ProjectId)
        .column(generation_spec::Column::RootProjectId)
        .column(generation_spec::Column::SourceSnapshotId)
        .column(generation_spec::Column::SourceChapterId)
        .column(generation_spec::Column::SourceSnapshotChapterId)
        .filter(generation_spec::Column::Id.is_in(generation_spec_ids.iter().copied()));
    if db.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    Ok(query
        .into_model::<CanvasGenerationSpecIdentity>()
        .all(db)
        .await?)
}

fn validate_canvas_media_generation_spec(
    asset: &media_asset::Model,
    spec: &CanvasGenerationSpecIdentity,
) -> Result<(), CanvasDbError> {
    if spec.project_id != asset.project_id || spec.root_project_id != asset.root_project_id {
        return Err(CanvasDbError::InvalidArgument(format!(
            "画布媒体来源生成意图 {} 不属于媒体资产项目族",
            spec.id
        )));
    }
    match asset.source_snapshot_id {
        Some(source_snapshot_id) if spec.source_snapshot_id != Some(source_snapshot_id) => {
            return Err(CanvasDbError::InvalidArgument(format!(
                "画布媒体来源生成意图 {} 的 source_snapshot_id 与媒体资产不一致",
                spec.id
            )));
        }
        None if spec.source_snapshot_id.is_some() || spec.source_snapshot_chapter_id.is_some() => {
            return Err(CanvasDbError::InvalidArgument(format!(
                "freeform 画布媒体资产不能引用带源快照上下文的生成意图 {}",
                spec.id
            )));
        }
        _ => {}
    }
    if let Some(source_snapshot_chapter_id) = asset.source_snapshot_chapter_id
        && spec.source_snapshot_chapter_id != Some(source_snapshot_chapter_id)
    {
        return Err(CanvasDbError::InvalidArgument(format!(
            "画布媒体来源生成意图 {} 的 source_snapshot_chapter_id 与媒体资产不一致",
            spec.id
        )));
    }
    if let Some(source_chapter_id) = asset.source_chapter_id
        && spec.source_chapter_id != Some(source_chapter_id)
    {
        return Err(CanvasDbError::InvalidArgument(format!(
            "画布媒体来源生成意图 {} 的 source_chapter_id 与媒体资产不一致",
            spec.id
        )));
    }
    Ok(())
}

async fn canvas_media_provenance_for_update<C>(
    db: &C,
    media_asset_id: i64,
) -> Result<Option<media_provenance::Model>, CanvasDbError>
where
    C: ConnectionTrait,
{
    let mut query = media_provenance::Entity::find_by_id(media_asset_id);
    if db.get_database_backend() != DbBackend::Sqlite {
        query = query.lock_exclusive();
    }
    Ok(query.one(db).await?)
}
