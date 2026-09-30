use a4n_db_models::models::project::{
    project_source_snapshot, source_snapshot_chapter, source_snapshot_outline,
    source_snapshot_profile, text_anchor,
};
use serde_json::{Value, json};

use super::{ProjectFamilyPackage, error, next_id};

fn hash(value: &impl serde::Serialize) -> Result<String, String> {
    a4n_domain::shooting_script::compat::serialized_sha256(value).map_err(error)
}

pub(super) fn source_for(
    package: &mut ProjectFamilyPackage,
    chapter_id: i64,
    snapshot: Option<i64>,
    snapshot_chapter: Option<i64>,
    content_hash: &str,
    next: &mut i64,
) -> Result<source_snapshot_chapter::Model, String> {
    match (snapshot, snapshot_chapter) {
        (Some(snapshot), Some(id)) => {
            let source = package
                .source_snapshot_chapters
                .iter()
                .find(|row| row.id == id)
                .ok_or("旧包原文快照章节不存在")?;
            if source.snapshot_id != snapshot
                || source.source_chapter_id != chapter_id
                || source.root_project_id != package.root_project_id
                || source.content_hash != content_hash
                || a4n_domain::source_text_hash(&source.content) != content_hash
            {
                return Err("旧包原文快照身份或内容不一致".into());
            }
            Ok(source.clone())
        }
        (None, None) => freeze_source(package, chapter_id, content_hash, next),
        _ => Err("旧包原文快照身份不完整".into()),
    }
}

