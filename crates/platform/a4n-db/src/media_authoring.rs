//! Versioned project artifacts. The domain owner validates its document before
//! saving; this boundary owns lineage, source integrity and concurrent writes.
use sea_orm::ConnectionTrait;

use a4n_db_models::models::project::{
    media_authoring_document as document, novel_project, project_source_snapshot,
    source_snapshot_chapter, standalone_comic_text_source,
};
use a4n_db_models::models::{
    chapter::{chapter_manga_prompt, manga_pipeline_artifact},
    galgame::{galgame_choice, galgame_line, galgame_scene, galgame_story_graph},
};
use a4n_domain::manga::{
    MangaPageGenerationLayoutV3, MangaPageGenerationLayoutV4, MangaPagePromptSourceSummary,
};
use a4n_domain::media_authoring::{
    AdaptationPlanV1, AdaptationTarget, AuthoringDocumentDeletionDependency,
    AuthoringDocumentDeletionImpact, AuthoringDocumentInputs, AuthoringDocumentKind,
    AuthoringDocumentRecord, AuthoringDocumentRef, AuthoringDocumentStatus,
    AuthoringGalgameScriptDeletion, AuthoringMangaPageDeletion, AuthoringSourceRef,
};
use a4n_domain::project::ProjectRecord;
use a4n_domain::shooting_script::ChapterShootingScriptReferenceRecord;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, DbErr, EntityTrait, QueryFilter, QueryOrder,
    QuerySelect, Set, sea_query::Expr,
};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};

pub const AUTHORING_READ_BATCH_SIZE: usize = 64;

/// Keeps batch readers able to parse and validate each document in input order.
#[derive(Clone)]
pub struct UnparsedAuthoringDocument(document::Model);

impl UnparsedAuthoringDocument {
    pub fn parse(self) -> Result<AuthoringDocumentRecord, AuthoringDocumentError> {
        parse_record(self.0)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AuthoringDocumentError {
    #[error("{0}")]
    InvalidArgument(String),
    #[error("{0}")]
    InvalidState(String),
    #[error("媒体创作文档不存在: {0}")]
    NotFound(i64),
    #[error("媒体创作文档版本冲突: {0}")]
    RevisionConflict(i64),
    #[error("删除影响范围在确认后发生变化，请重新预览并确认")]
    DeletionImpactChanged,
    #[error(transparent)]
    Database(#[from] DbErr),
}

pub struct SaveAuthoringDocument {
    pub project_id: i64,
    pub source_snapshot_id: i64,
    pub source_chapter_id: Option<i64>,
    pub kind: AuthoringDocumentKind,
    pub record_id: Option<i64>,
    pub expected_revision: Option<i32>,
    pub inputs: AuthoringDocumentInputs,
    pub document: Value,
}

pub enum AuthoringArtifactSource {
    MangaPage {
        prompt_id: i64,
        chapter_id: i64,
        page_number: i32,
        source: MangaPagePromptSourceSummary,
        page_plan_ref: Option<AuthoringDocumentRef>,
    },
    GalgameScript {
        script_id: i64,
        source_snapshot_id: i64,
        branch_plan_id: Option<i64>,
        scene_ids: Vec<i64>,
        scene_plan_ids: Vec<Option<i64>>,
    },
}

pub async fn authoring_artifact_sources<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    target: a4n_domain::media_authoring::AdaptationTarget,
    chapter_id: Option<i64>,
) -> Result<Vec<AuthoringArtifactSource>, AuthoringDocumentError> {
    use a4n_db_models::models::{
        chapter::chapter_manga_prompt,
        galgame::{galgame_scene, galgame_script},
    };
    match target {
        a4n_domain::media_authoring::AdaptationTarget::Manga => {
            let mut query = chapter_manga_prompt::Entity::find()
                .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id));
            if let Some(chapter_id) = chapter_id {
                query = query.filter(chapter_manga_prompt::Column::ChapterId.eq(chapter_id));
            }
            let rows = query.all(db).await?;
            rows.into_iter()
                .map(|row| {
                    let layout = row.layout.ok_or_else(|| {
                        AuthoringDocumentError::InvalidState(format!("漫画页 {} 缺少布局", row.id))
                    })?;
                    let page_number = u32::try_from(row.page_number)
                        .ok()
                        .filter(|number| *number > 0)
                        .ok_or_else(|| {
                            AuthoringDocumentError::InvalidState(format!(
                                "漫画页 {} 的 page_number 无效",
                                row.id
                            ))
                        })?;
                    let (source, page_plan_ref) =
                        match layout.get("schema_version").and_then(Value::as_str) {
                            Some("manga_page_generation_v4") => {
                                let layout: MangaPageGenerationLayoutV4 =
                                    serde_json::from_value(layout).map_err(|error| {
                                        AuthoringDocumentError::InvalidState(format!(
                                            "漫画页 {} 的 v4 layout 无效: {error}",
                                            row.id
                                        ))
                                    })?;
                                layout.validate(page_number).map_err(|error| {
                                    AuthoringDocumentError::InvalidState(format!(
                                        "漫画页 {} 的 v4 layout 无效: {error}",
                                        row.id
                                    ))
                                })?;
                                (layout.source, layout.page_plan_ref)
                            }
                            Some("manga_page_generation_v3") => {
                                let layout: MangaPageGenerationLayoutV3 =
                                    serde_json::from_value(layout).map_err(|error| {
                                        AuthoringDocumentError::InvalidState(format!(
                                            "漫画页 {} 的 v3 layout 无效: {error}",
                                            row.id
                                        ))
                                    })?;
                                layout.validate(page_number).map_err(|error| {
                                    AuthoringDocumentError::InvalidState(format!(
                                        "漫画页 {} 的 v3 layout 无效: {error}",
                                        row.id
                                    ))
                                })?;
                                (layout.source, layout.page_plan_ref)
                            }
                            schema => {
                                return Err(AuthoringDocumentError::InvalidState(format!(
                                    "漫画页 {} 的 layout schema_version 不支持: {}",
                                    row.id,
                                    schema.unwrap_or("<missing>")
                                )));
                            }
                        };
                    Ok(AuthoringArtifactSource::MangaPage {
                        prompt_id: row.id,
                        chapter_id: row.chapter_id,
                        page_number: row.page_number,
                        source,
                        page_plan_ref,
                    })
                })
                .collect()
        }
        a4n_domain::media_authoring::AdaptationTarget::Galgame => {
            let Some(script) = galgame_script::Entity::find()
                .filter(galgame_script::Column::ProjectId.eq(project_id))
                .one(db)
                .await?
            else {
                return Ok(Vec::new());
            };
            let scenes = galgame_scene::Entity::find()
                .filter(galgame_scene::Column::ScriptId.eq(script.id))
                .all(db)
                .await?;
            Ok(vec![AuthoringArtifactSource::GalgameScript {
                script_id: script.id,
                source_snapshot_id: script.source_snapshot_id,
                branch_plan_id: script.branch_plan_id,
                scene_ids: scenes.iter().map(|scene| scene.id).collect(),
                scene_plan_ids: scenes
                    .iter()
                    .map(|scene| scene.adaptation_plan_id)
                    .collect(),
            }])
        }
    }
}

pub(crate) async fn authoring_documents_for_projects<C: ConnectionTrait>(
    db: &C,
    project_ids: &[i64],
) -> Result<Vec<document::Model>, DbErr> {
    document::Entity::find()
        .filter(document::Column::ProjectId.is_in(project_ids.to_vec()))
        .order_by_asc(document::Column::Id)
        .all(db)
        .await
}

pub(crate) async fn authoring_receipts_for_projects<C: ConnectionTrait>(
    db: &C,
    project_ids: &[i64],
) -> Result<Vec<a4n_db_models::models::project::media_authoring_migration_receipt::Model>, DbErr> {
    use a4n_db_models::models::project::media_authoring_migration_receipt as receipt;
    receipt::Entity::find()
        .filter(receipt::Column::ProjectId.is_in(project_ids.to_vec()))
        .order_by_asc(receipt::Column::Kind)
        .order_by_asc(receipt::Column::OldId)
        .order_by_asc(receipt::Column::NewId)
        .all(db)
        .await
}

pub async fn authoring_document<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    record_id: i64,
) -> Result<AuthoringDocumentRecord, AuthoringDocumentError> {
    positive(project_id, "project_id")?;
    positive(record_id, "record_id")?;
    let row = document::Entity::find_by_id(record_id)
        .filter(document::Column::ProjectId.eq(project_id))
        .one(db)
        .await?
        .ok_or(AuthoringDocumentError::NotFound(record_id))?;
    parse_record(row)
}

/// Reads one bounded batch while leaving per-row errors for the caller to handle in order.
pub async fn authoring_document_rows_by_ids<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    record_ids: &[i64],
) -> Result<
    Vec<Result<Option<UnparsedAuthoringDocument>, AuthoringDocumentError>>,
    AuthoringDocumentError,
> {
    if record_ids.is_empty() {
        return Ok(Vec::new());
    }
    validate_authoring_read_batch_size(record_ids.len())?;
    positive(project_id, "project_id")?;
    positive(record_ids[0], "record_id")?;
    let rows = document::Entity::find()
        .filter(document::Column::ProjectId.eq(project_id))
        .filter(document::Column::Id.is_in(record_ids.iter().copied()))
        .all(db)
        .await?;
    let mut by_id = rows
        .into_iter()
        .map(|row| (row.id, row))
        .collect::<HashMap<_, _>>();
    let selected = take_unparsed_documents_in_order(record_ids, &mut by_id);
    Ok(record_ids
        .iter()
        .zip(selected)
        .map(|(id, row)| match positive(*id, "record_id") {
            Ok(()) => Ok(row),
            Err(error) => Err(error),
        })
        .collect())
}

pub async fn current_authoring_document<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    source_snapshot_id: i64,
    source_chapter_id: Option<i64>,
    kind: AuthoringDocumentKind,
    status: AuthoringDocumentStatus,
) -> Result<Option<AuthoringDocumentRecord>, AuthoringDocumentError> {
    document::Entity::find()
        .filter(scope(
            project_id,
            source_snapshot_id,
            source_chapter_id,
            kind,
        )?)
        .filter(document::Column::Status.eq(status.as_str()))
        .one(db)
        .await?
        .map(parse_record)
        .transpose()
}

