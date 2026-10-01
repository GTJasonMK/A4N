use super::OutlineDbError;
use crate::scene_profile::scene_profile_record;
use a4n_db_models::models::{
    asset::object_profile,
    chapter::{chapter, chapter_outline, chapter_outline_provenance, part_outline},
    character::{foreshadowing_index, protagonist_profile},
    project::{blueprint_character, blueprint_relationship, novel_blueprint, novel_project},
    scene::scene_profile,
};
use a4n_domain::chapter::{ChapterOutlineRecord, PartOutlineRecord};
use a4n_domain::project::ProjectRecord;
use a4n_domain::source_text_hash;
use a4n_domain::{object_profile::ObjectProfileRecord, scene_profile::SceneProfileRecord};
use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DatabaseTransaction, DbBackend, DbErr,
    EntityTrait, QueryFilter, QueryOrder, QuerySelect, Select, Statement, TransactionTrait,
};
use serde_json::{Value, json};

use super::record_mapping::{chapter_outline_record_from_model, part_outline_record_from_model};

pub const OUTLINE_GENERATION_SOURCE_CHANGED: &str = "outline_generation_source_changed";

pub struct OutlineGenerationSourceRows {
    pub project: ProjectRecord,
    pub chapters: Vec<chapter::Model>,
    pub blueprint: Option<novel_blueprint::Model>,
    pub blueprint_characters: Vec<blueprint_character::Model>,
    pub blueprint_relationships: Vec<blueprint_relationship::Model>,
    pub character_profiles: Vec<protagonist_profile::Model>,
    pub objects: Vec<ObjectProfileRecord>,
    pub scenes: Vec<SceneProfileRecord>,
    pub foreshadowing: Vec<foreshadowing_index::Model>,
    pub parts: Vec<PartOutlineRecord>,
    pub chapter_outlines: Vec<ChapterOutlineRecord>,
    pub chapter_outline_provenance: Vec<chapter_outline_provenance::Model>,
}

pub async fn load_outline_generation_source_rows<C>(
    db: &C,
    project_id: i64,
) -> Result<Option<OutlineGenerationSourceRows>, DbErr>
where
    C: ConnectionTrait,
{
    load_outline_generation_source_rows_with_lock(db, project_id, false).await
}

pub async fn load_outline_generation_source_rows_for_update<C>(
    db: &C,
    project_id: i64,
) -> Result<Option<OutlineGenerationSourceRows>, DbErr>
where
    C: ConnectionTrait,
{
    load_outline_generation_source_rows_with_lock(db, project_id, true).await
}

pub async fn lock_outline_generation_sources(
    db: &DatabaseConnection,
    project_id: i64,
    expected_source_snapshot: &Value,
) -> Result<DatabaseTransaction, OutlineDbError> {
    let tx = db.begin().await?;
    let rows = load_outline_generation_source_rows_for_update(&tx, project_id)
        .await?
        .ok_or_else(outline_generation_source_changed)?;
    if rows.project.lifecycle_gate != crate::generation::PROJECT_GATE_ACTIVE {
        return Err(OutlineDbError::InvalidState {
            current: rows.project.lifecycle_gate,
            operation: "保存生成的大纲".to_string(),
        });
    }
    if expected_source_snapshot != &outline_generation_source_snapshot(&rows) {
        return Err(outline_generation_source_changed());
    }
    Ok(tx)
}

pub fn outline_generation_source_snapshot(rows: &OutlineGenerationSourceRows) -> Value {
    json!({
        "project": {
            "id": rows.project.id,
            "title": rows.project.title,
            "description": rows.project.description,
            "project_type": rows.project.project_type,
            "target_chapters": rows.project.target_chapters,
            "total_chapters": rows.project.total_chapters,
            "chapters_per_part": rows.project.chapters_per_part,
            "raw_blueprint": rows.project.raw_blueprint,
            "import_analysis_progress": rows.project.import_analysis_progress,
        },
        "chapters": rows.chapters.iter().map(chapter_source_snapshot).collect::<Vec<_>>(),
        "blueprint": rows.blueprint,
        "blueprint_characters": rows.blueprint_characters,
        "blueprint_relationships": rows.blueprint_relationships,
        "character_profiles": rows.character_profiles,
        "objects": rows.objects,
        "scenes": rows.scenes,
        "foreshadowing": rows.foreshadowing,
        "parts": rows.parts,
        "chapter_outlines": rows.chapter_outlines,
        "chapter_outline_provenance": rows.chapter_outline_provenance,
    })
}

