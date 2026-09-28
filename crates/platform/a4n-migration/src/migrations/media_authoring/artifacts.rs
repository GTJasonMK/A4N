use std::collections::{BTreeMap, HashMap};

use a4n_db_models::models::{
    ai::generated_image,
    chapter::chapter_manga_prompt,
    project::{
        media_authoring_document as document, novel_project, project_source_snapshot,
        source_snapshot_chapter,
    },
};
use a4n_domain::{
    manga::{
        MangaPagePlanSchemaVersion, MangaPagePlanV4, MangaPagePromptSourceSummary, MangaPageV4,
    },
    media_authoring::{
        AdaptationPlanV1, AdaptationPlanVersion, AdaptationTarget, AdaptationUnit,
        AuthoringDocumentInputs, AuthoringDocumentRef, AuthoringSourceRef,
        legacy::{historical_manga_page, saved_manga_source, upgrade_saved_manga_layout},
    },
};
use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DbBackend, DbErr, EntityTrait, NotSet,
    QueryFilter, QueryOrder, Set, Statement,
};
use serde_json::{Value, json};

use super::{
    hash_value,
    ownership::{FilmOwnership, source_for_record},
    record_receipt,
};

struct MangaChapterMigration {
    source: source_snapshot_chapter::Model,
    current: bool,
    pages: Vec<(chapter_manga_prompt::Model, AdaptationUnit, MangaPageV4)>,
}

