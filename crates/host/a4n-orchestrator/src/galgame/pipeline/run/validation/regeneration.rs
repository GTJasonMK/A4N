use a4n_error::AppError;

use crate::galgame::{GalgameCharacterStaging, GalgameScriptAdaptation, GalgameVisualPlan};

pub(in crate::galgame::pipeline::run) fn ensure_single_regenerated_scene_lines(
    script_adaptation: &GalgameScriptAdaptation,
) -> Result<(), AppError> {
    if !script_adaptation.branch_scene_lines.is_empty()
        || script_adaptation.scene_lines.len() != 1
        || script_adaptation
            .scene_lines
            .first()
            .is_none_or(|scene| scene.scene_index != 1 || scene.lines.is_empty())
    {
        return Err(AppError::InvalidArgument(
            "Galgame source sync 局部重跑必须返回且只返回 scene_index=1 的非空剧本行".to_string(),
        ));
    }
    Ok(())
}

pub(in crate::galgame::pipeline::run) fn ensure_single_regenerated_staged_lines(
    character_staging: &GalgameCharacterStaging,
) -> Result<(), AppError> {
    if !character_staging.branch_scene_lines.is_empty()
        || character_staging.scene_lines.len() != 1
        || character_staging
            .scene_lines
            .first()
            .is_none_or(|scene| scene.scene_index != 1 || scene.lines.is_empty())
    {
        return Err(AppError::InvalidArgument(
            "Galgame source sync 局部重跑必须返回且只返回 scene_index=1 的非空角色舞台行"
                .to_string(),
        ));
    }
    Ok(())
}

pub(in crate::galgame::pipeline::run) fn ensure_single_regenerated_visual_plan(
    visual_plan: &GalgameVisualPlan,
) -> Result<(), AppError> {
    if visual_plan.backgrounds.len() != 1 {
        return Err(AppError::InvalidArgument(
            "Galgame source sync 局部重跑必须返回且只返回一个 scene_index=1 背景规划".to_string(),
        ));
    }
    if visual_plan.backgrounds[0].scene_index != 1 {
        return Err(AppError::InvalidArgument(
            "Galgame source sync 局部重跑背景规划必须只引用 scene_index=1".to_string(),
        ));
    }
    if let Some(event_cg) = visual_plan
        .event_cgs
        .iter()
        .find(|item| item.scene_index != 1)
    {
        return Err(AppError::InvalidArgument(format!(
            "Galgame source sync 局部重跑事件 CG 不能引用其他场: scene_index={}",
            event_cg.scene_index
        )));
    }
    Ok(())
}
