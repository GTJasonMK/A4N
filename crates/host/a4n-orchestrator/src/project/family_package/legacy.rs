//! Explicit upgrade at the archive boundary. Current manifests remain strict.

use std::collections::HashMap;

use a4n_db::project::bundle::ProjectBundleHistoryRows;
use a4n_db_models::models::project::{
    media_authoring_document, media_authoring_migration_receipt, source_snapshot_chapter,
};
use a4n_domain::media_authoring::{
    AuthoringDocumentInputs, AuthoringDocumentRef, AuthoringSourceRef,
};
use serde_json::{Value, json};

use super::ProjectFamilyPackage;

mod film;
mod galgame;
mod history;
mod manga;
mod source;

impl ProjectFamilyPackage {
    pub(crate) fn from_manifest(
        original: Value,
        history: &mut ProjectBundleHistoryRows,
        media_bytes: &mut HashMap<String, Vec<u8>>,
        reserved_id: i64,
    ) -> Result<Self, String> {
        let legacy = matches!(
            original["version"].as_str(),
            Some("project_family_v5" | "project_family_v6" | "project_family_v7")
        );
        let mut manifest = original.clone();
        if legacy {
            let root_id = manifest["root_project_id"]
                .as_i64()
                .filter(|id| *id > 0)
                .ok_or("旧项目包缺少原作身份")?;
            for table in ["chapter_visual_plans", "chapter_shooting_scripts"] {
                for row in manifest
                    .get_mut(table)
                    .and_then(Value::as_array_mut)
                    .ok_or_else(|| format!("旧项目包 {table} 必须是数组"))?
                {
                    row.as_object_mut()
                        .ok_or("旧项目包创作记录必须是对象")?
                        .insert("project_id".into(), root_id.into());
                }
            }
            manifest["authoring_documents"] = json!([]);
            manifest["authoring_receipts"] = json!([]);
        }
        let mut package: Self = serde_json::from_value(manifest).map_err(error)?;
        if !legacy {
            return Ok(package);
        }
        let mut next = maximum_manifest_id(&original).max(reserved_id);
        package.upgrade_shooting_contract(history)?;
        film::upgrade(&mut package, history, &mut next)?;
        manga::upgrade(&mut package, media_bytes, &mut next)?;
        galgame::upgrade(&mut package, &mut next)?;
        history::upgrade(&mut package, history, &mut next)?;
        archive_source_sync_history(&mut package)?;
        source::remove_film_hashes(&mut package)?;
        package.version = super::PROJECT_FAMILY_PACKAGE_VERSION.into();
        let after = hash(&package)?;
        package
            .authoring_receipts
            .push(media_authoring_migration_receipt::Model {
                kind: "legacy_package".into(),
                old_id: package.root_project_id,
                new_id: package.root_project_id,
                project_id: package.root_project_id,
                before_json: original,
                after_sha256: after,
            });
        Ok(package)
    }
}

fn archive_source_sync_history(package: &mut ProjectFamilyPackage) -> Result<(), String> {
    let owners = package
        .source_sync_conflicts
        .iter()
        .map(|row| (row.id, row.derived_project_id))
        .collect::<HashMap<_, _>>();
    let mut receipts = Vec::new();
    let conflicts = package.source_sync_conflicts.iter_mut().filter_map(|row| {
        row.decision_json.as_mut().map(|payload| {
            (
                "source_sync_conflict_archive",
                row.id,
                Some(row.derived_project_id),
                payload,
            )
        })
    });
    let decisions = package.source_sync_decisions.iter_mut().map(|row| {
        (
            "source_sync_decision_archive",
            row.id,
            owners.get(&row.conflict_id).copied(),
            &mut row.decision_payload_json,
        )
    });
    for (kind, id, project_id, payload) in conflicts.chain(decisions) {
        let original = payload.clone();
        if a4n_domain::media_authoring::legacy::source_sync::archive_legacy_source_sync_payload(
            payload,
        )? {
            receipts.push(media_authoring_migration_receipt::Model {
                kind: kind.into(),
                old_id: id,
                new_id: id,
                project_id: project_id.ok_or("旧同步决策的项目不存在")?,
                before_json: original,
                after_sha256: hash(payload)?,
            });
        }
    }
    package.authoring_receipts.extend(receipts);
    Ok(())
}

pub(crate) fn maximum_manifest_id(value: &Value) -> i64 {
    match value {
        Value::Object(object) => object
            .iter()
            .map(|(key, value)| {
                let id = if key == "id" || key.ends_with("_id") {
                    value.as_i64().unwrap_or(0)
                } else {
                    0
                };
                id.max(maximum_manifest_id(value))
            })
            .max()
            .unwrap_or(0),
        Value::Array(values) => values.iter().map(maximum_manifest_id).max().unwrap_or(0),
        _ => 0,
    }
}

fn next_id(next: &mut i64) -> Result<i64, String> {
    *next = next
        .checked_add(1)
        .filter(|id| *id > 0)
        .ok_or("旧包记录 ID 溢出")?;
    Ok(*next)
}

fn hash(value: &impl serde::Serialize) -> Result<String, String> {
    a4n_domain::generation::canonical_json_sha256(value).map_err(error)
}

fn error(error: impl std::fmt::Display) -> String {
    error.to_string()
}

fn insert_document(
    package: &mut ProjectFamilyPackage,
    project_id: i64,
    source: &source_snapshot_chapter::Model,
    kind: &str,
    payload: Value,
    dependencies: Vec<AuthoringDocumentRef>,
    adopted: bool,
    next: &mut i64,
) -> Result<media_authoring_document::Model, String> {
    let project = package
        .projects
        .iter()
        .find(|project| project.id == project_id)
        .ok_or("历史改编所属项目不存在")?;
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
    .map_err(error)?;
    let previous = package
        .authoring_documents
        .iter()
        .filter(|doc| {
            doc.project_id == project_id
                && doc.source_snapshot_id == source.snapshot_id
                && doc.source_chapter_id == Some(source.source_chapter_id)
                && doc.kind == kind
        })
        .collect::<Vec<_>>();
    let version = previous
        .iter()
        .map(|doc| doc.version)
        .max()
        .unwrap_or(0)
        .checked_add(1)
        .ok_or("历史改编版本溢出")?;
    let status = if adopted && !previous.iter().any(|doc| doc.status == "adopted") {
        "adopted"
    } else {
        "superseded"
    };
    let doc = media_authoring_document::Model {
        id: next_id(next)?,
        project_id,
        source_snapshot_id: source.snapshot_id,
        source_chapter_id: Some(source.source_chapter_id),
        kind: kind.into(),
        version,
        revision: 1,
        status: status.into(),
        document_sha256: hash(&(&inputs, &payload))?,
        inputs_json: inputs,
        document_json: payload,
        created_at: project.created_at,
        updated_at: project.updated_at,
    };
    package.authoring_documents.push(doc.clone());
    Ok(doc)
}

fn receipt(
    package: &mut ProjectFamilyPackage,
    kind: &str,
    old_id: i64,
    new_id: i64,
    project_id: i64,
    before: Value,
    after: &impl serde::Serialize,
) -> Result<(), String> {
    package
        .authoring_receipts
        .push(media_authoring_migration_receipt::Model {
            kind: kind.into(),
            old_id,
            new_id,
            project_id,
            before_json: before,
            after_sha256: hash(after)?,
        });
    Ok(())
}
