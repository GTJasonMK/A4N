use super::*;

pub(in crate::project::dependency_graph) async fn enrich_lineage_records_with_generation_inputs<C>(
    db: &C,
    records: &mut [AssetLineageRecord],
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    let runs_by_target = load_generation_runs_for_records(db, records).await?;
    for record in records {
        let runs = generation_target_key(record)
            .and_then(|target_key| runs_by_target.get(&target_key))
            .map(Vec::as_slice)
            .unwrap_or_default();
        apply_generation_runs(record, runs);
    }
    Ok(())
}

fn has_structural_lineage(record: &AssetLineageRecord) -> bool {
    !record.source_refs.is_empty()
        || !record.prompt_sources.is_empty()
        || !record.media_sources.is_empty()
        || record.request_snapshot.is_some()
}

type GenerationTargetKey = (&'static str, i64);

async fn load_generation_runs_for_records<C>(
    db: &C,
    records: &[AssetLineageRecord],
) -> Result<BTreeMap<GenerationTargetKey, Vec<generation_run::Model>>, AppError>
where
    C: ConnectionTrait,
{
    let mut target_ids_by_table: BTreeMap<&'static str, BTreeSet<i64>> = BTreeMap::new();
    for record in records {
        if let Some((target_table, target_id)) = generation_target_key(record) {
            target_ids_by_table
                .entry(target_table)
                .or_default()
                .insert(target_id);
        }
    }

    if target_ids_by_table.is_empty() {
        return Ok(BTreeMap::new());
    }

    let mut run_ids_by_target: BTreeMap<GenerationTargetKey, BTreeSet<i64>> = BTreeMap::new();
    for (target_table, target_ids) in target_ids_by_table {
        let targets = generation_run_target::Entity::find()
            .filter(generation_run_target::Column::TargetTable.eq(target_table))
            .filter(generation_run_target::Column::TargetId.is_in(target_ids.iter().copied()))
            .order_by_asc(generation_run_target::Column::Id)
            .all(db)
            .await
            .map_err(AppError::Database)?;
        for target in targets {
            run_ids_by_target
                .entry((target_table, target.target_id))
                .or_default()
                .insert(target.generation_run_id);
        }
    }

    if run_ids_by_target.is_empty() {
        return Ok(BTreeMap::new());
    }

    let mut target_keys_by_run_id: BTreeMap<i64, Vec<GenerationTargetKey>> = BTreeMap::new();
    for (target_key, run_ids) in &run_ids_by_target {
        for run_id in run_ids {
            target_keys_by_run_id
                .entry(*run_id)
                .or_default()
                .push(*target_key);
        }
    }
    let run_ids = target_keys_by_run_id.keys().copied().collect::<Vec<_>>();
    let runs = generation_run::Entity::find()
        .filter(generation_run::Column::Id.is_in(run_ids))
        .order_by_asc(generation_run::Column::CreatedAt)
        .all(db)
        .await
        .map_err(AppError::Database)?;

    let mut runs_by_target: BTreeMap<GenerationTargetKey, Vec<generation_run::Model>> =
        BTreeMap::new();
    for run in runs {
        if let Some(target_keys) = target_keys_by_run_id.get(&run.id) {
            for target_key in target_keys {
                runs_by_target
                    .entry(*target_key)
                    .or_default()
                    .push(run.clone());
            }
        }
    }

    Ok(runs_by_target)
}

fn generation_target_key(record: &AssetLineageRecord) -> Option<GenerationTargetKey> {
    let target_id = record.asset_id?;
    let target_table = generation_target_table(record.asset_kind.as_str())?;
    Some((target_table, target_id))
}

fn apply_generation_runs(record: &mut AssetLineageRecord, runs: &[generation_run::Model]) {
    let inputs = generation_inputs_from_runs(runs);
    record.generation_runs = runs.iter().map(generation_run_snapshot).collect();

    record.input_status = if !record.generation_runs.is_empty() && !inputs.is_empty() {
        "complete".to_string()
    } else if !record.generation_runs.is_empty() {
        if should_expect_generation_inputs(record) {
            record
                .warnings
                .push("该模型生成资产有 generation_run，但缺少生成输入快照。".to_string());
        }
        "run_without_inputs".to_string()
    } else if should_expect_generation_inputs(record) {
        record
            .warnings
            .push("该模型生成资产缺少 generation_run 生成输入记录。".to_string());
        "missing".to_string()
    } else if has_structural_lineage(record) {
        "structural".to_string()
    } else {
        "not_applicable".to_string()
    };
    record.generation_inputs = inputs;
}

fn generation_target_table(asset_kind: &str) -> Option<&'static str> {
    match asset_kind {
        "novel_blueprint" => Some("novel_blueprint"),
        "part_outline" => Some("part_outline"),
        "chapter_outline" => Some("chapter_outline"),
        "chapter_content" => Some("chapter_content"),
        "chapter_analysis" => Some("chapter_analysis"),
        "character_profile_record" => Some("protagonist_profile"),
        "object_profile_record" => Some("object_profile"),
        "scene_profile_record" => Some("scene_profile"),
        "character_portrait" => Some("character_portrait"),
        "chapter_manga_prompt" => Some("chapter_manga_prompt"),
        "manga_page_image" | "profile_material_image" | "video_frame_image" => {
            Some("generated_image")
        }
        "video_frame_prompt" | "chapter_video_prompt" => Some("chapter_video_prompt"),
        "generated_video" => Some("generated_video"),
        "prompt_artifact" => Some("prompt_artifact"),
        "media_asset" => Some("media_asset"),
        _ => None,
    }
}

