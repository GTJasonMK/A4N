use crate::project::family::import::{
    ImportIdMaps, ProjectFamilyImportError, ProjectFamilyImportPackage, hash_json, map_optional,
    map_required, remap_chapter_hashes_json, remap_profile_hashes_json, remap_profile_id,
    remap_snapshot_outline_source_id, remap_source_snapshot_outline_payload,
    remap_source_snapshot_profile_payload, snapshot_profile_key,
};
use a4n_db_models::models::project::{
    project_source_snapshot, source_snapshot_chapter, source_snapshot_outline,
    source_snapshot_profile,
};
use sea_orm::{
    ActiveModelTrait, ConnectionTrait, DatabaseTransaction, DbBackend, DbErr, Set, Statement,
};
use serde_json::json;
use std::collections::{BTreeSet, HashMap};

pub(in crate::project::family::import::import_rows) async fn insert_source_snapshots(
    txn: &DatabaseTransaction,
    package: &ProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    reserve_snapshot_outline_sources(txn, package, maps).await?;
    let new_root_id = map_required(
        &maps.project_ids,
        package.root_project_id,
        "源文本快照 root_project_id",
    )?;
    let snapshot_profiles = package
        .source_snapshot_profiles
        .iter()
        .map(|profile| {
            (
                (
                    profile.snapshot_id,
                    snapshot_profile_key(&profile.profile_kind, profile.source_profile_id),
                ),
                profile,
            )
        })
        .collect::<HashMap<_, _>>();
    for snapshot in &package.source_snapshots {
        if snapshot.root_project_id != package.root_project_id {
            return Err(ProjectFamilyImportError::InvalidPackage(format!(
                "源文本快照不属于导入包根项目: {}",
                snapshot.id
            )));
        }
        let blueprint_version_id = map_optional(
            &maps.blueprint_version_ids,
            snapshot.blueprint_version_id,
            "源文本快照 blueprint_version_id",
        )?;
        let chapter_hashes_json = remap_chapter_hashes_json(&snapshot.chapter_hashes_json, maps)?;
        let profile_hashes_json = remap_profile_hashes_json(
            &snapshot.profile_hashes_json,
            snapshot.id,
            &snapshot_profiles,
            maps,
        )?;
        let source_version_hash = hash_json(&json!({
            "root_project_id": new_root_id,
            "chapter_scope": snapshot.chapter_scope_json,
            "chapter_hashes": chapter_hashes_json,
            "profile_hashes": profile_hashes_json,
            "blueprint_version_id": blueprint_version_id,
        }));
        let active = project_source_snapshot::ActiveModel {
            root_project_id: Set(new_root_id),
            source_version_hash: Set(source_version_hash.clone()),
            blueprint_version_id: Set(blueprint_version_id),
            chapter_scope_json: Set(snapshot.chapter_scope_json.clone()),
            chapter_hashes_json: Set(chapter_hashes_json),
            profile_hashes_json: Set(profile_hashes_json),
            created_at: Set(snapshot.created_at),
            ..Default::default()
        };
        let inserted = active.insert(txn).await.map_err(|source| {
            ProjectFamilyImportError::DatabaseContext {
                context: "导入源文本快照失败",
                source,
            }
        })?;
        maps.source_snapshot_ids.insert(snapshot.id, inserted.id);
        maps.source_snapshot_version_hashes
            .insert(snapshot.id, source_version_hash);
    }
    insert_mapped_import_rows!(
        txn,
        &package.source_snapshot_chapters,
        chapter,
        source_snapshot_chapter::ActiveModel,
        maps.source_snapshot_chapter_ids,
        "导入源快照章节失败",
        |active| {
            active.snapshot_id = Set(map_required(
                &maps.source_snapshot_ids,
                chapter.snapshot_id,
                "源快照章节 snapshot_id",
            )?);
            active.root_project_id = Set(new_root_id);
            active.source_chapter_id = Set(map_required(
                &maps.chapter_ids,
                chapter.source_chapter_id,
                "源快照章节 source_chapter_id",
            )?);
            active.visual_plan_id = Set(None);
            active.visual_plan_version = Set(None);
            active.visual_plan_sha256 = Set(None);
        }
    );
    insert_import_rows!(
        txn,
        &package.source_snapshot_outlines,
        outline,
        source_snapshot_outline::ActiveModel,
        "导入源快照大纲失败",
        |active| {
            let (payload_json, payload_hash) =
                remap_source_snapshot_outline_payload(outline, maps)?;
            active.snapshot_id = Set(map_required(
                &maps.source_snapshot_ids,
                outline.snapshot_id,
                "源快照大纲 snapshot_id",
            )?);
            active.root_project_id = Set(new_root_id);
            active.source_outline_id = Set(remap_snapshot_outline_source_id(outline, maps)?);
            active.payload_json = Set(payload_json);
            active.payload_hash = Set(payload_hash);
        }
    );
    insert_import_rows!(
        txn,
        &package.source_snapshot_profiles,
        profile,
        source_snapshot_profile::ActiveModel,
        "导入源快照档案失败",
        |active| {
            let (payload_json, payload_hash) =
                remap_source_snapshot_profile_payload(profile, maps)?;
            active.snapshot_id = Set(map_required(
                &maps.source_snapshot_ids,
                profile.snapshot_id,
                "源快照档案 snapshot_id",
            )?);
            active.root_project_id = Set(new_root_id);
            active.source_profile_id = Set(remap_profile_id(
                &profile.profile_kind,
                profile.source_profile_id,
                maps,
                "源快照档案 source_profile_id",
            )?);
            active.payload_json = Set(payload_json);
            active.payload_hash = Set(payload_hash);
        }
    );
    Ok(())
}

