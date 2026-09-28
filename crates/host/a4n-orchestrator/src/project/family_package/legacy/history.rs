use a4n_db::project::bundle::ProjectBundleHistoryRows;
use a4n_domain::{
    generation::canonical_json_sha256,
    media_authoring::legacy::workflow::{
        LEGACY_MEDIA_MODE, relocate_film_fingerprint, uses_legacy_media_pipeline,
    },
};
use serde_json::{Value, json};
use std::collections::BTreeMap;

use super::{ProjectFamilyPackage, error, receipt};

pub(super) fn upgrade(
    package: &mut ProjectFamilyPackage,
    history: &mut ProjectBundleHistoryRows,
    next: &mut i64,
) -> Result<(), String> {
    let plans = package
        .authoring_receipts
        .iter()
        .filter(|row| row.kind == "visual_plan")
        .map(|row| ((row.old_id, row.project_id), row.new_id))
        .collect::<BTreeMap<_, _>>();
    let scripts = package
        .authoring_receipts
        .iter()
        .filter(|row| row.kind == "shooting_script")
        .map(|row| ((row.old_id, row.project_id), row.new_id))
        .collect::<BTreeMap<_, _>>();
    let mut owners = BTreeMap::new();
    for run in &mut history.workflow_runs {
        let Some(old_project) = run.project_id else {
            continue;
        };
        if !package
            .projects
            .iter()
            .any(|project| project.id == old_project)
        {
            continue;
        }
        let film = run.workflow_kind == "chapter_shooting_script";
        let legacy_pipeline = uses_legacy_media_pipeline(
            &run.workflow_kind,
            run.mode.as_deref(),
            run.input_fingerprint_json.as_ref(),
        );
        if !film && !legacy_pipeline {
            continue;
        }
        let original = serde_json::to_value(&*run).map_err(error)?;
        let mut owner = old_project;
        if film {
            let old_plan = run
                .input_fingerprint_json
                .as_ref()
                .and_then(|value| value.get("visual_plan_id"))
                .and_then(Value::as_i64);
            if let Some(old) = old_plan {
                let destination = plans
                    .get(&(old, old_project))
                    .map(|id| (old_project, *id))
                    .or_else(|| {
                        plans
                            .iter()
                            .find(|((id, _), _)| *id == old)
                            .map(|((_, project), id)| (*project, *id))
                    });
                if let Some((project, id)) = destination {
                    owner = project;
                    run.source_snapshot_id = package
                        .chapter_visual_plans
                        .iter()
                        .find(|plan| plan.id == id)
                        .ok_or("旧包工作流导演方案丢失")?
                        .source_snapshot_id;
                    let mapping = plans
                        .iter()
                        .filter(|((_, project), _)| *project == owner)
                        .map(|((old, _), new)| (*old, *new))
                        .collect();
                    if let Some(input) = &mut run.input_fingerprint_json {
                        relocate_film_fingerprint(input, old_project, owner, &mapping)?;
                    }
                    for checkpoint in history
                        .workflow_checkpoints
                        .iter_mut()
                        .filter(|checkpoint| checkpoint.workflow_run_id == run.id)
                    {
                        let Some(input) = &mut checkpoint.input_fingerprint_json else {
                            continue;
                        };
                        if let Some(expected) = &checkpoint.input_fingerprint_sha256
                            && canonical_json_sha256(input).map_err(error)? != *expected
                        {
                            return Err("旧包工作流断点输入哈希不一致".into());
                        }
                        relocate_film_fingerprint(input, old_project, owner, &mapping)?;
                        if checkpoint.input_fingerprint_sha256.is_some() {
                            checkpoint.input_fingerprint_sha256 =
                                Some(canonical_json_sha256(input).map_err(error)?);
                        }
                    }
                }
            }
        } else if run.workflow_kind == "manga_pipeline" && old_project == package.root_project_id {
            if let Some(chapter_id) = run.chapter_id {
                let hash = run
                    .input_fingerprint_json
                    .as_ref()
                    .and_then(|value| value.pointer("/source/content_sha256"))
                    .and_then(Value::as_str);
                if let Some(page) = package.chapter_manga_prompts.iter().find(|page| {
                    page.chapter_id == chapter_id
                        && page
                            .layout
                            .as_ref()
                            .and_then(|value| value.pointer("/source/content_sha256"))
                            .and_then(Value::as_str)
                            == hash
                }) {
                    owner = page.project_id;
                    run.source_snapshot_id = page
                        .layout
                        .as_ref()
                        .and_then(|value| value.pointer("/source/source_snapshot_id"))
                        .and_then(Value::as_i64);
                } else if let Some(hash) = hash {
                    let source =
                        super::source::source_for(package, chapter_id, None, None, hash, next)?;
                    owner = super::source::historical_project(
                        package,
                        source.snapshot_id,
                        "comic",
                        next,
                    )?;
                    run.source_snapshot_id = Some(source.snapshot_id);
                }
            }
        }
        run.project_id = Some(owner);
        run.root_project_id = Some(package.root_project_id);
        run.active_task_id = None;
        if owner != old_project {
            let prefix = format!("{}:{old_project}:", run.workflow_kind);
            if let Some(rest) = run.business_key.strip_prefix(&prefix) {
                run.business_key = format!("{}:{owner}:{rest}", run.workflow_kind);
            }
        }
        if legacy_pipeline || owner == package.root_project_id {
            run.mode = Some(LEGACY_MEDIA_MODE.into());
            run.can_resume = false;
            run.resume_block_reason =
                Some("旧媒体依赖已转换；原始调用保留，请从派生工作区已保存的计划阶段继续。".into());
        }
        if run.status == "running" {
            run.status = "interrupted".into();
            run.error_message = Some("旧包运行已中断，等待在派生工作区继续。".into());
        }
        receipt(
            package,
            "media_workflow",
            run.id,
            run.id,
            owner,
            original,
            run,
        )?;
        owners.insert(run.id, owner);
    }
    let mut generation_owners = BTreeMap::new();
    for run in &mut history.generation_runs {
        let owner = run
            .workflow_run_id
            .and_then(|id| owners.get(&id))
            .copied()
            .or_else(|| {
                history
                    .generation_run_targets
                    .iter()
                    .filter(|target| target.generation_run_id == run.id)
                    .find_map(|target| match target.target_table.as_str() {
                        "chapter_visual_plan" => plans
                            .iter()
                            .find(|((id, _), _)| *id == target.target_id)
                            .map(|((_, project), _)| *project),
                        "chapter_shooting_script" => scripts
                            .iter()
                            .find(|((id, _), _)| *id == target.target_id)
                            .map(|((_, project), _)| *project),
                        "chapter_manga_prompt" => package
                            .chapter_manga_prompts
                            .iter()
                            .find(|page| page.id == target.target_id)
                            .map(|page| page.project_id),
                        _ => None,
                    })
            });
        if let Some(owner) = owner {
            let before = serde_json::to_value(&*run).map_err(error)?;
            run.project_id = owner;
            receipt(
                package,
                "generation_run_owner",
                run.id,
                run.id,
                owner,
                before,
                &json!({"project_id":owner}),
            )?;
            generation_owners.insert(run.id, owner);
        }
    }
    for target in &mut history.generation_run_targets {
        if let Some(owner) = generation_owners.get(&target.generation_run_id) {
            let ids = match target.target_table.as_str() {
                "chapter_visual_plan" => Some(&plans),
                "chapter_shooting_script" => Some(&scripts),
                _ => None,
            };
            if let Some(id) = ids.and_then(|ids| ids.get(&(target.target_id, *owner))) {
                target.target_id = *id;
            }
        }
    }
    for trace in &mut history.llm_call_traces {
        if let Some(owner) = trace
            .generation_run_id
            .and_then(|id| generation_owners.get(&id))
            .or_else(|| trace.workflow_run_id.and_then(|id| owners.get(&id)))
        {
            trace.project_id = Some(*owner);
        }
    }
    for usage in &mut history.model_usage_records {
        if let Some(owner) = usage
            .generation_run_id
            .and_then(|id| generation_owners.get(&id))
            .or_else(|| usage.workflow_run_id.and_then(|id| owners.get(&id)))
        {
            usage.project_id = Some(*owner);
        }
    }
    Ok(())
}
