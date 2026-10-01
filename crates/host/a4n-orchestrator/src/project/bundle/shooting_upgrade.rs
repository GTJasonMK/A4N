use a4n_domain::shooting_script::{
    compat::{HashUpgrades, checkpoint::upgrade_completed_checkpoint, serialized_sha256},
    project_visual_plan_for_shooting_script,
};
use serde_json::{Value, json};

use super::{ProjectBundleHistoryRows, ProjectFamilyPackage};

impl ProjectFamilyPackage {
    /// Upgrade only the already-supported legacy family formats before current validation.
    /// Original model requests/responses remain in the bundle's audit history.
    pub(crate) fn upgrade_shooting_contract(
        &mut self,
        history: &mut ProjectBundleHistoryRows,
    ) -> Result<(), String> {
        let family = self;
        if !matches!(
            family.version.as_str(),
            "project_family_v5" | "project_family_v6"
        ) {
            return Ok(());
        }
        let mut hashes = HashUpgrades::default();
        for row in family
            .chapter_visual_plans
            .iter()
            .filter(|row| matches!(row.status.as_str(), "confirmed" | "superseded"))
        {
            let plan = serde_json::from_value(row.plan_json.clone())
                .map_err(|e| format!("导演编排 {}: {e}", row.id))?;
            let plan = project_visual_plan_for_shooting_script(&plan, &[])
                .map_err(|e| format!("导演编排 {}: {e}", row.id))?;
            hashes.register_plan(row.id, plan);
        }
        for script in &mut family.chapter_shooting_scripts {
            if script.script_json["schema_version"].as_str()
                != Some(script.contract_version.as_str())
            {
                return Err(format!("拍摄剧本 {} 的记录版本与 JSON 不一致", script.id));
            }
            script.script_sha256 = hashes
                .upgrade_record(&mut script.script_json, &script.script_sha256)
                .map_err(|e| format!("拍摄剧本 {}: {e}", script.id))?;
            script.contract_version =
                a4n_domain::shooting_script::CHAPTER_SHOOTING_SCRIPT_V2.into();
        }
        let readable_runs = history
            .workflow_runs
            .iter()
            .filter(|run| run.can_resume || run.status == "completed")
            .map(|run| run.id)
            .collect::<std::collections::HashSet<_>>();
        for run in &mut history.workflow_runs {
            if readable_runs.contains(&run.id)
                && let Some(progress) = run.progress_json.as_mut()
            {
                hashes.upgrade_embedded(progress)?;
            }
        }
        for snapshot in &mut family.source_snapshots {
            let mut payload = json!({
                "root_project_id": snapshot.root_project_id,
                "chapter_scope": snapshot.chapter_scope_json,
                "chapter_hashes": snapshot.chapter_hashes_json,
                "profile_hashes": snapshot.profile_hashes_json,
                "blueprint_version_id": snapshot.blueprint_version_id,
            });
            if serialized_sha256(&payload).map_err(|e| e.to_string())?
                != snapshot.source_version_hash
            {
                return Err(format!("来源快照 {} 的原始内容哈希不一致", snapshot.id));
            }
            hashes.rewrite_references(&mut snapshot.chapter_hashes_json);
            payload["chapter_hashes"] = snapshot.chapter_hashes_json.clone();
            let next = serialized_sha256(&payload).map_err(|e| e.to_string())?;
            hashes.insert(snapshot.source_version_hash.clone(), next.clone())?;
            snapshot.source_version_hash = next;
        }
        for row in &mut family.chapter_video_prompts {
            upgrade_optional(&mut hashes, &mut row.metadata_json)?;
        }
        for row in &mut family.generated_images {
            upgrade_optional(&mut hashes, &mut row.metadata_json)?;
        }
        for row in &mut family.generated_videos {
            upgrade_optional(&mut hashes, &mut row.metadata_json)?;
        }
        for row in &mut family.chapter_manga_prompts {
            upgrade_optional(&mut hashes, &mut row.layout)?;
        }
        for row in &mut family.generation_specs {
            upgrade_value(&mut hashes, &mut row.params_json)?;
            if let Some(next) = hashes.mapped(&row.source_version_hash) {
                row.source_version_hash = next.into();
            }
        }
        for row in &mut family.source_sync_runs {
            upgrade_value(&mut hashes, &mut row.summary_json)?;
        }
        for row in &mut family.source_sync_conflicts {
            upgrade_value(&mut hashes, &mut row.source_summary_json)?;
            upgrade_value(&mut hashes, &mut row.local_summary_json)?;
            upgrade_value(&mut hashes, &mut row.affected_refs_json)?;
            upgrade_optional(&mut hashes, &mut row.decision_json)?;
        }
        for row in &mut family.source_sync_decisions {
            upgrade_value(&mut hashes, &mut row.decision_payload_json)?;
        }
        for row in &mut history.generation_runs {
            upgrade_optional(&mut hashes, &mut row.parsed_output_json)?;
            upgrade_optional(&mut hashes, &mut row.artifact_refs_json)?;
            upgrade_optional(&mut hashes, &mut row.source_refs_json)?;
        }
        for row in &mut history.workflow_checkpoints {
            if row.status != "completed" || !readable_runs.contains(&row.workflow_run_id) {
                continue;
            }
            if let Some(checkpoint) = row.checkpoint_json.as_mut() {
                if checkpoint.get("parsed_output").is_some()
                    && checkpoint.get("response_content").is_some()
                {
                    upgrade_completed_checkpoint(
                        &row.step_key,
                        row.input_fingerprint_json.as_ref(),
                        row.input_fingerprint_sha256.as_deref(),
                        checkpoint,
                        &mut row.artifacts_json,
                        &mut hashes,
                    )
                    .map_err(|error| format!("工作流断点 {}: {error}", row.id))?;
                } else {
                    hashes.upgrade_embedded(checkpoint)?;
                    hashes.rewrite_references(checkpoint);
                }
            }
        }
        family.version = "project_family_v7".into();
        Ok(())
    }
}

fn upgrade_optional(hashes: &mut HashUpgrades, value: &mut Option<Value>) -> Result<(), String> {
    if let Some(value) = value {
        upgrade_value(hashes, value)?;
    }
    Ok(())
}

fn upgrade_value(hashes: &mut HashUpgrades, value: &mut Value) -> Result<(), String> {
    hashes.upgrade_embedded(value)?;
    hashes.rewrite_references(value);
    Ok(())
}