/// Reads current draft/adopted documents for one source-chapter batch.
/// Draft rows keep priority, and returned rows remain unparsed for ordered caller validation.
pub async fn current_authoring_document_rows_by_source_chapters<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    source_snapshot_id: i64,
    kind: AuthoringDocumentKind,
    source_chapter_ids: &[i64],
) -> Result<
    Vec<Result<Option<UnparsedAuthoringDocument>, AuthoringDocumentError>>,
    AuthoringDocumentError,
> {
    if source_chapter_ids.is_empty() {
        return Ok(Vec::new());
    }
    validate_authoring_read_batch_size(source_chapter_ids.len())?;
    positive(project_id, "project_id")?;
    positive(source_snapshot_id, "source_snapshot_id")?;
    positive(source_chapter_ids[0], "source_chapter_id")?;
    let draft_rows = document::Entity::find()
        .filter(document::Column::ProjectId.eq(project_id))
        .filter(document::Column::SourceSnapshotId.eq(source_snapshot_id))
        .filter(document::Column::SourceChapterId.is_in(source_chapter_ids.iter().copied()))
        .filter(document::Column::Kind.eq(kind.as_str()))
        .filter(document::Column::Status.eq(AuthoringDocumentStatus::Draft.as_str()))
        .all(db)
        .await?;
    let mut by_chapter = draft_rows
        .into_iter()
        .filter_map(|row| row.source_chapter_id.map(|chapter_id| (chapter_id, row)))
        .collect::<HashMap<_, _>>();
    let adopted_chapter_ids = source_chapter_ids
        .iter()
        .copied()
        .filter(|chapter_id| !by_chapter.contains_key(chapter_id))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    if !adopted_chapter_ids.is_empty() {
        let adopted_rows = document::Entity::find()
            .filter(document::Column::ProjectId.eq(project_id))
            .filter(document::Column::SourceSnapshotId.eq(source_snapshot_id))
            .filter(document::Column::SourceChapterId.is_in(adopted_chapter_ids))
            .filter(document::Column::Kind.eq(kind.as_str()))
            .filter(document::Column::Status.eq(AuthoringDocumentStatus::Adopted.as_str()))
            .all(db)
            .await?;
        by_chapter = prefer_adopted_fallback(by_chapter, adopted_rows);
    }
    let selected = take_unparsed_documents_in_order(source_chapter_ids, &mut by_chapter);
    Ok(source_chapter_ids
        .iter()
        .zip(selected)
        .map(|(id, row)| match positive(*id, "source_chapter_id") {
            Ok(()) => Ok(row),
            Err(error) => Err(error),
        })
        .collect())
}

/// Reads only adopted authoring plans for one bounded source-chapter batch.
pub async fn adopted_authoring_document_rows_by_source_chapters<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    source_snapshot_id: i64,
    kind: AuthoringDocumentKind,
    source_chapter_ids: &[i64],
) -> Result<
    Vec<Result<Option<UnparsedAuthoringDocument>, AuthoringDocumentError>>,
    AuthoringDocumentError,
> {
    if source_chapter_ids.is_empty() {
        return Ok(Vec::new());
    }
    validate_authoring_read_batch_size(source_chapter_ids.len())?;
    positive(project_id, "project_id")?;
    positive(source_snapshot_id, "source_snapshot_id")?;
    positive(source_chapter_ids[0], "source_chapter_id")?;
    let rows = document::Entity::find()
        .filter(document::Column::ProjectId.eq(project_id))
        .filter(document::Column::SourceSnapshotId.eq(source_snapshot_id))
        .filter(document::Column::SourceChapterId.is_in(source_chapter_ids.iter().copied()))
        .filter(document::Column::Kind.eq(kind.as_str()))
        .filter(document::Column::Status.eq(AuthoringDocumentStatus::Adopted.as_str()))
        .all(db)
        .await?
        .into_iter()
        .filter_map(|row| row.source_chapter_id.map(|chapter_id| (chapter_id, row)))
        .collect::<HashMap<_, _>>();
    let mut rows = rows;
    let selected = take_unparsed_documents_in_order(source_chapter_ids, &mut rows);
    Ok(source_chapter_ids
        .iter()
        .zip(selected)
        .map(|(id, row)| match positive(*id, "source_chapter_id") {
            Ok(()) => Ok(row),
            Err(error) => Err(error),
        })
        .collect())
}

fn validate_authoring_read_batch_size(size: usize) -> Result<(), AuthoringDocumentError> {
    if size > AUTHORING_READ_BATCH_SIZE {
        return Err(AuthoringDocumentError::InvalidArgument(format!(
            "创作文档批量读取超过 {AUTHORING_READ_BATCH_SIZE} 条上限"
        )));
    }
    Ok(())
}

fn prefer_adopted_fallback(
    mut drafts: HashMap<i64, document::Model>,
    adopted_rows: Vec<document::Model>,
) -> HashMap<i64, document::Model> {
    for row in adopted_rows {
        if let Some(chapter_id) = row.source_chapter_id {
            drafts.entry(chapter_id).or_insert(row);
        }
    }
    drafts
}

fn take_unparsed_documents_in_order(
    ids: &[i64],
    rows: &mut HashMap<i64, document::Model>,
) -> Vec<Option<UnparsedAuthoringDocument>> {
    let mut first_positions = HashMap::<i64, usize>::new();
    let mut result = Vec::<Option<UnparsedAuthoringDocument>>::with_capacity(ids.len());
    for id in ids {
        let candidate = if let Some(position) = first_positions.get(id) {
            result[*position].clone()
        } else {
            first_positions.insert(*id, result.len());
            rows.remove(id).map(UnparsedAuthoringDocument)
        };
        result.push(candidate);
    }
    result
}

pub async fn list_authoring_documents<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    chapter_id: Option<i64>,
) -> Result<Vec<AuthoringDocumentRecord>, AuthoringDocumentError> {
    positive(project_id, "project_id")?;
    let mut query = document::Entity::find().filter(document::Column::ProjectId.eq(project_id));
    if let Some(id) = chapter_id {
        positive(id, "chapter_id")?;
        query = query.filter(document::Column::SourceChapterId.eq(id));
    }
    query
        .order_by_desc(document::Column::Id)
        .all(db)
        .await?
        .into_iter()
        .map(parse_record)
        .collect()
}

pub async fn editable_authoring_document<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    snapshot_id: i64,
    chapter_id: Option<i64>,
    kind: AuthoringDocumentKind,
) -> Result<Option<AuthoringDocumentRecord>, AuthoringDocumentError> {
    document::Entity::find()
        .filter(scope(project_id, snapshot_id, chapter_id, kind)?)
        .filter(document::Column::Status.is_in(["draft", "adopted"]))
        .order_by_desc(document::Column::Status)
        .one(db)
        .await?
        .map(parse_record)
        .transpose()
}

