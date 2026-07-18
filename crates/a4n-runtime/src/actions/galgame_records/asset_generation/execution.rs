use super::{
    AppState, GALGAME_TARGET_BACKGROUND, GALGAME_TARGET_CG, GALGAME_TARGET_PORTRAIT_EXPRESSION,
    GalgameAssetGenerationSummary, GalgameAssetPlan, GalgameProjectInput,
    RunGalgameAssetGenerationInput, access, run_galgame_portrait_spec_generation,
    run_galgame_visual_spec_generation,
};
use a4n_core::models::generation::generation_spec;
use std::collections::BTreeSet;

pub async fn plan_galgame_assets(
    state: &AppState,
    user_id: i64,
    input: GalgameProjectInput,
) -> Result<GalgameAssetPlan, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    state
        .plan_galgame_assets(input.project_id)
        .await
        .map_err(|err| err.to_string())
}

pub async fn run_galgame_asset_generation(
    state: &AppState,
    user_id: i64,
    input: RunGalgameAssetGenerationInput,
) -> Result<GalgameAssetGenerationSummary, String> {
    access::verify_project_editor_string(state, user_id, input.project_id).await?;
    validate_run_galgame_asset_generation_input(&input)?;

    let specs = load_galgame_asset_specs(state, &input).await?;
    let mut summary = GalgameAssetGenerationSummary {
        project_id: input.project_id,
        generated_count: 0,
        skipped_completed_count: 0,
        items: Vec::with_capacity(specs.len()),
    };
    for spec in specs {
        let (item, generated) = match spec.target_kind.as_str() {
            GALGAME_TARGET_BACKGROUND | GALGAME_TARGET_CG => {
                run_galgame_visual_spec_generation(state, user_id, &input, &spec).await?
            }
            GALGAME_TARGET_PORTRAIT_EXPRESSION => {
                run_galgame_portrait_spec_generation(state, user_id, &spec).await?
            }
            other => {
                return Err(format!(
                    "Galgame 资产生成不支持 target_kind: spec_id={}, target_kind={other}",
                    spec.id
                ));
            }
        };
        if generated {
            summary.generated_count += 1;
        } else {
            summary.skipped_completed_count += 1;
        }
        summary.items.push(item);
    }
    Ok(summary)
}

pub(super) fn validate_run_galgame_asset_generation_input(
    input: &RunGalgameAssetGenerationInput,
) -> Result<(), String> {
    if let Some(limit) = input.limit
        && limit == 0
    {
        return Err("limit 必须大于 0".to_string());
    }
    let mut seen = BTreeSet::new();
    for spec_id in &input.generation_spec_ids {
        if *spec_id <= 0 {
            return Err("generation_spec_ids 必须全部是正整数".to_string());
        }
        if !seen.insert(*spec_id) {
            return Err(format!("generation_spec_ids 不能包含重复 ID: {spec_id}"));
        }
    }
    Ok(())
}

pub(super) async fn load_galgame_asset_specs(
    state: &AppState,
    input: &RunGalgameAssetGenerationInput,
) -> Result<Vec<generation_spec::Model>, String> {
    state
        .orchestrator
        .galgame_asset_generation_specs(
            input.project_id,
            &[
                GALGAME_TARGET_BACKGROUND,
                GALGAME_TARGET_CG,
                GALGAME_TARGET_PORTRAIT_EXPRESSION,
            ],
            &input.generation_spec_ids,
            input.limit,
        )
        .await
        .map_err(crate::actions::app_error_message)
}

pub(super) async fn mark_galgame_generation_spec_status(
    state: &AppState,
    generation_spec_id: i64,
    status: &str,
) -> Result<(), String> {
    state
        .orchestrator
        .mark_galgame_generation_spec_status_record(generation_spec_id, status)
        .await
        .map_err(crate::actions::app_error_message)
}
