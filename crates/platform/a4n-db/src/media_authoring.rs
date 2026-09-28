//! Versioned project artifacts. The domain owner validates its document before
//! saving; this boundary owns lineage, source integrity and concurrent writes.

use a4n_db_models::models::project::{
    media_authoring_document as document, novel_project, project_source_snapshot,
    source_snapshot_chapter,
};
use a4n_domain::media_authoring::{
    AuthoringDocumentInputs, AuthoringDocumentKind, AuthoringDocumentRecord,
    AuthoringDocumentStatus,
};
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, Condition, ConnectionTrait, DatabaseConnection, DbErr,
    EntityTrait, QueryFilter, QueryOrder, QuerySelect, Set, TransactionTrait, sea_query::Expr,
};
use serde_json::Value;

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
        chapter_id: i64,
        layout: a4n_domain::manga::MangaPageGenerationLayoutV3,
    },
    GalgameScript {
        source_snapshot_id: i64,
        branch_plan_id: Option<i64>,
        scene_plan_ids: Vec<Option<i64>>,
    },
}

pub async fn authoring_artifact_sources<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    target: a4n_domain::media_authoring::AdaptationTarget,
) -> Result<Vec<AuthoringArtifactSource>, AuthoringDocumentError> {
    use a4n_db_models::models::{
        chapter::chapter_manga_prompt,
        galgame::{galgame_scene, galgame_script},
    };
    match target {
        a4n_domain::media_authoring::AdaptationTarget::Manga => {
            let rows = chapter_manga_prompt::Entity::find()
                .filter(chapter_manga_prompt::Column::ProjectId.eq(project_id))
                .all(db)
                .await?;
            rows.into_iter()
                .map(|row| {
                    let layout = row.layout.ok_or_else(|| {
                        AuthoringDocumentError::InvalidState(format!("漫画页 {} 缺少布局", row.id))
                    })?;
                    let layout: a4n_domain::manga::MangaPageGenerationLayoutV3 =
                        serde_json::from_value(layout).map_err(|error| {
                            AuthoringDocumentError::InvalidState(error.to_string())
                        })?;
                    layout
                        .validate(u32::try_from(row.page_number).map_err(|error| {
                            AuthoringDocumentError::InvalidState(error.to_string())
                        })?)
                        .map_err(|error| AuthoringDocumentError::InvalidState(error.to_string()))?;
                    Ok(AuthoringArtifactSource::MangaPage {
                        chapter_id: row.chapter_id,
                        layout,
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
                source_snapshot_id: script.source_snapshot_id,
                branch_plan_id: script.branch_plan_id,
                scene_plan_ids: scenes
                    .into_iter()
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
    db: &DatabaseConnection,
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
    db: &DatabaseConnection,
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
    if !accepted || project.root_project_id.is_none() || project.source_snapshot_id.is_none() {
        return Err(AuthoringDocumentError::InvalidArgument(
            "文档类型与派生项目不匹配".into(),
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
    if Some(snapshot.root_project_id) != project.root_project_id
        || (kind != AuthoringDocumentKind::GalgameBranchPlan
            && inputs.sources[0].source_snapshot_id != snapshot_id)
    {
        return Err(AuthoringDocumentError::InvalidArgument(
            "创作文档基准快照不匹配".into(),
        ));
    }
    let mut sources = std::collections::BTreeSet::new();
    for source in &inputs.sources {
        if !sources.insert(source.source_snapshot_chapter_id) {
            return Err(AuthoringDocumentError::InvalidArgument(
                "创作文档原文引用重复".into(),
            ));
        }
        let row = source_snapshot_chapter::Entity::find_by_id(source.source_snapshot_chapter_id)
            .one(db)
            .await?
            .ok_or_else(|| AuthoringDocumentError::InvalidArgument("原文快照章节不存在".into()))?;
        if Some(source.root_project_id) != project.root_project_id
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
    }
    validate_adopted_authoring_dependencies(db, project_id, &inputs.dependencies).await?;
    if kind == AuthoringDocumentKind::AdaptationPlan && !inputs.dependencies.is_empty() {
        return Err(AuthoringDocumentError::InvalidArgument(
            "改编计划不能依赖其他本地阶段".into(),
        ));
    }
    Ok(())
}

pub(crate) async fn validate_adopted_authoring_dependencies<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    dependencies: &[a4n_domain::media_authoring::AuthoringDocumentRef],
) -> Result<(), AuthoringDocumentError> {
    let mut seen = std::collections::BTreeSet::new();
    for dependency in dependencies {
        if !seen.insert(dependency.id) {
            return Err(AuthoringDocumentError::InvalidArgument(
                "创作文档依赖重复".into(),
            ));
        }
        let record = authoring_document(db, project_id, dependency.id).await?;
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
) -> Result<
    Vec<(
        novel_project::Model,
        a4n_db_models::models::chapter::chapter_shooting_script::Model,
    )>,
    DbErr,
> {
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
        rows.push((project.clone(), script));
    }
    Ok(rows)
}

pub async fn video_reference_script<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    script_id: i64,
) -> Result<Option<a4n_db_models::models::chapter::chapter_shooting_script::Model>, DbErr> {
    use a4n_db_models::models::chapter::chapter_shooting_script as script;
    script::Entity::find_by_id(script_id)
        .filter(script::Column::ProjectId.eq(project_id))
        .one(db)
        .await
}