pub async fn save_authoring_document(
    db: &impl crate::DbExecutor,
    input: SaveAuthoringDocument,
) -> Result<AuthoringDocumentRecord, AuthoringDocumentError> {
    let lineage = scope(
        input.project_id,
        input.source_snapshot_id,
        input.source_chapter_id,
        input.kind,
    )?;
    if input.record_id.is_some_and(|id| {
        input
            .inputs
            .dependencies
            .iter()
            .any(|dependency| dependency.id == id)
    }) {
        return Err(AuthoringDocumentError::InvalidArgument(
            "创作文档不能依赖自身".into(),
        ));
    }
    if input.record_id.is_some() != input.expected_revision.is_some() {
        return Err(AuthoringDocumentError::InvalidArgument(
            "record_id 与 expected_revision 必须同时指定".into(),
        ));
    }
    let tx = db.begin().await?;
    validate_owner_and_sources(
        &tx,
        input.project_id,
        input.source_snapshot_id,
        input.kind,
        input.source_chapter_id,
        &input.inputs,
    )
    .await?;
    let inputs_json = serde_json::to_value(&input.inputs).map_err(invalid_json)?;
    let hash = crate::generation::canonical_json_sha256(&(&inputs_json, &input.document))
        .map_err(invalid_json)?;
    let existing = document::Entity::find()
        .filter(lineage.clone())
        .filter(document::Column::Status.eq("draft"))
        .lock_exclusive()
        .one(&tx)
        .await?;
    let now = Utc::now().naive_utc();
    let row = if let Some(existing) = existing {
        if input.record_id != Some(existing.id)
            || input.expected_revision != Some(existing.revision)
        {
            return Err(AuthoringDocumentError::RevisionConflict(existing.id));
        }
        if existing.document_sha256 == hash {
            existing
        } else {
            let mut active: document::ActiveModel = existing.clone().into();
            active.revision = Set(next(existing.revision)?);
            active.inputs_json = Set(inputs_json);
            active.document_json = Set(input.document);
            active.document_sha256 = Set(hash);
            active.updated_at = Set(now);
            active.update(&tx).await?
        }
    } else {
        if let Some(id) = input.record_id {
            return Err(AuthoringDocumentError::RevisionConflict(id));
        }
        let previous = document::Entity::find()
            .filter(lineage)
            .order_by_desc(document::Column::Version)
            .one(&tx)
            .await?;
        let version = previous
            .map(|row| next(row.version))
            .transpose()?
            .unwrap_or(1);
        document::ActiveModel {
            project_id: Set(input.project_id),
            source_snapshot_id: Set(input.source_snapshot_id),
            source_chapter_id: Set(input.source_chapter_id),
            kind: Set(input.kind.as_str().into()),
            version: Set(version),
            revision: Set(1),
            status: Set("draft".into()),
            inputs_json: Set(inputs_json),
            document_json: Set(input.document),
            document_sha256: Set(hash),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(&tx)
        .await?
    };
    let result = parse_record(row)?;
    tx.commit().await?;
    Ok(result)
}

pub async fn adopt_authoring_document(
    db: &impl crate::DbExecutor,
    project_id: i64,
    record_id: i64,
    expected_revision: i32,
) -> Result<AuthoringDocumentRecord, AuthoringDocumentError> {
    let tx = db.begin().await?;
    let current = authoring_document(&tx, project_id, record_id).await?;
    validate_owner_and_sources(
        &tx,
        project_id,
        current.source_snapshot_id,
        current.kind,
        current.source_chapter_id,
        &current.inputs,
    )
    .await?;
    let current = authoring_document(&tx, project_id, record_id).await?;
    if current.revision != expected_revision {
        return Err(AuthoringDocumentError::RevisionConflict(record_id));
    }
    if current.status == AuthoringDocumentStatus::Adopted {
        tx.commit().await?;
        return Ok(current);
    }
    if current.status != AuthoringDocumentStatus::Draft {
        return Err(AuthoringDocumentError::InvalidState(
            "只能采用草稿版本".into(),
        ));
    }
    let now = Utc::now().naive_utc();
    document::Entity::update_many()
        .filter(scope(
            project_id,
            current.source_snapshot_id,
            current.source_chapter_id,
            current.kind,
        )?)
        .filter(document::Column::Status.eq("adopted"))
        .col_expr(document::Column::Status, Expr::value("superseded"))
        .col_expr(document::Column::UpdatedAt, Expr::value(now))
        .exec(&tx)
        .await?;
    let updated = document::Entity::update_many()
        .filter(document::Column::Id.eq(record_id))
        .filter(document::Column::ProjectId.eq(project_id))
        .filter(document::Column::Revision.eq(expected_revision))
        .filter(document::Column::Status.eq("draft"))
        .col_expr(document::Column::Status, Expr::value("adopted"))
        .col_expr(document::Column::UpdatedAt, Expr::value(now))
        .exec(&tx)
        .await?;
    if updated.rows_affected != 1 {
        return Err(AuthoringDocumentError::RevisionConflict(record_id));
    }
    let result = authoring_document(&tx, project_id, record_id).await?;
    tx.commit().await?;
    Ok(result)
}

pub async fn delete_authoring_document(
    db: &impl crate::DbExecutor,
    project_id: i64,
    record_id: i64,
    kind: AuthoringDocumentKind,
    expected_revision: i32,
    expected_impact_fingerprint: &str,
) -> Result<(), AuthoringDocumentError> {
    validate_delete_identity(project_id, record_id, expected_revision)?;
    if expected_impact_fingerprint.len() != 64
        || !expected_impact_fingerprint
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(AuthoringDocumentError::InvalidArgument(
            "expected_impact_fingerprint 必须是 SHA-256".into(),
        ));
    }

    let tx = db.begin().await?;
    let current = locked_delete_record(&tx, project_id, record_id, kind, expected_revision).await?;
    let plan = authoring_deletion_plan(&tx, &current).await?;
    if plan.impact.fingerprint != expected_impact_fingerprint {
        return Err(AuthoringDocumentError::DeletionImpactChanged);
    }

    let mut generated_image_paths = Vec::new();
    for page in &plan.manga_pages {
        crate::generation::delete_generation_run_targets_for(
            &tx,
            "chapter_manga_prompt",
            &page.prompt_ids,
        )
        .await
        .map_err(generation_deletion_error)?;
        generated_image_paths.extend(
            crate::video::delete_manga_page_image_records_in(
                &tx,
                project_id,
                page.chapter_id,
                page.page_number,
                None,
            )
            .await
            .map_err(video_deletion_error)?,
        );
        chapter_manga_prompt::Entity::delete_many()
            .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
            .filter(chapter_manga_prompt::Column::ChapterId.eq(page.chapter_id))
            .filter(chapter_manga_prompt::Column::PageNumber.eq(page.page_number))
            .exec(&tx)
            .await?;
        manga_pipeline_artifact::Entity::delete_many()
            .filter(manga_pipeline_artifact::Column::ProjectId.eq(project_id))
            .filter(manga_pipeline_artifact::Column::ChapterId.eq(page.chapter_id))
            .filter(manga_pipeline_artifact::Column::PageNumber.eq(page.page_number))
            .exec(&tx)
            .await?;
    }

    if plan.galgame_script.is_some() {
        crate::galgame::delete_galgame_target_generation_media_graph_record(&tx, project_id)
            .await?;
        crate::galgame::delete_galgame_script_records_for_project(&tx, project_id).await?;
        crate::galgame::delete_galgame_story_graph_revisions(&tx, project_id).await?;
    }

    let deleted = document::Entity::delete_many()
        .filter(document::Column::ProjectId.eq(project_id))
        .filter(document::Column::Id.is_in(plan.document_ids.clone()))
        .exec(&tx)
        .await?;
    if usize::try_from(deleted.rows_affected).ok() != Some(plan.document_ids.len()) {
        return Err(AuthoringDocumentError::RevisionConflict(record_id));
    }
    if !generated_image_paths.is_empty() {
        crate::media::schedule_unreferenced_media_cleanup(
            &tx,
            project_id,
            &generated_image_paths,
            format!(
                "media-authoring-delete:{project_id}:{record_id}:{expected_impact_fingerprint}"
            ),
        )
        .await?;
    }
    tx.commit().await?;
    Ok(())
}

pub async fn preview_delete_authoring_document(
    db: &impl crate::DbExecutor,
    project_id: i64,
    record_id: i64,
    kind: AuthoringDocumentKind,
    expected_revision: i32,
) -> Result<AuthoringDocumentDeletionImpact, AuthoringDocumentError> {
    validate_delete_identity(project_id, record_id, expected_revision)?;
    let tx = db.begin().await?;
    let current = locked_delete_record(&tx, project_id, record_id, kind, expected_revision).await?;
    let impact = authoring_deletion_plan(&tx, &current).await?.impact;
    tx.commit().await?;
    Ok(impact)
}

struct MangaPageDeleteTarget {
    chapter_id: i64,
    page_number: i32,
    prompt_ids: Vec<i64>,
    image_ids: Vec<i64>,
}

struct GalgameScriptDeleteTarget {
    script_id: i64,
    scene_ids: Vec<i64>,
    line_ids: Vec<i64>,
    choice_ids: Vec<i64>,
    story_graph_revision_ids: Vec<i64>,
}

struct AuthoringDeletionPlan {
    document_ids: Vec<i64>,
    manga_pages: Vec<MangaPageDeleteTarget>,
    galgame_script: Option<GalgameScriptDeleteTarget>,
    impact: AuthoringDocumentDeletionImpact,
}

async fn locked_delete_record<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    record_id: i64,
    kind: AuthoringDocumentKind,
    expected_revision: i32,
) -> Result<AuthoringDocumentRecord, AuthoringDocumentError> {
    novel_project::Entity::find_by_id(project_id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| AuthoringDocumentError::InvalidState("目标项目不存在".into()))?;
    let row = document::Entity::find_by_id(record_id)
        .filter(document::Column::ProjectId.eq(project_id))
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or(AuthoringDocumentError::NotFound(record_id))?;
    let current = parse_record(row)?;
    if current.kind != kind {
        return Err(AuthoringDocumentError::InvalidArgument(
            "文档种类与删除请求不匹配".into(),
        ));
    }
    if current.revision != expected_revision {
        return Err(AuthoringDocumentError::RevisionConflict(record_id));
    }
    Ok(current)
}

fn validate_delete_identity(
    project_id: i64,
    record_id: i64,
    expected_revision: i32,
) -> Result<(), AuthoringDocumentError> {
    positive(project_id, "project_id")?;
    positive(record_id, "record_id")?;
    if expected_revision <= 0 {
        return Err(AuthoringDocumentError::InvalidArgument(
            "expected_revision 必须大于 0".into(),
        ));
    }
    Ok(())
}