pub(super) async fn migrate_manga<C: ConnectionTrait>(
    db: &C,
    _films: &FilmOwnership,
    now: sea_orm::prelude::DateTime,
) -> Result<(), DbErr> {
    let rows = chapter_manga_prompt::Entity::find()
        .order_by_asc(chapter_manga_prompt::Column::PageNumber)
        .order_by_asc(chapter_manga_prompt::Column::Id)
        .all(db)
        .await?;
    let projects = novel_project::Entity::find()
        .all(db)
        .await?
        .into_iter()
        .map(|row| (row.id, row))
        .collect::<BTreeMap<_, _>>();
    let mut history_projects = BTreeMap::new();
    let mut sources = HashMap::new();
    let mut groups = BTreeMap::<(i64, i64, i64), MangaChapterMigration>::new();
    for row in rows {
        let layout = row
            .layout
            .as_ref()
            .ok_or_else(|| failure(format!("漫画页 {} 缺少原有布局", row.id)))?;
        if layout.get("schema_version").and_then(Value::as_str) == Some("manga_page_generation_v3")
        {
            continue;
        }
        let original = saved_manga_source(layout).map_err(failure)?;
        let root = projects
            .get(&original.root_project_id)
            .ok_or_else(|| failure("历史漫画原作不存在"))?;
        if root.project_kind != "text" || row.chapter_id != original.source_chapter_id {
            return Err(failure("历史漫画原作或章节身份不一致"));
        }
        let (_, source) = source_for_record(db, root, &json!({
            "source_chapter_id": original.source_chapter_id, "source_content_sha256": original.content_sha256,
            "source_snapshot_id": original.source_snapshot_id, "source_snapshot_chapter_id": original.source_snapshot_chapter_id,
        }), now, &mut sources).await?;
        let project = projects
            .get(&row.project_id)
            .ok_or_else(|| failure("漫画页所属项目不存在"))?;
        let (project_id, current) = match project.project_kind.as_str() {
            "comic" if project.root_project_id == Some(root.id) => (
                project.id,
                project.source_snapshot_id == Some(source.snapshot_id),
            ),
            "text" if project.id == root.id => {
                let key = (project.id, source.snapshot_id);
                let id = if let Some(id) = history_projects.get(&key) {
                    *id
                } else {
                    let id = create_historical_comic(db, project, &source, now).await?;
                    history_projects.insert(key, id);
                    id
                };
                (id, true)
            }
            _ => return Err(failure("历史漫画页所属项目类型或原作无效")),
        };
        let (unit, page) = historical_manga_page(row.id, layout).map_err(failure)?;
        groups
            .entry((project_id, source.snapshot_id, source.source_chapter_id))
            .or_insert_with(|| MangaChapterMigration {
                source,
                current,
                pages: Vec::new(),
            })
            .pages
            .push((row, unit, page));
    }
    for ((project_id, _, _), group) in groups {
        let plan = AdaptationPlanV1 {
            schema_version: AdaptationPlanVersion::V1,
            target: AdaptationTarget::Manga,
            intention: "保留已保存的漫画分页、画格、对白和人工编辑；原始导演引用保存在迁移回执。"
                .into(),
            decisions: Vec::new(),
            units: group
                .pages
                .iter()
                .map(|(_, unit, _)| unit.clone())
                .collect(),
        };
        plan.validate(&Default::default()).map_err(failure)?;
        let doc = insert_document(
            db,
            project_id,
            &group.source,
            "adaptation_plan",
            serde_json::to_value(plan).map_err(json_error)?,
            Vec::new(),
            now,
        )
        .await?;
        if group.current {
            adopt_historical_document(db, &doc).await?;
        }
        let pages = group
            .pages
            .iter()
            .map(|(_, _, page)| page.clone())
            .collect::<Vec<_>>();
        let page_plan_ref =
            migrate_page_plan(db, &doc, &group.source, pages, group.current, now).await?;
        for (row, _, page) in group.pages {
            let source = &group.source;
            let mut converted = upgrade_saved_manga_layout(
                row.layout
                    .as_ref()
                    .ok_or_else(|| failure("历史漫画布局丢失"))?,
                MangaPagePromptSourceSummary {
                    root_project_id: source.root_project_id,
                    source_chapter_id: source.source_chapter_id,
                    source_snapshot_id: Some(source.snapshot_id),
                    source_snapshot_chapter_id: Some(source.id),
                    source_revision_key: format!(
                        "snapshot:{}:chapter:{}:{}",
                        source.snapshot_id, source.id, source.content_hash
                    ),
                    content_sha256: source.content_hash.clone(),
                    adaptation_plan_id: doc.id,
                    adaptation_plan_revision: doc.revision,
                    adaptation_plan_sha256: doc.document_sha256.clone(),
                },
                page.clone(),
            )
            .map_err(failure)?;
            converted["page_plan_ref"] =
                serde_json::to_value(&page_plan_ref).map_err(json_error)?;
            let image_id = copy_manga_images(db, &row, project_id).await?;
            let before = serde_json::to_value(&row).map_err(json_error)?;
            let mut active: chapter_manga_prompt::ActiveModel = row.clone().into();
            active.project_id = Set(project_id);
            active.layout = Set(Some(converted.clone()));
            active.generated_image_id = Set(image_id);
            active.update(db).await?;
            record_receipt(
                db,
                "manga_layout",
                row.id,
                row.id,
                project_id,
                &before,
                &hash_value(&converted)?,
            )
            .await?;
        }
    }
    Ok(())
}

async fn migrate_page_plan<C: ConnectionTrait>(
    db: &C,
    doc: &document::Model,
    source: &source_snapshot_chapter::Model,
    pages: Vec<MangaPageV4>,
    current: bool,
    now: sea_orm::prelude::DateTime,
) -> Result<Option<AuthoringDocumentRef>, DbErr> {
    let project_id = doc.project_id;
    if pages
        .iter()
        .enumerate()
        .all(|(index, page)| usize::try_from(page.page_number).ok() == Some(index + 1))
    {
        let page_plan = MangaPagePlanV4 {
            schema_version: MangaPagePlanSchemaVersion::V4,
            adaptation_plan_id: doc.id,
            adaptation_plan_revision: doc.revision,
            pages,
        };
        page_plan
            .validate()
            .map_err(|error| failure(error.to_string()))?;
        let page_doc = insert_document(
            db,
            project_id,
            source,
            "manga_page_plan",
            serde_json::to_value(page_plan).map_err(json_error)?,
            vec![AuthoringDocumentRef {
                id: doc.id,
                revision: doc.revision,
                sha256: doc.document_sha256.clone(),
            }],
            now,
        )
        .await?;
        if current {
            adopt_historical_document(db, &page_doc).await?;
        }
        return Ok(Some(AuthoringDocumentRef {
            id: page_doc.id,
            revision: page_doc.revision,
            sha256: page_doc.document_sha256,
        }));
    } else {
        let payload = json!({"reason": "保存的页面不连续；保留各页和本地改编计划，不能标记完整分页计划已采用", "pages": pages});
        record_receipt(
            db,
            "partial_manga_page_plan",
            doc.id,
            doc.id,
            project_id,
            &payload,
            &hash_value(&payload)?,
        )
        .await?;
    }
    Ok(None)
}

