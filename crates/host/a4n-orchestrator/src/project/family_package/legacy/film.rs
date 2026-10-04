use std::collections::{BTreeMap, BTreeSet};

use a4n_db::project::bundle::ProjectBundleHistoryRows;
use a4n_domain::{
    media_authoring::legacy::film::{
        ScriptReferenceChange, referenced_script_ids, rewrite_film_references,
    },
    shooting_script::ChapterShootingScriptV2,
};
use serde_json::Value;

use super::{
    ProjectFamilyPackage, error, hash, next_id, receipt,
    source::{historical_project, source_for},
};

pub(super) fn upgrade(
    package: &mut ProjectFamilyPackage,
    history: &mut ProjectBundleHistoryRows,
    next: &mut i64,
) -> Result<(), String> {
    let videos = package
        .projects
        .iter()
        .filter(|project| {
            project.project_kind == "video"
                && project.root_project_id == Some(package.root_project_id)
        })
        .map(|project| project.id)
        .collect::<BTreeSet<_>>();
    let mut snapshots = BTreeMap::<i64, BTreeSet<i64>>::new();
    for project in package
        .projects
        .iter()
        .filter(|project| videos.contains(&project.id))
    {
        if let Some(id) = project.source_snapshot_id {
            snapshots.entry(id).or_default().insert(project.id);
        }
    }
    for run in package
        .source_sync_runs
        .iter()
        .filter(|run| videos.contains(&run.derived_project_id))
    {
        snapshots
            .entry(run.from_snapshot_id)
            .or_default()
            .insert(run.derived_project_id);
        snapshots
            .entry(run.to_snapshot_id)
            .or_default()
            .insert(run.derived_project_id);
    }
    let mut consumers = BTreeMap::<i64, BTreeSet<i64>>::new();
    let mut collect = |project_id: i64, value: Option<&Value>| {
        if videos.contains(&project_id)
            && let Some(value) = value
        {
            for id in referenced_script_ids(value) {
                consumers.entry(id).or_default().insert(project_id);
            }
        }
    };
    for row in &package.chapter_video_prompts {
        collect(row.project_id, row.metadata_json.as_ref());
    }
    for row in &package.generated_images {
        collect(row.project_id, row.metadata_json.as_ref());
    }
    for row in &package.generated_videos {
        collect(row.project_id, row.metadata_json.as_ref());
    }
    for row in &package.generation_specs {
        collect(row.project_id, Some(&row.params_json));
    }
    for run in &history.workflow_runs {
        if let Some(project) = run.project_id {
            collect(project, run.progress_json.as_ref());
            collect(project, run.input_fingerprint_json.as_ref());
        }
    }
    let scripts = std::mem::take(&mut package.chapter_shooting_scripts);
    let mut plan_consumers = BTreeMap::<i64, BTreeSet<i64>>::new();
    for script in &scripts {
        let owners = consumers.entry(script.id).or_default();
        if let Some(snapshot) = script.source_snapshot_id.and_then(|id| snapshots.get(&id)) {
            owners.extend(snapshot);
        }
        if owners.is_empty() {
            let source = source_for(
                package,
                script.source_chapter_id,
                script.source_snapshot_id,
                script.source_snapshot_chapter_id,
                &script.source_content_sha256,
                next,
            )?;
            owners.insert(historical_project(
                package,
                source.snapshot_id,
                "video",
                next,
            )?);
        }
        if let Some(plan) = script.visual_plan_id {
            plan_consumers
                .entry(plan)
                .or_default()
                .extend(owners.iter().copied());
        }
    }
    let originals = std::mem::take(&mut package.chapter_visual_plans);
    let mut plan_maps = BTreeMap::<i64, BTreeMap<i64, i64>>::new();
    for original in originals {
        let typed: a4n_domain::chapter_visual_plan::ChapterVisualPlanV3 =
            serde_json::from_value(original.plan_json.clone()).map_err(error)?;
        typed.validate_draft().map_err(error)?;
        if hash(&typed)? != original.plan_sha256 {
            return Err(format!("旧包导演 {} 内容哈希不一致", original.id));
        }
        let source = source_for(
            package,
            original.source_chapter_id,
            original.source_snapshot_id,
            original.source_snapshot_chapter_id,
            &original.source_content_sha256,
            next,
        )?;
        let mut owners = original
            .source_snapshot_id
            .and_then(|id| snapshots.get(&id))
            .cloned()
            .unwrap_or_default();
        owners.extend(plan_consumers.remove(&original.id).unwrap_or_default());
        if owners.is_empty() {
            owners.insert(historical_project(
                package,
                source.snapshot_id,
                "video",
                next,
            )?);
        }
        for (index, owner) in owners.into_iter().enumerate() {
            let mut row = original.clone();
            if index > 0 {
                row.id = next_id(next)?;
            }
            row.project_id = owner;
            row.source_snapshot_id = Some(source.snapshot_id);
            row.source_snapshot_chapter_id = Some(source.id);
            row.source_revision_key = revision_key(&source);
            plan_maps
                .entry(owner)
                .or_default()
                .insert(original.id, row.id);
            receipt(
                package,
                "visual_plan",
                original.id,
                row.id,
                owner,
                serde_json::to_value(&original).map_err(error)?,
                &row,
            )?;
            package.chapter_visual_plans.push(row);
        }
    }
    let mut script_maps = BTreeMap::<i64, BTreeMap<i64, ScriptReferenceChange>>::new();
    for original in scripts {
        let typed: ChapterShootingScriptV2 =
            serde_json::from_value(original.script_json.clone()).map_err(error)?;
        typed.validate().map_err(error)?;
        if a4n_domain::shooting_script::compat::serialized_sha256(&typed).map_err(error)?
            != original.script_sha256
        {
            return Err(format!("旧包剧本 {} 内容哈希不一致", original.id));
        }
        let source = source_for(
            package,
            original.source_chapter_id,
            original.source_snapshot_id,
            original.source_snapshot_chapter_id,
            &original.source_content_sha256,
            next,
        )?;
        for (index, owner) in consumers
            .remove(&original.id)
            .ok_or("旧包剧本归属缺失")?
            .into_iter()
            .enumerate()
        {
            let mut row = original.clone();
            if index > 0 {
                row.id = next_id(next)?;
            }
            row.project_id = owner;
            row.source_snapshot_id = Some(source.snapshot_id);
            row.source_snapshot_chapter_id = Some(source.id);
            row.source_revision_key = revision_key(&source);
            let mut script = typed.clone();
            match (original.visual_plan_id, script.visual_plan_source.as_mut()) {
                (Some(old), Some(identity)) if identity.plan_id == old => {
                    let mapped = plan_maps
                        .get(&owner)
                        .and_then(|map| map.get(&old))
                        .copied()
                        .ok_or("剧本缺少所属视频的导演方案")?;
                    row.visual_plan_id = Some(mapped);
                    identity.plan_id = mapped;
                }
                (None, None) => {}
                _ => return Err("旧包剧本导演身份不一致".into()),
            }
            row.script_sha256 =
                a4n_domain::shooting_script::compat::serialized_sha256(&script).map_err(error)?;
            row.script_json = serde_json::to_value(script).map_err(error)?;
            script_maps.entry(owner).or_default().insert(
                original.id,
                ScriptReferenceChange {
                    id: row.id,
                    revision: row.revision,
                    old_sha256: original.script_sha256.clone(),
                    sha256: row.script_sha256.clone(),
                    old_source_revision_key: original.source_revision_key.clone(),
                    source_revision_key: row.source_revision_key.clone(),
                },
            );
            receipt(
                package,
                "shooting_script",
                original.id,
                row.id,
                owner,
                serde_json::to_value(&original).map_err(error)?,
                &row,
            )?;
            package.chapter_shooting_scripts.push(row);
        }
    }
    for (owner, scripts) in &script_maps {
        let empty = BTreeMap::new();
        let plans = plan_maps.get(owner).unwrap_or(&empty);
        for row in package
            .chapter_video_prompts
            .iter_mut()
            .filter(|row| row.project_id == *owner)
        {
            if let Some(value) = row.metadata_json.as_mut() {
                rewrite_film_references(value, scripts, plans)?;
            }
        }
        for row in package
            .generated_images
            .iter_mut()
            .filter(|row| row.project_id == *owner)
        {
            if let Some(value) = row.metadata_json.as_mut() {
                rewrite_film_references(value, scripts, plans)?;
            }
        }
        for row in package
            .generated_videos
            .iter_mut()
            .filter(|row| row.project_id == *owner)
        {
            if let Some(value) = row.metadata_json.as_mut() {
                rewrite_film_references(value, scripts, plans)?;
            }
        }
        for row in package
            .generation_specs
            .iter_mut()
            .filter(|row| row.project_id == *owner)
        {
            rewrite_film_references(&mut row.params_json, scripts, plans)?;
        }
    }
    Ok(())
}

fn revision_key(source: &a4n_domain::project::ProjectFamilySourceSnapshotChapterRecord) -> String {
    format!(
        "snapshot:{}:chapter:{}:{}",
        source.snapshot_id, source.id, source.content_hash
    )
}