async fn authoring_deletion_plan<C: ConnectionTrait>(
    db: &C,
    current: &AuthoringDocumentRecord,
) -> Result<AuthoringDeletionPlan, AuthoringDocumentError> {
    let records = document::Entity::find()
        .filter(document::Column::ProjectId.eq(current.project_id))
        .order_by_asc(document::Column::Id)
        .all(db)
        .await?
        .into_iter()
        .map(parse_record)
        .collect::<Result<Vec<_>, _>>()?;

    let mut document_ids = BTreeSet::from([current.id]);
    loop {
        let before = document_ids.len();
        for record in &records {
            if !document_ids.contains(&record.id)
                && record
                    .inputs
                    .dependencies
                    .iter()
                    .any(|dependency| document_ids.contains(&dependency.id))
            {
                document_ids.insert(record.id);
            }
        }
        if document_ids.len() == before {
            break;
        }
    }
    let mut targets = Vec::new();
    for record in records
        .iter()
        .filter(|record| document_ids.contains(&record.id))
    {
        let target = authoring_target(record)?;
        if !targets.contains(&target) {
            targets.push(target);
        }
    }

    let mut pages = BTreeMap::<(i64, i32), BTreeSet<i64>>::new();
    let mut galgame_script_id = None;
    for target in targets {
        for artifact in authoring_artifact_sources(db, current.project_id, target, None).await? {
            match artifact {
                AuthoringArtifactSource::MangaPage {
                    prompt_id,
                    chapter_id,
                    page_number,
                    source,
                    page_plan_ref,
                } if document_ids.contains(&source.adaptation_plan_id)
                    || page_plan_ref
                        .as_ref()
                        .is_some_and(|reference| document_ids.contains(&reference.id)) =>
                {
                    pages
                        .entry((chapter_id, page_number))
                        .or_default()
                        .insert(prompt_id);
                }
                AuthoringArtifactSource::GalgameScript {
                    script_id,
                    branch_plan_id,
                    scene_plan_ids,
                    ..
                } if branch_plan_id.is_some_and(|id| document_ids.contains(&id))
                    || scene_plan_ids
                        .iter()
                        .flatten()
                        .any(|id| document_ids.contains(id)) =>
                {
                    galgame_script_id = Some(script_id);
                }
                _ => {}
            }
        }
    }

    let mut manga_pages = Vec::with_capacity(pages.len());
    for ((chapter_id, page_number), prompt_ids) in pages {
        let image_ids = crate::video::manga_page_image_ids_for_deletion(
            db,
            current.project_id,
            chapter_id,
            page_number,
        )
        .await
        .map_err(video_deletion_error)?;
        manga_pages.push(MangaPageDeleteTarget {
            chapter_id,
            page_number,
            prompt_ids: prompt_ids.into_iter().collect(),
            image_ids,
        });
    }
    let galgame_script = if let Some(script_id) = galgame_script_id {
        let scene_ids = galgame_scene::Entity::find()
            .select_only()
            .column(galgame_scene::Column::Id)
            .filter(galgame_scene::Column::ScriptId.eq(script_id))
            .order_by_asc(galgame_scene::Column::Id)
            .into_tuple::<i64>()
            .all(db)
            .await?;
        let line_ids = if scene_ids.is_empty() {
            Vec::new()
        } else {
            galgame_line::Entity::find()
                .select_only()
                .column(galgame_line::Column::Id)
                .filter(galgame_line::Column::SceneId.is_in(scene_ids.clone()))
                .order_by_asc(galgame_line::Column::Id)
                .into_tuple::<i64>()
                .all(db)
                .await?
        };
        let choice_ids = if line_ids.is_empty() {
            Vec::new()
        } else {
            galgame_choice::Entity::find()
                .select_only()
                .column(galgame_choice::Column::Id)
                .filter(galgame_choice::Column::LineId.is_in(line_ids.clone()))
                .order_by_asc(galgame_choice::Column::Id)
                .into_tuple::<i64>()
                .all(db)
                .await?
        };
        let story_graph_revision_ids = galgame_story_graph::Entity::find()
            .select_only()
            .column(galgame_story_graph::Column::Id)
            .filter(galgame_story_graph::Column::ProjectId.eq(current.project_id))
            .order_by_asc(galgame_story_graph::Column::Revision)
            .into_tuple::<i64>()
            .all(db)
            .await?;
        Some(GalgameScriptDeleteTarget {
            script_id,
            scene_ids,
            line_ids,
            choice_ids,
            story_graph_revision_ids,
        })
    } else {
        None
    };

    let dependent_documents = records
        .iter()
        .filter(|record| record.id != current.id && document_ids.contains(&record.id))
        .map(|record| AuthoringDocumentDeletionDependency {
            id: record.id,
            kind: record.kind,
        })
        .collect::<Vec<_>>();
    let page_impact = manga_pages
        .iter()
        .map(|page| AuthoringMangaPageDeletion {
            chapter_id: page.chapter_id,
            page_number: page.page_number,
            image_count: page.image_ids.len(),
        })
        .collect::<Vec<_>>();
    let galgame_impact = galgame_script
        .as_ref()
        .map(|script| AuthoringGalgameScriptDeletion {
            scene_count: script.scene_ids.len(),
            line_count: script.line_ids.len(),
            choice_count: script.choice_ids.len(),
            story_graph_revision_count: script.story_graph_revision_ids.len(),
        });
    let document_ids = document_ids.into_iter().collect::<Vec<_>>();
    let fingerprint_payload = serde_json::json!({
        "record_id": current.id,
        "revision": current.revision,
        "document_ids": document_ids,
        "manga_pages": manga_pages.iter().map(|page| {
            (
                page.chapter_id,
                page.page_number,
                &page.prompt_ids,
                &page.image_ids,
            )
        }).collect::<Vec<_>>(),
        "galgame_script": galgame_script.as_ref().map(|script| {
            (
                script.script_id,
                &script.scene_ids,
                &script.line_ids,
                &script.choice_ids,
                &script.story_graph_revision_ids,
            )
        }),
    });
    let fingerprint =
        crate::generation::canonical_json_sha256(&fingerprint_payload).map_err(invalid_json)?;

    Ok(AuthoringDeletionPlan {
        document_ids,
        manga_pages,
        galgame_script,
        impact: AuthoringDocumentDeletionImpact {
            dependent_documents,
            manga_pages: page_impact,
            galgame_script: galgame_impact,
            fingerprint,
        },
    })
}

fn authoring_target(
    record: &AuthoringDocumentRecord,
) -> Result<AdaptationTarget, AuthoringDocumentError> {
    match record.kind {
        AuthoringDocumentKind::AdaptationPlan => Ok(serde_json::from_value::<AdaptationPlanV1>(
            record.document.clone(),
        )
        .map_err(invalid_json)?
        .target),
        AuthoringDocumentKind::GalgameBranchPlan => Ok(AdaptationTarget::Galgame),
        AuthoringDocumentKind::MangaPagePlan => Ok(AdaptationTarget::Manga),
    }
}

fn generation_deletion_error(
    error: crate::generation::GenerationDbError,
) -> AuthoringDocumentError {
    match error {
        crate::generation::GenerationDbError::Database(error) => {
            AuthoringDocumentError::Database(error)
        }
        error => AuthoringDocumentError::InvalidState(error.to_string()),
    }
}

fn video_deletion_error(error: crate::video::VideoDbError) -> AuthoringDocumentError {
    match error {
        crate::video::VideoDbError::Database(error) => AuthoringDocumentError::Database(error),
        error => AuthoringDocumentError::InvalidState(error.to_string()),
    }
}

fn scope(
    project_id: i64,
    snapshot_id: i64,
    chapter_id: Option<i64>,
    kind: AuthoringDocumentKind,
) -> Result<Condition, AuthoringDocumentError> {
    positive(project_id, "project_id")?;
    positive(snapshot_id, "source_snapshot_id")?;
    let condition = Condition::all()
        .add(document::Column::ProjectId.eq(project_id))
        .add(document::Column::SourceSnapshotId.eq(snapshot_id))
        .add(document::Column::Kind.eq(kind.as_str()));
    Ok(match chapter_id {
        Some(id) => {
            positive(id, "source_chapter_id")?;
            condition.add(document::Column::SourceChapterId.eq(id))
        }
        None => condition.add(document::Column::SourceChapterId.is_null()),
    })
}

async fn validate_owner_and_sources<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    snapshot_id: i64,
    kind: AuthoringDocumentKind,
    chapter_id: Option<i64>,
    inputs: &AuthoringDocumentInputs,
) -> Result<(), AuthoringDocumentError> {
    let project = novel_project::Entity::find_by_id(project_id)
        .lock_exclusive()
        .one(db)
        .await?
        .ok_or_else(|| AuthoringDocumentError::InvalidArgument("目标项目不存在".into()))?;
    let accepted = match kind {
        AuthoringDocumentKind::AdaptationPlan => {
            matches!(project.project_kind.as_str(), "comic" | "galgame")
        }
        AuthoringDocumentKind::GalgameBranchPlan => project.project_kind == "galgame",
        AuthoringDocumentKind::MangaPagePlan => project.project_kind == "comic",
    };
    if project.lifecycle_gate != "active" {
        return Err(AuthoringDocumentError::InvalidState(format!(
            "项目正在执行 {}，暂不能保存创作计划",
            project.lifecycle_gate
        )));
    }
    let standalone_comic = project.project_kind == "comic"
        && project.root_project_id.is_none()
        && project.source_snapshot_id.is_none();
    let expected_root_project_id = project.root_project_id.unwrap_or(project.id);
    if !accepted
        || (!standalone_comic
            && (project.root_project_id.is_none() || project.source_snapshot_id.is_none()))
    {
        return Err(AuthoringDocumentError::InvalidArgument(
            "文档类型与项目来源不匹配".into(),
        ));
    }
    if inputs.sources.is_empty()
        || (kind != AuthoringDocumentKind::GalgameBranchPlan
            && (inputs.sources.len() != 1
                || chapter_id != Some(inputs.sources[0].source_chapter_id)))
        || (kind == AuthoringDocumentKind::GalgameBranchPlan && chapter_id.is_some())
    {
        return Err(AuthoringDocumentError::InvalidArgument(
            "创作文档章节范围不一致".into(),
        ));
    }
    let snapshot = project_source_snapshot::Entity::find_by_id(snapshot_id)
        .one(db)
        .await?
        .ok_or_else(|| AuthoringDocumentError::InvalidArgument("创作文档基准快照不存在".into()))?;
    if snapshot.root_project_id != expected_root_project_id
        || (!standalone_comic && project.source_snapshot_id != Some(snapshot_id))
        || (kind != AuthoringDocumentKind::GalgameBranchPlan
            && inputs.sources[0].source_snapshot_id != snapshot_id)
    {
        return Err(AuthoringDocumentError::InvalidArgument(
            "创作文档基准快照不匹配".into(),
        ));
    }
    validate_source_references(
        db,
        project.id,
        expected_root_project_id,
        standalone_comic,
        &inputs.sources,
    )
    .await?;
    validate_adopted_authoring_dependencies(db, project_id, &inputs.dependencies).await?;
    if kind == AuthoringDocumentKind::AdaptationPlan && !inputs.dependencies.is_empty() {
        return Err(AuthoringDocumentError::InvalidArgument(
            "改编计划不能依赖其他本地阶段".into(),
        ));
    }
    Ok(())
}

async fn validate_source_references<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    expected_root_project_id: i64,
    standalone_comic: bool,
    source_refs: &[AuthoringSourceRef],
) -> Result<(), AuthoringDocumentError> {
    let mut seen = BTreeSet::new();
    for chunk in source_refs.chunks(AUTHORING_READ_BATCH_SIZE) {
        let mut rows = source_snapshot_chapter::Entity::find()
            .filter(
                source_snapshot_chapter::Column::Id
                    .is_in(chunk.iter().map(|source| source.source_snapshot_chapter_id)),
            )
            .all(db)
            .await?
            .into_iter()
            .map(|row| (row.id, row))
            .collect::<HashMap<_, _>>();

        for source in chunk {
            take_valid_source_row(expected_root_project_id, source, &mut rows, &mut seen)?;
            if standalone_comic {
                let mapped_source = standalone_comic_text_source::Entity::find()
                    .filter(standalone_comic_text_source::Column::ProjectId.eq(project_id))
                    .filter(
                        standalone_comic_text_source::Column::ChapterId
                            .eq(source.source_chapter_id),
                    )
                    .filter(
                        standalone_comic_text_source::Column::SourceSnapshotId
                            .eq(source.source_snapshot_id),
                    )
                    .filter(
                        standalone_comic_text_source::Column::SourceSnapshotChapterId
                            .eq(source.source_snapshot_chapter_id),
                    )
                    .one(db)
                    .await?;
                if mapped_source.is_none() {
                    return Err(AuthoringDocumentError::InvalidArgument(
                        "独立漫画改编来源不属于当前素材版本".into(),
                    ));
                }
            }
        }
    }
    Ok(())
}