fn freeze_source(
    package: &mut ProjectFamilyPackage,
    chapter_id: i64,
    content_hash: &str,
    next: &mut i64,
) -> Result<source_snapshot_chapter::Model, String> {
    let chapter = package
        .text_core
        .chapters
        .iter()
        .find(|chapter| chapter.id == chapter_id)
        .ok_or("旧包原文章节不存在")?
        .clone();
    if a4n_domain::source_text_hash(&chapter.content) != content_hash {
        let matches = package
            .source_snapshot_chapters
            .iter()
            .filter(|row| row.source_chapter_id == chapter_id && row.content_hash == content_hash)
            .collect::<Vec<_>>();
        let source = matches
            .first()
            .ok_or("旧包缺少创作记录实际使用的原文版本")?;
        let snapshot = package
            .source_snapshots
            .iter()
            .find(|row| row.id == source.snapshot_id)
            .ok_or("旧包快照不存在")?;
        for other in &matches {
            let other_snapshot = package
                .source_snapshots
                .iter()
                .find(|row| row.id == other.snapshot_id)
                .ok_or("旧包快照不存在")?;
            if other_snapshot.profile_hashes_json != snapshot.profile_hashes_json
                || other_snapshot.blueprint_version_id != snapshot.blueprint_version_id
                || other.chapter_number != source.chapter_number
                || a4n_domain::source_text_hash(&other.content) != content_hash
            {
                return Err("旧包原文存在不同档案版本，无法确定旧创作来源".into());
            }
        }
        return Ok((*source).clone());
    }
    // Reuse a snapshot created by this upgrade, never infer that a different
    // historical profile capture was the live source used by the author.
    if let Some(source) = package.source_snapshot_chapters.iter().find(|row| {
        row.source_chapter_id == chapter_id
            && row.content_hash == content_hash
            && package.authoring_receipts.iter().any(|receipt| {
                receipt.kind == "legacy_source_capture" && receipt.new_id == row.snapshot_id
            })
    }) {
        return Ok(source.clone());
    }
    let now = chapter.updated_at;
    let snapshot_id = next_id(next)?;
    let mut profiles = Vec::new();
    for row in &package.text_core.character_profiles {
        profiles.push((
            "character",
            row.id,
            serde_json::to_value(row).map_err(error)?,
        ));
    }
    for row in &package.text_core.object_profiles {
        profiles.push(("object", row.id, serde_json::to_value(row).map_err(error)?));
    }
    for row in package
        .text_core
        .scene_profiles
        .iter()
        .filter(|row| row.project_id == package.root_project_id)
    {
        profiles.push(("scene", row.id, serde_json::to_value(row).map_err(error)?));
    }
    let profile_hashes = profiles
        .iter()
        .map(|(kind, id, value)| Ok(json!({"kind": kind, "id": id, "hash": hash(value)?})))
        .collect::<Result<Vec<_>, String>>()?;
    let source = source_snapshot_chapter::Model {
        id: next_id(next)?,
        snapshot_id,
        root_project_id: package.root_project_id,
        source_chapter_id: chapter.id,
        chapter_number: chapter.chapter_number,
        title: chapter.title.clone(),
        content: chapter.content.clone(),
        summary: chapter.summary.clone(),
        analysis_data_json: chapter.analysis_data.clone(),
        content_hash: content_hash.into(),
        analysis_hash: chapter.analysis_data.as_ref().map(hash).transpose()?,
    };
    let mut snapshot = project_source_snapshot::Model {
        id: snapshot_id,
        root_project_id: package.root_project_id,
        source_version_hash: String::new(),
        blueprint_version_id: package
            .text_core
            .blueprint_versions
            .iter()
            .max_by_key(|row| (row.created_at, row.id))
            .map(|row| row.id),
        chapter_scope_json: json!({"start": chapter.chapter_number, "end": chapter.chapter_number}),
        chapter_hashes_json: json!([chapter_hash(&source)]),
        profile_hashes_json: json!(profile_hashes),
        created_at: now,
    };
    snapshot.source_version_hash = snapshot_hash(&snapshot)?;
    for (kind, id, value) in profiles {
        package
            .source_snapshot_profiles
            .push(source_snapshot_profile::Model {
                id: next_id(next)?,
                snapshot_id,
                root_project_id: package.root_project_id,
                profile_kind: kind.into(),
                source_profile_id: id,
                payload_hash: hash(&value)?,
                payload_json: value,
            });
    }
    let outlines = package
        .text_core
        .part_outlines
        .iter()
        .filter(|row| {
            row.start_chapter <= chapter.chapter_number && row.end_chapter >= chapter.chapter_number
        })
        .map(|row| Ok(("part", row.id, serde_json::to_value(row).map_err(error)?)))
        .chain(
            package
                .text_core
                .chapter_outlines
                .iter()
                .filter(|row| row.chapter_number == chapter.chapter_number)
                .map(|row| Ok(("chapter", row.id, serde_json::to_value(row).map_err(error)?))),
        )
        .collect::<Result<Vec<_>, String>>()?;
    for (kind, id, value) in outlines {
        package
            .source_snapshot_outlines
            .push(source_snapshot_outline::Model {
                id: next_id(next)?,
                snapshot_id,
                root_project_id: package.root_project_id,
                source_outline_id: id,
                outline_kind: kind.into(),
                payload_hash: hash(&value)?,
                payload_json: value,
            });
    }
    for span in a4n_domain::source_spans::source_spans_for_content(
        chapter.id,
        chapter.chapter_number,
        &chapter.content,
    ) {
        package.text_anchors.push(text_anchor::Model {
            id: next_id(next)?,
            project_id: package.root_project_id,
            source_chapter_id: Some(chapter.id),
            source_snapshot_id: Some(snapshot_id),
            source_snapshot_chapter_id: Some(source.id),
            source_kind: "chapter_content".into(),
            source_id: source.id,
            paragraph_index: i32::try_from(span.sentence_index).map_err(error)?,
            start_offset: i32::try_from(span.char_start).map_err(error)?,
            end_offset: i32::try_from(span.char_end).map_err(error)?,
            quote_hash: a4n_domain::source_text_hash(&span.text),
            quote: span.text,
            source_version_hash: source.content_hash.clone(),
            created_at: now,
        });
    }
    super::receipt(
        package,
        "legacy_source_capture",
        chapter.id,
        snapshot_id,
        package.root_project_id,
        serde_json::to_value(&chapter).map_err(error)?,
        &snapshot,
    )?;
    package.source_snapshots.push(snapshot);
    package.source_snapshot_chapters.push(source.clone());
    Ok(source)
}