async fn load_outline_generation_source_rows_with_lock<C>(
    db: &C,
    project_id: i64,
    lock: bool,
) -> Result<Option<OutlineGenerationSourceRows>, DbErr>
where
    C: ConnectionTrait,
{
    let backend = db.get_database_backend();
    let project = if lock {
        crate::project::project_by_id_for_update(db, project_id).await?
    } else {
        novel_project::Entity::find_by_id(project_id)
            .one(db)
            .await?
    };
    let Some(project) = project else {
        return Ok(None);
    };
    let project = crate::project::project_record_from_model(project);
    if lock {
        lock_outline_source_insert_tables(db, backend).await?;
    }
    if !lock {
        let (
            chapters_result,
            blueprint_result,
            blueprint_characters_result,
            blueprint_relationships_result,
            character_profiles_result,
            objects_result,
            scenes_result,
            foreshadowing_result,
            parts_result,
            chapter_outlines_result,
        ) = futures::join!(
            chapter::Entity::find_current()
                .filter(chapter::Column::ProjectId.eq(project_id))
                .order_by_asc(chapter::Column::ChapterNumber)
                .order_by_asc(chapter::Column::Id)
                .all(db),
            novel_blueprint::Entity::find()
                .filter(novel_blueprint::Column::ProjectId.eq(project_id))
                .order_by_asc(novel_blueprint::Column::Id)
                .one(db),
            blueprint_character::Entity::find()
                .filter(blueprint_character::Column::ProjectId.eq(project_id))
                .order_by_asc(blueprint_character::Column::Id)
                .all(db),
            blueprint_relationship::Entity::find()
                .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
                .order_by_asc(blueprint_relationship::Column::Id)
                .all(db),
            protagonist_profile::Entity::find()
                .filter(protagonist_profile::Column::ProjectId.eq(project_id))
                .order_by_asc(protagonist_profile::Column::Id)
                .all(db),
            object_profile::Entity::find()
                .filter(object_profile::Column::ProjectId.eq(project_id))
                .order_by_asc(object_profile::Column::Id)
                .all(db),
            scene_profile::Entity::find()
                .filter(scene_profile::Column::ProjectId.eq(project_id))
                .order_by_asc(scene_profile::Column::Id)
                .all(db),
            foreshadowing_index::Entity::find()
                .filter(foreshadowing_index::Column::ProjectId.eq(project_id))
                .order_by_asc(foreshadowing_index::Column::Id)
                .all(db),
            part_outline::Entity::find()
                .filter(part_outline::Column::ProjectId.eq(project_id))
                .order_by_asc(part_outline::Column::PartNumber)
                .order_by_asc(part_outline::Column::Id)
                .all(db),
            chapter_outline::Entity::find()
                .filter(chapter_outline::Column::ProjectId.eq(project_id))
                .order_by_asc(chapter_outline::Column::ChapterNumber)
                .order_by_asc(chapter_outline::Column::Id)
                .all(db),
        );
        let chapters = chapters_result?;
        let blueprint = blueprint_result?;
        let blueprint_characters = blueprint_characters_result?;
        let blueprint_relationships = blueprint_relationships_result?;
        let character_profiles = character_profiles_result?;
        let objects = objects_result?
            .into_iter()
            .map(crate::asset::object_profile_record)
            .collect();
        let scenes = scenes_result?
            .into_iter()
            .map(scene_profile_record)
            .collect();
        let foreshadowing = foreshadowing_result?;
        let parts = parts_result?;
        let chapter_outlines = chapter_outlines_result?;
        let outline_ids = chapter_outlines
            .iter()
            .map(|outline| outline.id)
            .collect::<Vec<_>>();
        let chapter_outline_provenance = if outline_ids.is_empty() {
            Vec::new()
        } else {
            chapter_outline_provenance::Entity::find()
                .filter(chapter_outline_provenance::Column::OutlineId.is_in(outline_ids))
                .order_by_asc(chapter_outline_provenance::Column::OutlineId)
                .all(db)
                .await?
        };
        return Ok(Some(OutlineGenerationSourceRows {
            project,
            chapters,
            blueprint,
            blueprint_characters,
            blueprint_relationships,
            character_profiles,
            objects,
            scenes,
            foreshadowing,
            parts: parts
                .into_iter()
                .map(part_outline_record_from_model)
                .collect(),
            chapter_outlines: chapter_outlines
                .into_iter()
                .map(chapter_outline_record_from_model)
                .collect(),
            chapter_outline_provenance,
        }));
    }
    let chapters = maybe_lock(
        chapter::Entity::find_current()
            .filter(chapter::Column::ProjectId.eq(project_id))
            .order_by_asc(chapter::Column::ChapterNumber)
            .order_by_asc(chapter::Column::Id),
        backend,
        lock,
    )
    .all(db)
    .await?;
    let blueprint = maybe_lock(
        novel_blueprint::Entity::find()
            .filter(novel_blueprint::Column::ProjectId.eq(project_id))
            .order_by_asc(novel_blueprint::Column::Id),
        backend,
        lock,
    )
    .one(db)
    .await?;
    let blueprint_characters = maybe_lock(
        blueprint_character::Entity::find()
            .filter(blueprint_character::Column::ProjectId.eq(project_id))
            .order_by_asc(blueprint_character::Column::Id),
        backend,
        lock,
    )
    .all(db)
    .await?;
    let blueprint_relationships = maybe_lock(
        blueprint_relationship::Entity::find()
            .filter(blueprint_relationship::Column::ProjectId.eq(project_id))
            .order_by_asc(blueprint_relationship::Column::Id),
        backend,
        lock,
    )
    .all(db)
    .await?;
    let character_profiles = maybe_lock(
        protagonist_profile::Entity::find()
            .filter(protagonist_profile::Column::ProjectId.eq(project_id))
            .order_by_asc(protagonist_profile::Column::Id),
        backend,
        lock,
    )
    .all(db)
    .await?;
    let objects = maybe_lock(
        object_profile::Entity::find()
            .filter(object_profile::Column::ProjectId.eq(project_id))
            .order_by_asc(object_profile::Column::Id),
        backend,
        lock,
    )
    .all(db)
    .await?
    .into_iter()
    .map(crate::asset::object_profile_record)
    .collect();
    let scenes = maybe_lock(
        scene_profile::Entity::find()
            .filter(scene_profile::Column::ProjectId.eq(project_id))
            .order_by_asc(scene_profile::Column::Id),
        backend,
        lock,
    )
    .all(db)
    .await?
    .into_iter()
    .map(scene_profile_record)
    .collect();
    let foreshadowing = maybe_lock(
        foreshadowing_index::Entity::find()
            .filter(foreshadowing_index::Column::ProjectId.eq(project_id))
            .order_by_asc(foreshadowing_index::Column::Id),
        backend,
        lock,
    )
    .all(db)
    .await?;
    let parts = maybe_lock(
        part_outline::Entity::find()
            .filter(part_outline::Column::ProjectId.eq(project_id))
            .order_by_asc(part_outline::Column::PartNumber)
            .order_by_asc(part_outline::Column::Id),
        backend,
        lock,
    )
    .all(db)
    .await?;
    let chapter_outlines = maybe_lock(
        chapter_outline::Entity::find()
            .filter(chapter_outline::Column::ProjectId.eq(project_id))
            .order_by_asc(chapter_outline::Column::ChapterNumber)
            .order_by_asc(chapter_outline::Column::Id),
        backend,
        lock,
    )
    .all(db)
    .await?;
    let outline_ids = chapter_outlines
        .iter()
        .map(|outline| outline.id)
        .collect::<Vec<_>>();
    let chapter_outline_provenance = if outline_ids.is_empty() {
        Vec::new()
    } else {
        maybe_lock(
            chapter_outline_provenance::Entity::find()
                .filter(chapter_outline_provenance::Column::OutlineId.is_in(outline_ids))
                .order_by_asc(chapter_outline_provenance::Column::OutlineId),
            backend,
            lock,
        )
        .all(db)
        .await?
    };

    Ok(Some(OutlineGenerationSourceRows {
        project,
        chapters,
        blueprint,
        blueprint_characters,
        blueprint_relationships,
        character_profiles,
        objects,
        scenes,
        foreshadowing,
        parts: parts
            .into_iter()
            .map(part_outline_record_from_model)
            .collect(),
        chapter_outlines: chapter_outlines
            .into_iter()
            .map(chapter_outline_record_from_model)
            .collect(),
        chapter_outline_provenance,
    }))
}