fn take_valid_source_row(
    expected_root_project_id: i64,
    source: &AuthoringSourceRef,
    rows: &mut HashMap<i64, source_snapshot_chapter::Model>,
    seen: &mut BTreeSet<i64>,
) -> Result<(), AuthoringDocumentError> {
    if !seen.insert(source.source_snapshot_chapter_id) {
        return Err(AuthoringDocumentError::InvalidArgument(
            "创作文档原文引用重复".into(),
        ));
    }
    let row = rows
        .remove(&source.source_snapshot_chapter_id)
        .ok_or_else(|| AuthoringDocumentError::InvalidArgument("原文快照章节不存在".into()))?;
    validate_source_row(expected_root_project_id, source, &row)?;
    Ok(())
}

fn validate_source_row(
    expected_root_project_id: i64,
    source: &AuthoringSourceRef,
    row: &source_snapshot_chapter::Model,
) -> Result<(), AuthoringDocumentError> {
    if source.root_project_id != expected_root_project_id
        || row.root_project_id != source.root_project_id
        || row.snapshot_id != source.source_snapshot_id
        || row.source_chapter_id != source.source_chapter_id
        || row.content_hash != source.content_sha256
        || a4n_domain::source_text_hash(&row.content) != source.content_sha256
    {
        return Err(AuthoringDocumentError::InvalidArgument(
            "创作文档原文来源不一致".into(),
        ));
    }
    Ok(())
}

pub(crate) async fn validate_adopted_authoring_dependencies<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    dependencies: &[a4n_domain::media_authoring::AuthoringDocumentRef],
) -> Result<(), AuthoringDocumentError> {
    if dependencies.is_empty() {
        return Ok(());
    }
    positive(project_id, "project_id")?;

    let mut seen = BTreeSet::new();
    for chunk in dependencies.chunks(AUTHORING_READ_BATCH_SIZE) {
        let first = &chunk[0];
        if seen.contains(&first.id) {
            return Err(AuthoringDocumentError::InvalidArgument(
                "创作文档依赖重复".into(),
            ));
        }
        positive(first.id, "record_id")?;
        let mut records = document::Entity::find()
            .filter(document::Column::ProjectId.eq(project_id))
            .filter(document::Column::Id.is_in(chunk.iter().map(|dependency| dependency.id)))
            .all(db)
            .await?
            .into_iter()
            .map(|record| (record.id, record))
            .collect::<HashMap<_, _>>();
        validate_adopted_dependency_rows(project_id, chunk, &mut records, &mut seen)?;
    }
    Ok(())
}

fn validate_adopted_dependency_rows(
    project_id: i64,
    dependencies: &[a4n_domain::media_authoring::AuthoringDocumentRef],
    records: &mut HashMap<i64, document::Model>,
    seen: &mut BTreeSet<i64>,
) -> Result<(), AuthoringDocumentError> {
    for dependency in dependencies {
        if !seen.insert(dependency.id) {
            return Err(AuthoringDocumentError::InvalidArgument(
                "创作文档依赖重复".into(),
            ));
        }
        positive(project_id, "project_id")?;
        positive(dependency.id, "record_id")?;
        let row = records
            .remove(&dependency.id)
            .ok_or(AuthoringDocumentError::NotFound(dependency.id))?;
        let record = parse_record(row)?;
        if record.revision != dependency.revision
            || record.sha256 != dependency.sha256
            || record.status != AuthoringDocumentStatus::Adopted
            || record.kind != AuthoringDocumentKind::AdaptationPlan
        {
            return Err(AuthoringDocumentError::InvalidArgument(
                "请先采用对应版本的改编计划".into(),
            ));
        }
    }
    Ok(())
}

#[cfg(test)]
mod dependency_validation_tests {
    use super::*;
    use a4n_domain::media_authoring::AuthoringDocumentRef;

    fn row(id: i64, status: &str, kind: &str) -> document::Model {
        let inputs_json = serde_json::json!({"sources": [], "video_reference": null, "dependencies": [], "preprocessing_fingerprint": null});
        let document_json = serde_json::json!({"units": []});
        let sha256 = crate::generation::canonical_json_sha256(&(&inputs_json, &document_json))
            .expect("canonical hash");
        document::Model {
            id,
            project_id: 7,
            source_snapshot_id: 11,
            source_chapter_id: Some(id),
            kind: kind.to_string(),
            version: 1,
            revision: 3,
            status: status.to_string(),
            inputs_json,
            document_json,
            document_sha256: sha256,
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
        }
    }

    #[test]
    fn batched_dependency_validation_preserves_order_and_failure_semantics() {
        let first = row(10, "adopted", "adaptation_plan");
        let second = row(20, "adopted", "adaptation_plan");
        let draft = row(30, "draft", "adaptation_plan");
        let wrong_kind = row(40, "adopted", "galgame_branch_plan");
        let mut records = HashMap::from([
            (first.id, first.clone()),
            (second.id, second.clone()),
            (draft.id, draft.clone()),
            (wrong_kind.id, wrong_kind.clone()),
        ]);
        let reference = |record: &document::Model| AuthoringDocumentRef {
            id: record.id,
            revision: record.revision,
            sha256: record.document_sha256.clone(),
        };

        let mut seen = BTreeSet::new();
        let mut valid_records = records.clone();
        assert!(
            validate_adopted_dependency_rows(
                7,
                &[reference(&first), reference(&second)],
                &mut valid_records,
                &mut seen,
            )
            .is_ok()
        );
        assert!(validate_adopted_dependency_rows(7, &[], &mut valid_records, &mut seen).is_ok());
        assert!(matches!(
            validate_adopted_dependency_rows(
                7,
                &[AuthoringDocumentRef { id: 0, revision: 1, sha256: "0".repeat(64) }],
                &mut records,
                &mut BTreeSet::new(),
            ),
            Err(AuthoringDocumentError::InvalidArgument(message)) if message == "record_id 必须大于 0"
        ));
        assert!(matches!(
            validate_adopted_dependency_rows(7, &[reference(&second), reference(&second)], &mut records, &mut BTreeSet::new()),
            Err(AuthoringDocumentError::InvalidArgument(message)) if message == "创作文档依赖重复"
        ));
        assert!(matches!(
            validate_adopted_dependency_rows(
                7,
                &[reference(&draft)],
                &mut records,
                &mut BTreeSet::new()
            ),
            Err(AuthoringDocumentError::InvalidArgument(_))
        ));
        assert!(matches!(
            validate_adopted_dependency_rows(
                7,
                &[reference(&wrong_kind)],
                &mut records,
                &mut BTreeSet::new()
            ),
            Err(AuthoringDocumentError::InvalidArgument(_))
        ));
        let mut wrong_revision = reference(&first);
        wrong_revision.revision += 1;
        assert!(matches!(
            validate_adopted_dependency_rows(
                7,
                &[wrong_revision],
                &mut records,
                &mut BTreeSet::new()
            ),
            Err(AuthoringDocumentError::InvalidArgument(_))
        ));
        let mut missing_records = records.clone();
        missing_records.remove(&first.id);
        assert!(matches!(
            validate_adopted_dependency_rows(
                7,
                &[reference(&first)],
                &mut missing_records,
                &mut BTreeSet::new()
            ),
            Err(AuthoringDocumentError::NotFound(10))
        ));
        assert!(matches!(
            validate_adopted_dependency_rows(0, &[reference(&first)], &mut records, &mut BTreeSet::new()),
            Err(AuthoringDocumentError::InvalidArgument(message)) if message == "project_id 必须大于 0"
        ));
    }

    #[test]
    fn batched_source_validation_preserves_identity_and_failure_semantics() {
        let content = "冻结原文".to_string();
        let hash = a4n_domain::source_text_hash(&content);
        let row = source_snapshot_chapter::Model {
            id: 101,
            snapshot_id: 11,
            root_project_id: 7,
            source_chapter_id: 13,
            chapter_number: 1,
            title: Some("第一章".into()),
            content,
            summary: None,
            analysis_data_json: None,
            content_hash: hash.clone(),
            analysis_hash: None,
        };
        let source = AuthoringSourceRef {
            root_project_id: 7,
            source_chapter_id: 13,
            source_snapshot_id: 11,
            source_snapshot_chapter_id: 101,
            content_sha256: hash,
        };
        assert!(validate_source_row(7, &source, &row).is_ok());

        let mut seen = BTreeSet::new();
        let mut rows = HashMap::from([(row.id, row.clone())]);
        assert!(take_valid_source_row(7, &source, &mut rows, &mut seen).is_ok());
        assert!(matches!(
            take_valid_source_row(7, &source, &mut rows, &mut seen),
            Err(AuthoringDocumentError::InvalidArgument(message)) if message == "创作文档原文引用重复"
        ));
        assert!(matches!(
            take_valid_source_row(7, &source, &mut HashMap::new(), &mut BTreeSet::new()),
            Err(AuthoringDocumentError::InvalidArgument(message)) if message == "原文快照章节不存在"
        ));

        let mut wrong_snapshot = row.clone();
        wrong_snapshot.snapshot_id += 1;
        assert!(matches!(
            validate_source_row(7, &source, &wrong_snapshot),
            Err(AuthoringDocumentError::InvalidArgument(message)) if message == "创作文档原文来源不一致"
        ));
        let mut wrong_root = row.clone();
        wrong_root.root_project_id += 1;
        assert!(matches!(
            validate_source_row(7, &source, &wrong_root),
            Err(AuthoringDocumentError::InvalidArgument(message)) if message == "创作文档原文来源不一致"
        ));
        let mut wrong_content_hash = row.clone();
        wrong_content_hash.content_hash = "0".repeat(64);
        assert!(matches!(
            validate_source_row(7, &source, &wrong_content_hash),
            Err(AuthoringDocumentError::InvalidArgument(message)) if message == "创作文档原文来源不一致"
        ));
    }