pub(super) fn historical_project(
    package: &mut ProjectFamilyPackage,
    snapshot_id: i64,
    kind: &str,
    next: &mut i64,
) -> Result<i64, String> {
    let receipt_kind = format!("historical_{kind}");
    if let Some(receipt) = package.authoring_receipts.iter().find(|row| {
        row.kind == receipt_kind
            && row.before_json["source_snapshot_id"].as_i64() == Some(snapshot_id)
    }) {
        return Ok(receipt.project_id);
    }
    let mut project = package
        .projects
        .iter()
        .find(|row| row.id == package.root_project_id)
        .ok_or("旧包原作不存在")?
        .clone();
    let snapshot = package
        .source_snapshots
        .iter()
        .find(|row| row.id == snapshot_id)
        .ok_or("旧包快照不存在")?;
    let start = snapshot.chapter_scope_json["start"]
        .as_i64()
        .filter(|n| *n > 0)
        .ok_or("旧包章节范围无效")?;
    let end = snapshot.chapter_scope_json["end"]
        .as_i64()
        .ok_or("旧包章节范围无效")?;
    let count = end
        .checked_sub(start)
        .and_then(|n| n.checked_add(1))
        .filter(|n| *n > 0)
        .ok_or("旧包章节范围无效")?;
    project.id = next_id(next)?;
    let media = if kind == "video" { "视频" } else { "漫画" };
    project.title = format!("{} · 历史{media}改编（{start}–{end}章）", project.title);
    project.description = "由原作中已保存的创作成果迁入；保留内容、版本和人工编辑。".into();
    project.project_kind = kind.into();
    project.root_project_id = Some(package.root_project_id);
    project.source_snapshot_id = Some(snapshot_id);
    project.source_sync_status = "root_updated".into();
    project.status = "draft".into();
    project.lifecycle_gate = "active".into();
    project.target_chapters = i32::try_from(count).map_err(error)?;
    project.total_chapters = Some(project.target_chapters);
    project.current_chapter = 0;
    project.raw_blueprint = None;
    project.import_analysis_progress = None;
    project.export_config = None;
    super::receipt(
        package,
        &receipt_kind,
        package.root_project_id,
        project.id,
        project.id,
        serde_json::to_value(&project).map_err(error)?,
        &project,
    )?;
    let id = project.id;
    package.projects.push(project);
    Ok(id)
}

fn chapter_hash(source: &source_snapshot_chapter::Model) -> Value {
    json!({"source_chapter_id": source.source_chapter_id, "chapter_number": source.chapter_number,
        "content_hash": source.content_hash, "analysis_hash": source.analysis_hash})
}

fn snapshot_hash(snapshot: &project_source_snapshot::Model) -> Result<String, String> {
    hash(
        &json!({"root_project_id": snapshot.root_project_id, "chapter_scope": snapshot.chapter_scope_json,
        "chapter_hashes": snapshot.chapter_hashes_json, "profile_hashes": snapshot.profile_hashes_json,
        "blueprint_version_id": snapshot.blueprint_version_id}),
    )
}

pub(super) fn remove_film_hashes(package: &mut ProjectFamilyPackage) -> Result<(), String> {
    for snapshot in &mut package.source_snapshots {
        if snapshot_hash(snapshot)? != snapshot.source_version_hash {
            return Err(format!("旧包快照 {} 哈希不一致", snapshot.id));
        }
        let mut chapters = package
            .source_snapshot_chapters
            .iter()
            .filter(|row| row.snapshot_id == snapshot.id)
            .collect::<Vec<_>>();
        chapters.sort_by_key(|row| row.chapter_number);
        snapshot.chapter_hashes_json =
            json!(chapters.into_iter().map(chapter_hash).collect::<Vec<_>>());
        let old = snapshot.source_version_hash.clone();
        snapshot.source_version_hash = snapshot_hash(snapshot)?;
        for spec in package.generation_specs.iter_mut().filter(|row| {
            row.source_snapshot_id == Some(snapshot.id) && row.source_version_hash == old
        }) {
            spec.source_version_hash = snapshot.source_version_hash.clone();
        }
        for anchor in package.text_anchors.iter_mut().filter(|row| {
            row.source_snapshot_id == Some(snapshot.id) && row.source_version_hash == old
        }) {
            anchor.source_version_hash = snapshot.source_version_hash.clone();
        }
    }
    Ok(())
}