async fn lock_outline_source_insert_tables<C>(db: &C, backend: DbBackend) -> Result<(), DbErr>
where
    C: ConnectionTrait,
{
    if backend != DbBackend::Postgres {
        return Ok(());
    }
    // ponytail: these brief SHARE locks close insert phantoms without adding a second revision
    // system; replace with project-scoped source revisions only if cross-project contention is real.
    db.execute(Statement::from_string(
        backend,
        r#"LOCK TABLE
            "chapter",
            "chapter_outline",
            "chapter_outline_provenance",
            "part_outline",
            "novel_blueprint",
            "blueprint_version",
            "blueprint_character",
            "blueprint_relationship",
            "protagonist_profile",
            "object_profile",
            "scene_profile",
            "foreshadowing_index"
           IN SHARE MODE"#
            .to_string(),
    ))
    .await?;
    Ok(())
}

fn maybe_lock<E>(query: Select<E>, backend: DbBackend, lock: bool) -> Select<E>
where
    E: EntityTrait,
{
    if lock && backend != DbBackend::Sqlite {
        query.lock_exclusive()
    } else {
        query
    }
}

fn outline_generation_source_changed() -> OutlineDbError {
    OutlineDbError::InvalidState {
        current: OUTLINE_GENERATION_SOURCE_CHANGED.to_string(),
        operation: "保存生成的大纲（项目正式资料已变化，请重新生成）".to_string(),
    }
}

fn chapter_source_snapshot(chapter: &chapter::Model) -> Value {
    json!({
        "id": chapter.id,
        "project_id": chapter.project_id,
        "outline_id": chapter.outline_id,
        "chapter_number": chapter.chapter_number,
        "title": chapter.title,
        "content_sha256": source_text_hash(&chapter.content),
        "word_count": chapter.word_count,
        "status": chapter.status,
        "summary": chapter.summary,
        "style_tags": chapter.style_tags,
        "analysis_data": chapter.analysis_data,
        "created_at": chapter.created_at,
        "updated_at": chapter.updated_at,
    })
}