    #[test]
    fn batched_authoring_document_reads_preserve_order_and_draft_priority() {
        let adopted_first = row(10, "adopted", "adaptation_plan");
        let mut draft_first = row(11, "draft", "adaptation_plan");
        draft_first.source_chapter_id = Some(adopted_first.id);
        let adopted_second = row(20, "adopted", "adaptation_plan");

        let by_chapter = prefer_adopted_fallback(
            HashMap::from([(adopted_first.id, draft_first)]),
            vec![adopted_first.clone(), adopted_second.clone()],
        );
        let mut by_chapter = by_chapter;
        let current = take_unparsed_documents_in_order(&[20, 10, 404, 20], &mut by_chapter);
        assert_eq!(
            current
                .iter()
                .map(|row| row.as_ref().map(|row| row.0.id))
                .collect::<Vec<_>>(),
            [Some(20), Some(11), None, Some(20)]
        );
        assert_eq!(
            current[1]
                .clone()
                .expect("draft selected")
                .parse()
                .unwrap()
                .status,
            AuthoringDocumentStatus::Draft
        );

        let mut by_id = HashMap::from([
            (adopted_first.id, adopted_first),
            (adopted_second.id, adopted_second),
        ]);
        let by_id = take_unparsed_documents_in_order(&[20, 10, 20, 404], &mut by_id);
        assert_eq!(
            by_id
                .iter()
                .map(|row| row.as_ref().map(|row| row.0.id))
                .collect::<Vec<_>>(),
            [Some(20), Some(10), Some(20), None]
        );
    }
}

pub fn parse_record(
    row: document::Model,
) -> Result<AuthoringDocumentRecord, AuthoringDocumentError> {
    let actual = crate::generation::canonical_json_sha256(&(&row.inputs_json, &row.document_json))
        .map_err(invalid_json)?;
    if actual != row.document_sha256 {
        return Err(AuthoringDocumentError::InvalidState(format!(
            "创作文档 {} 内容哈希不一致",
            row.id
        )));
    }
    Ok(AuthoringDocumentRecord {
        id: row.id,
        project_id: row.project_id,
        source_snapshot_id: row.source_snapshot_id,
        source_chapter_id: row.source_chapter_id,
        kind: serde_json::from_value(Value::String(row.kind)).map_err(invalid_json)?,
        version: row.version,
        revision: row.revision,
        status: serde_json::from_value(Value::String(row.status)).map_err(invalid_json)?,
        inputs: serde_json::from_value(row.inputs_json).map_err(invalid_json)?,
        document: row.document_json,
        sha256: row.document_sha256,
    })
}

pub fn parse_family_transfer_record(
    row: a4n_domain::project::ProjectFamilyAuthoringDocumentRecord,
) -> Result<AuthoringDocumentRecord, AuthoringDocumentError> {
    let row = serde_json::from_value(serde_json::to_value(row).map_err(invalid_json)?)
        .map_err(invalid_json)?;
    parse_record(row)
}

fn positive(value: i64, field: &str) -> Result<(), AuthoringDocumentError> {
    if value <= 0 {
        return Err(AuthoringDocumentError::InvalidArgument(format!(
            "{field} 必须大于 0"
        )));
    }
    Ok(())
}

fn next(value: i32) -> Result<i32, AuthoringDocumentError> {
    value
        .checked_add(1)
        .ok_or_else(|| AuthoringDocumentError::InvalidState("创作文档版本溢出".into()))
}

fn invalid_json(error: impl std::fmt::Display) -> AuthoringDocumentError {
    AuthoringDocumentError::InvalidState(error.to_string())
}

pub async fn video_reference_scripts<C: ConnectionTrait>(
    db: &C,
    root_project_id: i64,
    chapter_id: Option<i64>,
) -> Result<Vec<(ProjectRecord, ChapterShootingScriptReferenceRecord)>, DbErr> {
    use a4n_db_models::models::chapter::chapter_shooting_script as script;
    let projects = novel_project::Entity::find()
        .filter(novel_project::Column::RootProjectId.eq(root_project_id))
        .filter(novel_project::Column::ProjectKind.eq("video"))
        .all(db)
        .await?
        .into_iter()
        .map(|row| (row.id, row))
        .collect::<std::collections::BTreeMap<_, _>>();
    let mut query =
        script::Entity::find().filter(script::Column::ProjectId.is_in(projects.keys().copied()));
    if let Some(id) = chapter_id {
        query = query.filter(script::Column::SourceChapterId.eq(id));
    }
    let mut rows = Vec::new();
    for script in query
        .order_by_desc(script::Column::UpdatedAt)
        .all(db)
        .await?
    {
        let project = projects
            .get(&script.project_id)
            .ok_or_else(|| DbErr::Custom("视频剧本归属不一致".into()))?;
        rows.push((
            crate::project::project_record_from_model(project.clone()),
            chapter_shooting_script_reference_record(script),
        ));
    }
    Ok(rows)
}

pub async fn video_reference_script<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    script_id: i64,
) -> Result<Option<ChapterShootingScriptReferenceRecord>, DbErr> {
    use a4n_db_models::models::chapter::chapter_shooting_script as script;
    script::Entity::find_by_id(script_id)
        .filter(script::Column::ProjectId.eq(project_id))
        .one(db)
        .await
        .map(|row| row.map(chapter_shooting_script_reference_record))
}

fn chapter_shooting_script_reference_record(
    row: a4n_db_models::models::chapter::chapter_shooting_script::Model,
) -> ChapterShootingScriptReferenceRecord {
    ChapterShootingScriptReferenceRecord {
        id: row.id,
        project_id: row.project_id,
        root_project_id: row.root_project_id,
        source_chapter_id: row.source_chapter_id,
        source_snapshot_id: row.source_snapshot_id,
        source_snapshot_chapter_id: row.source_snapshot_chapter_id,
        source_revision_key: row.source_revision_key,
        source_content_sha256: row.source_content_sha256,
        revision: row.revision,
        script_sha256: row.script_sha256,
        script_json: row.script_json,
    }
}

#[cfg(test)]
mod perf_probe {
    use super::*;
    use a4n_db_models::models::chapter::chapter as source_chapter;
    use a4n_db_models::models::project::media_authoring_document as document;
    use a4n_db_models::models::project::{project_source_snapshot, source_snapshot_chapter};
    use a4n_domain::media_authoring::{
        AuthoringDocumentInputs, AuthoringDocumentRef, AuthoringSourceRef,
    };
    use chrono::Utc;
    use sea_orm::Set;
    use std::time::Instant;

    async fn validate_source_references_serial<C: ConnectionTrait>(
        db: &C,
        expected_root_project_id: i64,
        source_refs: &[AuthoringSourceRef],
    ) -> Result<(), AuthoringDocumentError> {
        let mut seen = BTreeSet::new();
        for source in source_refs {
            if !seen.insert(source.source_snapshot_chapter_id) {
                return Err(AuthoringDocumentError::InvalidArgument(
                    "创作文档原文引用重复".into(),
                ));
            }
            let row =
                source_snapshot_chapter::Entity::find_by_id(source.source_snapshot_chapter_id)
                    .one(db)
                    .await?
                    .ok_or_else(|| {
                        AuthoringDocumentError::InvalidArgument("原文快照章节不存在".into())
                    })?;
            validate_source_row(expected_root_project_id, source, &row)?;
        }
        Ok(())
    }

    type AuthoringLookupSummary = (i64, AuthoringDocumentRef, AuthoringDocumentStatus);

    async fn current_authoring_documents_serial<C: ConnectionTrait>(
        db: &C,
        project_id: i64,
        snapshot_id: i64,
        source_chapter_ids: &[i64],
    ) -> Result<Vec<AuthoringLookupSummary>, AuthoringDocumentError> {
        let mut summary = Vec::new();
        for source_chapter_id in source_chapter_ids {
            let draft = current_authoring_document(
                db,
                project_id,
                snapshot_id,
                Some(*source_chapter_id),
                AuthoringDocumentKind::AdaptationPlan,
                AuthoringDocumentStatus::Draft,
            )
            .await?;
            let record = match draft {
                Some(record) => Some(record),
                None => {
                    current_authoring_document(
                        db,
                        project_id,
                        snapshot_id,
                        Some(*source_chapter_id),
                        AuthoringDocumentKind::AdaptationPlan,
                        AuthoringDocumentStatus::Adopted,
                    )
                    .await?
                }
            };
            if let Some(record) = record {
                summary.push((*source_chapter_id, record.reference(), record.status));
            }
        }
        Ok(summary)
    }

    async fn current_authoring_documents_batched<C: ConnectionTrait>(
        db: &C,
        project_id: i64,
        snapshot_id: i64,
        source_chapter_ids: &[i64],
    ) -> Result<Vec<AuthoringLookupSummary>, AuthoringDocumentError> {
        let mut summary = Vec::new();
        for chunk in source_chapter_ids.chunks(AUTHORING_READ_BATCH_SIZE) {
            let rows = current_authoring_document_rows_by_source_chapters(
                db,
                project_id,
                snapshot_id,
                AuthoringDocumentKind::AdaptationPlan,
                chunk,
            )
            .await?;
            for (source_chapter_id, row) in chunk.iter().zip(rows) {
                if let Some(record) = row?.map(UnparsedAuthoringDocument::parse).transpose()? {
                    summary.push((*source_chapter_id, record.reference(), record.status));
                }
            }
        }
        Ok(summary)
    }

    async fn authoring_documents_by_ids_serial<C: ConnectionTrait>(
        db: &C,
        project_id: i64,
        record_ids: &[i64],
    ) -> Result<Vec<AuthoringLookupSummary>, AuthoringDocumentError> {
        let mut summary = Vec::with_capacity(record_ids.len());
        for record_id in record_ids {
            let record = authoring_document(db, project_id, *record_id).await?;
            summary.push((*record_id, record.reference(), record.status));
        }
        Ok(summary)
    }