async fn adopt_historical_document<C: ConnectionTrait>(
    db: &C,
    doc: &document::Model,
) -> Result<(), DbErr> {
    let existing = document::Entity::find()
        .filter(document::Column::ProjectId.eq(doc.project_id))
        .filter(document::Column::SourceSnapshotId.eq(doc.source_snapshot_id))
        .filter(document::Column::SourceChapterId.eq(doc.source_chapter_id))
        .filter(document::Column::Kind.eq(&doc.kind))
        .filter(document::Column::Status.eq("adopted"))
        .one(db)
        .await?;
    if existing.is_none() {
        document::ActiveModel {
            id: Set(doc.id),
            status: Set("adopted".into()),
            ..Default::default()
        }
        .update(db)
        .await?;
    }
    Ok(())
}

async fn copy_manga_images<C: ConnectionTrait>(
    db: &C,
    row: &chapter_manga_prompt::Model,
    project_id: i64,
) -> Result<Option<i64>, DbErr> {
    if row.project_id == project_id {
        return Ok(row.generated_image_id);
    }
    let images = generated_image::Entity::find().from_raw_sql(Statement::from_sql_and_values(DbBackend::Postgres,
        "SELECT * FROM generated_image WHERE project_id=$1 AND image_type='manga_page' AND (id=$2 OR metadata_json->>'page_prompt_id'=$3)",
        [row.project_id.into(), row.generated_image_id.into(), row.id.to_string().into()],
    )).all(db).await?;
    let mut adopted = None;
    for image in images {
        if image.chapter_id != Some(row.chapter_id) {
            return Err(failure("历史漫画图像章节归属不一致"));
        }
        let before = serde_json::to_value(&image).map_err(json_error)?;
        let mut active: generated_image::ActiveModel = image.clone().into();
        active.id = NotSet;
        active.project_id = Set(project_id);
        let copied = active.insert(db).await?;
        if Some(image.id) == row.generated_image_id {
            adopted = Some(copied.id);
        }
        // Keep the root's original media/history references; both records share
        // the existing file, which the normal cleanup reference checks protect.
        record_receipt(
            db,
            "manga_image_copy",
            image.id,
            copied.id,
            project_id,
            &before,
            &hash_value(&serde_json::to_value(&copied).map_err(json_error)?)?,
        )
        .await?;
    }
    if row.generated_image_id.is_some() && adopted.is_none() {
        return Err(failure("历史漫画已采用图像不存在"));
    }
    Ok(adopted)
}