fn generation_run_snapshot(run: &generation_run::Model) -> GenerationRunSnapshot {
    GenerationRunSnapshot {
        id: Some(run.id),
        workflow: run.workflow.clone(),
        step_key: run.step_key.clone(),
        title: run.title.clone(),
        model_kind: run.model_kind.clone(),
        provider: run.provider.clone(),
        model: run.model.clone(),
        status: run.status.clone(),
        created_at: Some(run.created_at.to_string()),
        completed_at: run.completed_at.map(|value| value.to_string()),
    }
}

fn generation_inputs_from_runs(runs: &[generation_run::Model]) -> Vec<GenerationInputSource> {
    let mut inputs = Vec::new();
    for run in runs {
        push_json_input(
            &mut inputs,
            Some(run.id),
            "prompt",
            "rendered_messages",
            "渲染消息",
            run.rendered_messages_json.clone(),
        );
        push_json_input(
            &mut inputs,
            Some(run.id),
            "prompt",
            "rendered_prompts",
            "渲染提示词",
            run.rendered_prompts_json.clone(),
        );
        push_json_input(
            &mut inputs,
            Some(run.id),
            "variables",
            "input_variables",
            "输入变量",
            run.input_variables_json.clone(),
        );
        push_json_input(
            &mut inputs,
            Some(run.id),
            "source_text",
            "source_refs",
            "上游文本与资产",
            run.source_refs_json.clone(),
        );
        push_json_input(
            &mut inputs,
            Some(run.id),
            "media",
            "media_refs",
            "参考媒体",
            run.media_refs_json.clone(),
        );
        push_json_input(
            &mut inputs,
            Some(run.id),
            "request",
            "request",
            "模型请求",
            run.request_json.clone(),
        );
        push_json_input(
            &mut inputs,
            Some(run.id),
            "response",
            "response_summary",
            "响应摘要",
            run.response_summary_json.clone(),
        );
    }
    inputs
}

fn push_json_input(
    inputs: &mut Vec<GenerationInputSource>,
    run_id: Option<i64>,
    group: &str,
    kind: &str,
    label: &str,
    value: Option<Value>,
) {
    let Some(value) = value else {
        return;
    };
    if value.is_null() {
        return;
    }
    inputs.push(GenerationInputSource {
        id: format!("{}:{}:{}", run_id.unwrap_or_default(), group, kind),
        run_id,
        group: group.to_string(),
        kind: kind.to_string(),
        label: label.to_string(),
        content_type: "json".to_string(),
        text: None,
        value: Some(value),
        source_record_id: None,
    });
}

fn should_expect_generation_inputs(record: &AssetLineageRecord) -> bool {
    matches!(
        record.origin.as_str(),
        "llm_generated" | "image_generated" | "video_generated"
    ) && matches!(
        record.asset_kind.as_str(),
        "novel_blueprint"
            | "part_outline"
            | "chapter_outline"
            | "chapter_content"
            | "chapter_analysis"
            | "character_profile_record"
            | "object_profile_record"
            | "scene_profile_record"
            | "character_portrait"
            | "profile_material_image"
            | "manga_page_image"
            | "video_frame_image"
            | "chapter_manga_prompt"
            | "video_frame_prompt"
            | "chapter_video_prompt"
            | "generated_video"
            | "prompt_artifact"
    )
}