    async fn authoring_documents_by_ids_batched<C: ConnectionTrait>(
        db: &C,
        project_id: i64,
        record_ids: &[i64],
    ) -> Result<Vec<AuthoringLookupSummary>, AuthoringDocumentError> {
        let mut summary = Vec::with_capacity(record_ids.len());
        for chunk in record_ids.chunks(AUTHORING_READ_BATCH_SIZE) {
            let rows = authoring_document_rows_by_ids(db, project_id, chunk).await?;
            for (record_id, row) in chunk.iter().zip(rows) {
                let row = row?.ok_or(AuthoringDocumentError::NotFound(*record_id))?;
                let record = row.parse()?;
                summary.push((*record_id, record.reference(), record.status));
            }
        }
        Ok(summary)
    }

    async fn adopted_authoring_documents_serial<C: ConnectionTrait>(
        db: &C,
        project_id: i64,
        snapshot_id: i64,
        source_chapter_ids: &[i64],
    ) -> Result<Vec<AuthoringLookupSummary>, AuthoringDocumentError> {
        let mut summary = Vec::with_capacity(source_chapter_ids.len());
        for source_chapter_id in source_chapter_ids {
            let record = current_authoring_document(
                db,
                project_id,
                snapshot_id,
                Some(*source_chapter_id),
                AuthoringDocumentKind::AdaptationPlan,
                AuthoringDocumentStatus::Adopted,
            )
            .await?
            .ok_or_else(|| AuthoringDocumentError::NotFound(*source_chapter_id))?;
            summary.push((*source_chapter_id, record.reference(), record.status));
        }
        Ok(summary)
    }

    async fn adopted_authoring_documents_batched<C: ConnectionTrait>(
        db: &C,
        project_id: i64,
        snapshot_id: i64,
        source_chapter_ids: &[i64],
    ) -> Result<Vec<AuthoringLookupSummary>, AuthoringDocumentError> {
        let mut summary = Vec::with_capacity(source_chapter_ids.len());
        for chunk in source_chapter_ids.chunks(AUTHORING_READ_BATCH_SIZE) {
            let rows = adopted_authoring_document_rows_by_source_chapters(
                db,
                project_id,
                snapshot_id,
                AuthoringDocumentKind::AdaptationPlan,
                chunk,
            )
            .await?;
            for (source_chapter_id, row) in chunk.iter().zip(rows) {
                let record = row?
                    .map(UnparsedAuthoringDocument::parse)
                    .transpose()?
                    .ok_or_else(|| AuthoringDocumentError::NotFound(*source_chapter_id))?;
                summary.push((*source_chapter_id, record.reference(), record.status));
            }
        }
        Ok(summary)
    }

