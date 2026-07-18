pub(super) use super::portrait_plan::GalgamePortraitSpecPlan;
use super::*;

pub(super) async fn run_galgame_portrait_spec_generation(
    state: &AppState,
    user_id: i64,
    spec: &generation_spec::Model,
) -> Result<(GalgameAssetGenerationItem, bool), String> {
    let plan = GalgamePortraitSpecPlan::from_spec(spec)?;
    let profile = validate_galgame_portrait_spec_target(state, spec, &plan).await?;
    access::verify_project_editor_string(state, user_id, spec.root_project_id).await?;
    if let Some(item) = completed_galgame_portrait(state, spec, &plan).await? {
        return Ok((item, false));
    }

    mark_galgame_generation_spec_status(state, spec.id, WORKFLOW_STATUS_RUNNING).await?;
    let reference_image_ids = match primary_portrait_reference_image_ids(state, &profile).await {
        Ok(ids) => ids,
        Err(err) => {
            mark_galgame_generation_spec_status(state, spec.id, WORKFLOW_STATUS_FAILED).await?;
            return Err(format!(
                "Galgame 立绘差分缺少可用主立绘参考: spec_id={}，{err}",
                spec.id
            ));
        }
    };
    let portrait_result = portrait_generation::generate_portrait(
        state,
        user_id,
        GeneratePortraitInput {
            project_id: spec.root_project_id,
            character_name: profile.name.clone(),
            style: Some(GALGAME_PORTRAIT_VARIANT_STYLE.to_string()),
            expressions: Some(vec![plan.expression.clone()]),
            pose: plan.pose.clone(),
            reference_image_ids,
        },
        None,
        CancellationToken::new(),
    )
    .await;
    let portrait = match portrait_result {
        Ok(portrait) => portrait,
        Err(err) => {
            mark_galgame_generation_spec_status(state, spec.id, WORKFLOW_STATUS_FAILED).await?;
            return Err(format!(
                "Galgame 立绘差分生成失败: spec_id={}，{}",
                spec.id, err
            ));
        }
    };
    mark_galgame_generation_spec_status(state, spec.id, WORKFLOW_STATUS_COMPLETED).await?;
    Ok((
        galgame_portrait_generation_item(spec, &plan, &portrait),
        true,
    ))
}

pub(super) async fn validate_galgame_portrait_spec_target(
    state: &AppState,
    spec: &generation_spec::Model,
    plan: &GalgamePortraitSpecPlan,
) -> Result<protagonist_profile::Model, String> {
    state
        .orchestrator
        .galgame_portrait_spec_target_profile(
            spec,
            &GalgamePortraitSpecTargetInput {
                profile_id: plan.profile_id,
                expression: plan.expression.clone(),
                pose: plan.pose.clone(),
                position_samples: plan.position_samples.clone(),
                source_line_ids: plan.source_line_ids.clone(),
            },
        )
        .await
        .map_err(crate::actions::app_error_message)
}

#[cfg(test)]
pub(super) fn validate_galgame_portrait_spec_family_context(
    spec: &generation_spec::Model,
    root_project_id: i64,
    source_snapshot_id: i64,
) -> Result<(), String> {
    a4n_orchestrator::galgame::validate_galgame_portrait_spec_family_context(
        spec,
        root_project_id,
        source_snapshot_id,
    )
    .map_err(|error| error.to_string())
}

#[cfg(test)]
pub(super) fn validate_galgame_portrait_position_samples(
    spec_id: i64,
    position_samples: &[String],
    source_positions: &BTreeSet<String>,
) -> Result<(), String> {
    a4n_orchestrator::galgame::validate_galgame_portrait_position_samples(
        spec_id,
        position_samples,
        source_positions,
    )
    .map_err(|error| error.to_string())
}

pub(super) async fn completed_galgame_portrait(
    state: &AppState,
    spec: &generation_spec::Model,
    plan: &GalgamePortraitSpecPlan,
) -> Result<Option<GalgameAssetGenerationItem>, String> {
    if spec.status != WORKFLOW_STATUS_COMPLETED {
        return Ok(None);
    }
    let portrait = state
        .orchestrator
        .completed_galgame_portrait_record(
            spec,
            plan.profile_id,
            &plan.expression,
            plan.pose.as_deref(),
        )
        .await
        .map_err(crate::actions::app_error_message)?;
    canonicalize_data_file_path(
        &state.config.data_dir,
        &portrait.image_path,
        "Galgame 立绘差分",
    )
    .map_err(|err| {
        format!(
            "Galgame 立绘差分生成意图已完成但立绘文件不可用: spec_id={}, portrait_id={}，{}",
            spec.id, portrait.id, err
        )
    })?;
    let summary = portrait_generation::to_summary(&portrait);
    Ok(Some(galgame_portrait_generation_item(spec, plan, &summary)))
}

pub(super) async fn primary_portrait_reference_image_ids(
    state: &AppState,
    profile: &protagonist_profile::Model,
) -> Result<Vec<i64>, String> {
    state
        .orchestrator
        .primary_galgame_portrait_reference_image_ids(profile)
        .await
        .map_err(crate::actions::app_error_message)
}

pub(super) fn galgame_portrait_generation_item(
    spec: &generation_spec::Model,
    plan: &GalgamePortraitSpecPlan,
    portrait: &PortraitSummary,
) -> GalgameAssetGenerationItem {
    GalgameAssetGenerationItem {
        generation_spec_id: spec.id,
        target_kind: spec.target_kind.clone(),
        target_entity_kind: spec.target_entity_kind.clone(),
        target_entity_id: plan.profile_id,
        status: WORKFLOW_STATUS_COMPLETED.to_string(),
        prompt_artifact_id: None,
        generated_image_id: None,
        portrait_id: Some(portrait.id),
        media_asset_id: None,
        binding_id: None,
    }
}