async fn reserve_snapshot_outline_sources(
    txn: &DatabaseTransaction,
    package: &ProjectFamilyImportPackage,
    maps: &mut ImportIdMaps,
) -> Result<(), ProjectFamilyImportError> {
    maps.source_part_outline_ids
        .clone_from(&maps.part_outline_ids);
    maps.source_chapter_outline_ids
        .clone_from(&maps.chapter_outline_ids);
    // Deleted outline payloads live in snapshots. Reserve their identities without
    // recreating current outlines or admitting them into live-anchor ID lookups.
    for (kind, table, ids) in [
        ("part", "part_outline", &mut maps.source_part_outline_ids),
        (
            "chapter",
            "chapter_outline",
            &mut maps.source_chapter_outline_ids,
        ),
    ] {
        let missing = package
            .source_snapshot_outlines
            .iter()
            .filter(|outline| outline.outline_kind == kind)
            .map(|outline| outline.source_outline_id)
            .filter(|id| !ids.contains_key(id))
            .collect::<BTreeSet<_>>();
        if missing.is_empty() {
            continue;
        }
        let rows = txn.query_all(Statement::from_sql_and_values(
            DbBackend::Postgres,
            "SELECT nextval(pg_get_serial_sequence($1, 'id')) AS id FROM generate_series(1, $2::bigint)",
            [table.into(), (missing.len() as i64).into()],
        )).await?;
        if rows.len() != missing.len() {
            return Err(ProjectFamilyImportError::Database(DbErr::Custom(
                "冻结大纲源身份分配数量不一致".into(),
            )));
        }
        for (old, row) in missing.into_iter().zip(rows) {
            ids.insert(old, row.try_get("", "id")?);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sea_orm::TransactionTrait;

    #[tokio::test]
    async fn archived_outline_sources_reserve_ids_without_recreating_current_outlines() {
        let db = a4n_migration::test_support::connect_test_database().await;
        let tx = db.begin().await.unwrap();
        tx.execute_unprepared("CREATE TEMP TABLE part_outline (id bigserial PRIMARY KEY) ON COMMIT DROP; CREATE TEMP TABLE chapter_outline (id bigserial PRIMARY KEY) ON COMMIT DROP; INSERT INTO part_outline DEFAULT VALUES;").await.unwrap();
        let mut maps = ImportIdMaps::default();
        maps.part_outline_ids.insert(101, 1);
        maps.project_ids.insert(1, 501);
        let outlines = [
            (1, "part", 202),
            (2, "part", 202),
            (3, "chapter", 303),
            (4, "chapter", 303),
        ]
        .into_iter()
        .map(|(id, kind, source)| {
            let payload = json!({"id":source,"project_id":1,"part_id":202});
            source_snapshot_outline::Model {
                id,
                snapshot_id: id,
                root_project_id: 1,
                outline_kind: kind.into(),
                source_outline_id: source,
                payload_hash: hash_json(&payload),
                payload_json: payload,
            }
        })
        .collect();
        let package = ProjectFamilyImportPackage {
            source_snapshot_outlines: outlines,
            ..Default::default()
        };
        reserve_snapshot_outline_sources(&tx, &package, &mut maps)
            .await
            .unwrap();
        assert_eq!(maps.part_outline_ids, HashMap::from([(101, 1)]));
        assert!(maps.chapter_outline_ids.is_empty());
        assert_eq!(
            maps.source_part_outline_ids,
            HashMap::from([(101, 1), (202, 2)])
        );
        assert_eq!(maps.source_chapter_outline_ids, HashMap::from([(303, 1)]));
        let (payload, hash) =
            remap_source_snapshot_outline_payload(&package.source_snapshot_outlines[2], &maps)
                .unwrap();
        assert_eq!(payload["id"], 1);
        assert_eq!(payload["part_id"], 2);
        assert_eq!(payload["project_id"], 501);
        assert_eq!(hash, hash_json(&payload));
        let counts = tx.query_one(Statement::from_string(DbBackend::Postgres, "SELECT (SELECT COUNT(*) FROM part_outline) AS parts, (SELECT COUNT(*) FROM chapter_outline) AS chapters")).await.unwrap().unwrap();
        assert_eq!(counts.try_get::<i64>("", "parts").unwrap(), 1);
        assert_eq!(counts.try_get::<i64>("", "chapters").unwrap(), 0);
        let next = tx
            .query_one(Statement::from_string(
                DbBackend::Postgres,
                "INSERT INTO part_outline DEFAULT VALUES RETURNING id",
            ))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(next.try_get::<i64>("", "id").unwrap(), 3);
        tx.rollback().await.unwrap();
    }
}