    #[tokio::test]
    #[ignore = "manual performance probe"]
    async fn media_authoring_reference_probe() {
        use std::time::Duration;

        let db = a4n_migration::test_support::connect_test_database().await;
        let user = crate::auth::create_user(
            &db,
            "media-authoring-perf-user",
            "media-authoring-perf@example.test",
            "password",
            "user",
        )
        .await
        .expect("create user");
        let project = crate::project::create_project_record(
            &db,
            crate::project::CreateProjectRecordInput {
                user_id: user.id,
                title: "media authoring perf".to_string(),
                description: String::new(),
                project_type: "novel_long".to_string(),
            },
        )
        .await
        .expect("create project");
        let now = Utc::now().naive_utc();
        let dependency_count = 1000_i64;
        let chapters = (1..=dependency_count)
            .map(|number| source_chapter::ActiveModel {
                id: Default::default(),
                project_id: Set(project.id),
                outline_id: Set(None),
                chapter_number: Set(number as i32),
                title: Set(Some(format!("第 {number} 章"))),
                content: Set(format!("来源 {number}")),
                word_count: Set(4),
                status: Set("imported".to_string()),
                summary: Set(None),
                style_tags: Set(None),
                analysis_data: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
            })
            .collect::<Vec<_>>();
        let chapters = source_chapter::Entity::insert_many(chapters)
            .exec_with_returning_many(&db)
            .await
            .expect("insert source chapters");
        let snapshot = project_source_snapshot::ActiveModel {
            root_project_id: Set(project.id),
            source_version_hash: Set("perf-source-snapshot".to_string()),
            blueprint_version_id: Set(None),
            chapter_scope_json: Set(
                serde_json::json!({"start_chapter":1,"end_chapter":dependency_count}),
            ),
            chapter_hashes_json: Set(serde_json::json!([])),
            profile_hashes_json: Set(serde_json::json!({})),
            created_at: Set(now),
            ..Default::default()
        }
        .insert(&db)
        .await
        .expect("insert source snapshot");
        let snapshot_content = "x".repeat(8 * 1024);
        let snapshot_content_hash = a4n_domain::source_text_hash(&snapshot_content);
        let snapshot_chapters = chapters
            .iter()
            .map(|chapter| source_snapshot_chapter::ActiveModel {
                snapshot_id: Set(snapshot.id),
                root_project_id: Set(project.id),
                source_chapter_id: Set(chapter.id),
                chapter_number: Set(chapter.chapter_number),
                title: Set(chapter.title.clone()),
                content: Set(snapshot_content.clone()),
                summary: Set(None),
                analysis_data_json: Set(None),
                content_hash: Set(snapshot_content_hash.clone()),
                analysis_hash: Set(None),
                ..Default::default()
            })
            .collect::<Vec<_>>();
        source_snapshot_chapter::Entity::insert_many(snapshot_chapters)
            .exec(&db)
            .await
            .expect("insert snapshot chapters");
        let source_rows = source_snapshot_chapter::Entity::find()
            .filter(source_snapshot_chapter::Column::SnapshotId.eq(snapshot.id))
            .order_by_asc(source_snapshot_chapter::Column::ChapterNumber)
            .all(&db)
            .await
            .expect("load source snapshot chapters");
        let source_refs = source_rows
            .iter()
            .map(|row| AuthoringSourceRef {
                root_project_id: project.id,
                source_chapter_id: row.source_chapter_id,
                source_snapshot_id: snapshot.id,
                source_snapshot_chapter_id: row.id,
                content_sha256: snapshot_content_hash.clone(),
            })
            .collect::<Vec<_>>();
        let now = Utc::now().naive_utc();
        let mut refs = Vec::with_capacity(dependency_count as usize);
        let mut rows = Vec::with_capacity(dependency_count as usize);
        let mut document_bytes = 0usize;
        for (index, chapter) in chapters.iter().enumerate() {
            let document_id = index as i64 + 1;
            let inputs = AuthoringDocumentInputs {
                sources: Vec::new(),
                video_reference: None,
                dependencies: Vec::new(),
                preprocessing_fingerprint: None,
            };
            let document_json = serde_json::json!({
                "schema_version": "adaptation_plan_v1",
                "target": "galgame",
                "intention": "x".repeat(8 * 1024),
                "decisions": [],
                "units": [],
            });
            document_bytes += serde_json::to_vec(&document_json)
                .expect("serialize benchmark document")
                .len();
            let inputs_json = serde_json::to_value(&inputs).expect("inputs json");
            let hash = crate::generation::canonical_json_sha256(&(&inputs_json, &document_json))
                .expect("hash");
            refs.push(AuthoringDocumentRef {
                id: document_id,
                revision: 1,
                sha256: hash.clone(),
            });
            rows.push(document::ActiveModel {
                id: Set(document_id),
                project_id: Set(project.id),
                source_snapshot_id: Set(snapshot.id),
                source_chapter_id: Set(Some(chapter.id)),
                kind: Set("adaptation_plan".to_string()),
                version: Set(1),
                revision: Set(1),
                status: Set("adopted".to_string()),
                inputs_json: Set(inputs_json),
                document_json: Set(document_json),
                document_sha256: Set(hash),
                created_at: Set(now),
                updated_at: Set(now),
            });
        }
        document::Entity::insert_many(rows)
            .exec(&db)
            .await
            .expect("insert docs");
        let draft_inputs = serde_json::to_value(&AuthoringDocumentInputs {
            sources: Vec::new(),
            video_reference: None,
            dependencies: Vec::new(),
            preprocessing_fingerprint: None,
        })
        .expect("draft inputs json");
        let draft_document = serde_json::json!({
            "schema_version": "adaptation_plan_v1",
            "target": "galgame",
            "intention": "x".repeat(8 * 1024),
            "decisions": [],
            "units": [],
        });
        let draft_hash =
            crate::generation::canonical_json_sha256(&(&draft_inputs, &draft_document))
                .expect("draft hash");
        document::ActiveModel {
            id: Set(dependency_count + 1),
            project_id: Set(project.id),
            source_snapshot_id: Set(snapshot.id),
            source_chapter_id: Set(Some(chapters[0].id)),
            kind: Set("adaptation_plan".to_string()),
            version: Set(2),
            revision: Set(2),
            status: Set("draft".to_string()),
            inputs_json: Set(draft_inputs),
            document_json: Set(draft_document),
            document_sha256: Set(draft_hash),
            created_at: Set(now),
            updated_at: Set(now),
        }
        .insert(&db)
        .await
        .expect("insert draft adaptation document");

        let mut old_samples = Vec::with_capacity(3);
        let mut new_samples = Vec::with_capacity(3);
        for pair in 0..5 {
            let old_started = Instant::now();
            for reference in &refs {
                let record = authoring_document(&db, project.id, reference.id)
                    .await
                    .expect("old dependency lookup");
                assert_eq!(record.revision, reference.revision);
                assert_eq!(record.sha256, reference.sha256);
                assert_eq!(record.status, AuthoringDocumentStatus::Adopted);
                assert_eq!(record.kind, AuthoringDocumentKind::AdaptationPlan);
            }
            let old_ms = old_started.elapsed().as_secs_f64() * 1_000.0;

            let new_started = Instant::now();
            validate_adopted_authoring_dependencies(&db, project.id, &refs)
                .await
                .expect("batch dependency validation");
            let new_ms = new_started.elapsed().as_secs_f64() * 1_000.0;
            if pair >= 2 {
                old_samples.push(old_ms);
                new_samples.push(new_ms);
            }
            tokio::time::sleep(Duration::from_millis(15)).await;
        }
        old_samples.sort_by(f64::total_cmp);
        new_samples.sort_by(f64::total_cmp);
        let old_ms = old_samples[old_samples.len() / 2];
        let new_ms = new_samples[new_samples.len() / 2];
        println!(
            "dependency_batch rows={} document_bytes={document_bytes} p50_old_ms={old_ms:.3} p50_new_ms={new_ms:.3} old_samples={old_samples:?} new_samples={new_samples:?}",
            refs.len(),
        );
        assert!(
            old_ms > new_ms * 1.3,
            "batch probe must clear the latency gate"
        );

        let source_chapter_ids = chapters
            .iter()
            .map(|chapter| chapter.id)
            .collect::<Vec<_>>();
        let record_ids = refs
            .iter()
            .map(|reference| reference.id)
            .collect::<Vec<_>>();
        let mut old_current_samples = Vec::with_capacity(3);
        let mut new_current_samples = Vec::with_capacity(3);
        for pair in 0..5 {
            let sample = if pair % 2 == 0 {
                let old_started = Instant::now();
                let old = current_authoring_documents_serial(
                    &db,
                    project.id,
                    snapshot.id,
                    &source_chapter_ids,
                )
                .await
                .expect("serial current document reads");
                let old_ms = old_started.elapsed().as_secs_f64() * 1_000.0;
                let new_started = Instant::now();
                let new = current_authoring_documents_batched(
                    &db,
                    project.id,
                    snapshot.id,
                    &source_chapter_ids,
                )
                .await
                .expect("batched current document reads");
                (
                    old_ms,
                    new_started.elapsed().as_secs_f64() * 1_000.0,
                    old,
                    new,
                )
            } else {
                let new_started = Instant::now();
                let new = current_authoring_documents_batched(
                    &db,
                    project.id,
                    snapshot.id,
                    &source_chapter_ids,
                )
                .await
                .expect("batched current document reads");
                let new_ms = new_started.elapsed().as_secs_f64() * 1_000.0;
                let old_started = Instant::now();
                let old = current_authoring_documents_serial(
                    &db,
                    project.id,
                    snapshot.id,
                    &source_chapter_ids,
                )
                .await
                .expect("serial current document reads");
                (
                    old_started.elapsed().as_secs_f64() * 1_000.0,
                    new_ms,
                    old,
                    new,
                )
            };
            assert_eq!(sample.2, sample.3);
            assert_eq!(sample.3.len(), source_chapter_ids.len());
            assert_eq!(sample.3[0].2, AuthoringDocumentStatus::Draft);
            if pair >= 2 {
                old_current_samples.push(sample.0);
                new_current_samples.push(sample.1);
            }
            tokio::time::sleep(Duration::from_millis(15)).await;
        }
        old_current_samples.sort_by(f64::total_cmp);
        new_current_samples.sort_by(f64::total_cmp);
        let old_current_ms = old_current_samples[old_current_samples.len() / 2];
        let new_current_ms = new_current_samples[new_current_samples.len() / 2];
        println!(
            "current_authoring_batch chapters={} p50_old_ms={old_current_ms:.3} p50_new_ms={new_current_ms:.3} old_samples={old_current_samples:?} new_samples={new_current_samples:?}",
            source_chapter_ids.len(),
        );
        assert!(
            old_current_ms > new_current_ms * 1.3,
            "current authoring read batch probe must clear the latency gate"
        );

        let mut old_id_samples = Vec::with_capacity(3);
        let mut new_id_samples = Vec::with_capacity(3);
        for pair in 0..5 {
            let sample = if pair % 2 == 0 {
                let old_started = Instant::now();
                let old = authoring_documents_by_ids_serial(&db, project.id, &record_ids)
                    .await
                    .expect("serial branch plan document reads");
                let old_ms = old_started.elapsed().as_secs_f64() * 1_000.0;
                let new_started = Instant::now();
                let new = authoring_documents_by_ids_batched(&db, project.id, &record_ids)
                    .await
                    .expect("batched branch plan document reads");
                (
                    old_ms,
                    new_started.elapsed().as_secs_f64() * 1_000.0,
                    old,
                    new,
                )
            } else {
                let new_started = Instant::now();
                let new = authoring_documents_by_ids_batched(&db, project.id, &record_ids)
                    .await
                    .expect("batched branch plan document reads");
                let new_ms = new_started.elapsed().as_secs_f64() * 1_000.0;
                let old_started = Instant::now();
                let old = authoring_documents_by_ids_serial(&db, project.id, &record_ids)
                    .await
                    .expect("serial branch plan document reads");
                (
                    old_started.elapsed().as_secs_f64() * 1_000.0,
                    new_ms,
                    old,
                    new,
                )
            };
            assert_eq!(sample.2, sample.3);
            assert!(
                sample
                    .3
                    .iter()
                    .all(|(_, _, status)| status == &AuthoringDocumentStatus::Adopted)
            );
            if pair >= 2 {
                old_id_samples.push(sample.0);
                new_id_samples.push(sample.1);
            }
            tokio::time::sleep(Duration::from_millis(15)).await;
        }
        old_id_samples.sort_by(f64::total_cmp);
        new_id_samples.sort_by(f64::total_cmp);
        let old_id_ms = old_id_samples[old_id_samples.len() / 2];
        let new_id_ms = new_id_samples[new_id_samples.len() / 2];
        println!(
            "authoring_document_id_batch rows={} p50_old_ms={old_id_ms:.3} p50_new_ms={new_id_ms:.3} old_samples={old_id_samples:?} new_samples={new_id_samples:?}",
            record_ids.len(),
        );
        assert!(
            old_id_ms > new_id_ms * 1.3,
            "document ID batch probe must clear the latency gate"
        );

        let mut old_adopted_samples = Vec::with_capacity(3);
        let mut new_adopted_samples = Vec::with_capacity(3);
        for pair in 0..5 {
            let sample = if pair % 2 == 0 {
                let old_started = Instant::now();
                let old = adopted_authoring_documents_serial(
                    &db,
                    project.id,
                    snapshot.id,
                    &source_chapter_ids,
                )
                .await
                .expect("serial adopted plan dependencies");
                let old_ms = old_started.elapsed().as_secs_f64() * 1_000.0;
                let new_started = Instant::now();
                let new = adopted_authoring_documents_batched(
                    &db,
                    project.id,
                    snapshot.id,
                    &source_chapter_ids,
                )
                .await
                .expect("batched adopted plan dependencies");
                (
                    old_ms,
                    new_started.elapsed().as_secs_f64() * 1_000.0,
                    old,
                    new,
                )
            } else {
                let new_started = Instant::now();
                let new = adopted_authoring_documents_batched(
                    &db,
                    project.id,
                    snapshot.id,
                    &source_chapter_ids,
                )
                .await
                .expect("batched adopted plan dependencies");
                let new_ms = new_started.elapsed().as_secs_f64() * 1_000.0;
                let old_started = Instant::now();
                let old = adopted_authoring_documents_serial(
                    &db,
                    project.id,
                    snapshot.id,
                    &source_chapter_ids,
                )
                .await
                .expect("serial adopted plan dependencies");
                (
                    old_started.elapsed().as_secs_f64() * 1_000.0,
                    new_ms,
                    old,
                    new,
                )
            };
            assert_eq!(sample.2, sample.3);
            assert_eq!(sample.3.len(), source_chapter_ids.len());
            assert!(
                sample
                    .3
                    .iter()
                    .all(|(_, _, status)| status == &AuthoringDocumentStatus::Adopted)
            );
            if pair >= 2 {
                old_adopted_samples.push(sample.0);
                new_adopted_samples.push(sample.1);
            }
            tokio::time::sleep(Duration::from_millis(15)).await;
        }
        old_adopted_samples.sort_by(f64::total_cmp);
        new_adopted_samples.sort_by(f64::total_cmp);
        let old_adopted_ms = old_adopted_samples[old_adopted_samples.len() / 2];
        let new_adopted_ms = new_adopted_samples[new_adopted_samples.len() / 2];
        println!(
            "adopted_plan_dependency_batch chapters={} p50_old_ms={old_adopted_ms:.3} p50_new_ms={new_adopted_ms:.3} old_samples={old_adopted_samples:?} new_samples={new_adopted_samples:?}",
            source_chapter_ids.len(),
        );
        assert!(
            old_adopted_ms > new_adopted_ms * 1.3,
            "adopted authoring dependency batch probe must clear the latency gate"
        );

        let mut old_source_samples = Vec::with_capacity(3);
        let mut new_source_samples = Vec::with_capacity(3);
        for pair in 0..5 {
            let (old_source_ms, new_source_ms) = if pair % 2 == 0 {
                let old_started = Instant::now();
                validate_source_references_serial(&db, project.id, &source_refs)
                    .await
                    .expect("serial source validation");
                let old_ms = old_started.elapsed().as_secs_f64() * 1_000.0;
                let new_started = Instant::now();
                validate_source_references(&db, project.id, project.id, false, &source_refs)
                    .await
                    .expect("batched source validation");
                (old_ms, new_started.elapsed().as_secs_f64() * 1_000.0)
            } else {
                let new_started = Instant::now();
                validate_source_references(&db, project.id, project.id, false, &source_refs)
                    .await
                    .expect("batched source validation");
                let new_ms = new_started.elapsed().as_secs_f64() * 1_000.0;
                let old_started = Instant::now();
                validate_source_references_serial(&db, project.id, &source_refs)
                    .await
                    .expect("serial source validation");
                (old_started.elapsed().as_secs_f64() * 1_000.0, new_ms)
            };
            if pair >= 2 {
                old_source_samples.push(old_source_ms);
                new_source_samples.push(new_source_ms);
            }
            tokio::time::sleep(Duration::from_millis(15)).await;
        }
        old_source_samples.sort_by(f64::total_cmp);
        new_source_samples.sort_by(f64::total_cmp);
        let old_source_ms = old_source_samples[old_source_samples.len() / 2];
        let new_source_ms = new_source_samples[new_source_samples.len() / 2];
        println!(
            "source_reference_batch rows={} content_bytes={} p50_old_ms={old_source_ms:.3} p50_new_ms={new_source_ms:.3} old_samples={old_source_samples:?} new_samples={new_source_samples:?}",
            source_refs.len(),
            source_refs.len() * snapshot_content.len(),
        );
        assert!(
            old_source_ms > new_source_ms * 1.3,
            "source reference batch probe must clear the latency gate"
        );
    }
}