pub(super) async fn insert_document<C: ConnectionTrait>(
    db: &C,
    project_id: i64,
    source: &source_snapshot_chapter::Model,
    kind: &str,
    payload: Value,
    dependencies: Vec<AuthoringDocumentRef>,
    now: sea_orm::prelude::DateTime,
) -> Result<document::Model, DbErr> {
    let previous = document::Entity::find()
        .filter(document::Column::ProjectId.eq(project_id))
        .filter(document::Column::SourceSnapshotId.eq(source.snapshot_id))
        .filter(document::Column::SourceChapterId.eq(source.source_chapter_id))
        .filter(document::Column::Kind.eq(kind))
        .order_by_desc(document::Column::Version)
        .one(db)
        .await?;
    let version = previous
        .map(|row| {
            row.version
                .checked_add(1)
                .ok_or_else(|| failure("历史改编版本溢出"))
        })
        .transpose()?
        .unwrap_or(1);
    let inputs = serde_json::to_value(AuthoringDocumentInputs {
        sources: vec![AuthoringSourceRef {
            root_project_id: source.root_project_id,
            source_chapter_id: source.source_chapter_id,
            source_snapshot_id: source.snapshot_id,
            source_snapshot_chapter_id: source.id,
            content_sha256: source.content_hash.clone(),
        }],
        video_reference: None,
        dependencies,
    })
    .map_err(json_error)?;
    let hash = a4n_domain::generation::canonical_json_sha256(&(&inputs, &payload))
        .map_err(|error| failure(error.to_string()))?;
    document::ActiveModel {
        project_id: Set(project_id),
        source_snapshot_id: Set(source.snapshot_id),
        source_chapter_id: Set(Some(source.source_chapter_id)),
        kind: Set(kind.into()),
        version: Set(version),
        revision: Set(1),
        status: Set("superseded".into()),
        inputs_json: Set(inputs),
        document_json: Set(payload),
        document_sha256: Set(hash),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
}

pub(super) async fn create_historical_comic<C: ConnectionTrait>(
    db: &C,
    root: &novel_project::Model,
    source: &source_snapshot_chapter::Model,
    now: sea_orm::prelude::DateTime,
) -> Result<i64, DbErr> {
    let snapshot = project_source_snapshot::Entity::find_by_id(source.snapshot_id)
        .one(db)
        .await?
        .ok_or_else(|| failure("历史漫画来源快照不存在"))?;
    let start = number(&snapshot.chapter_scope_json, "start")?;
    let end = number(&snapshot.chapter_scope_json, "end")?;
    let count = i32::try_from(
        end.checked_sub(start)
            .and_then(|value| value.checked_add(1))
            .filter(|value| *value > 0)
            .ok_or_else(|| failure("历史漫画范围无效"))?,
    )
    .map_err(|error| failure(error.to_string()))?;
    let project = novel_project::ActiveModel {
        user_id: Set(root.user_id),
        title: Set(format!("{} · 历史漫画改编（{start}–{end}章）", root.title)),
        description: Set("由原作项目中的已有漫画迁移，保留原图和页面编辑。".into()),
        project_type: Set(root.project_type.clone()),
        project_kind: Set("comic".into()),
        root_project_id: Set(Some(root.id)),
        source_snapshot_id: Set(Some(source.snapshot_id)),
        source_sync_status: Set("root_updated".into()),
        status: Set("draft".into()),
        lifecycle_gate: Set("active".into()),
        target_chapters: Set(count),
        current_chapter: Set(0),
        total_chapters: Set(Some(count)),
        chapters_per_part: Set(root.chapters_per_part),
        avatar_svg: Set(root.avatar_svg.clone()),
        avatar_animal: Set(root.avatar_animal.clone()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await?;
    record_receipt(
        db,
        "historical_comic",
        root.id,
        project.id,
        project.id,
        &serde_json::to_value(&project).map_err(json_error)?,
        &hash_value(&json!({"snapshot": source.snapshot_id}))?,
    )
    .await?;
    Ok(project.id)
}

fn number(value: &Value, field: &str) -> Result<i64, DbErr> {
    value
        .get(field)
        .and_then(Value::as_i64)
        .filter(|value| *value > 0)
        .ok_or_else(|| failure(format!("历史字段 {field} 必须是正整数")))
}
fn failure(message: impl Into<String>) -> DbErr {
    DbErr::Custom(message.into())
}
fn json_error(error: serde_json::Error) -> DbErr {
    failure(error.to_string())
}
