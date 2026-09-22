use a4n_error::AppError;
use serde_json::Value;

use super::contracts::*;
use super::lines::line_staged_from_raw;
use super::rules::*;

pub fn character_staging_from_value(
    value: Value,
    context: &str,
) -> Result<GalgameCharacterStaging, AppError> {
    let raw: CharacterStagingRaw = from_value(value, context)?;
    require_nonempty(
        raw.scene_lines.as_slice(),
        &format!("{context}.scene_lines"),
    )?;
    let scene_lines = raw
        .scene_lines
        .into_iter()
        .map(|scene| {
            require_positive(
                scene.scene_index,
                &format!("{context}.scene_lines[].scene_index"),
            )?;
            require_nonempty(
                scene.lines.as_slice(),
                &format!("{context}.scene_lines[].lines"),
            )?;
            let lines = scene
                .lines
                .into_iter()
                .map(|line| line_staged_from_raw(line, context))
                .collect::<Result<Vec<_>, _>>()?;
            ensure_contiguous_indexes(
                lines.iter().map(|line| line.index),
                &format!("{context}.scene_lines[{}].lines", scene.scene_index),
            )?;
            Ok(GalgamePipelineSceneLineStaged {
                scene_index: scene.scene_index,
                lines,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    ensure_unique_scene_indexes(scene_lines.iter().map(|item| item.scene_index), context)?;
    let branch_scene_lines = raw
        .branch_scene_lines
        .into_iter()
        .map(|branch| {
            let branch_id = required_text(
                branch.branch_id,
                &format!("{context}.branch_scene_lines[].branch_id"),
            )?;
            require_nonempty(
                branch.lines.as_slice(),
                &format!("{context}.branch_scene_lines[].lines"),
            )?;
            let lines = branch
                .lines
                .into_iter()
                .map(|line| line_staged_from_raw(line, context))
                .collect::<Result<Vec<_>, _>>()?;
            ensure_contiguous_indexes(
                lines.iter().map(|line| line.index),
                &format!("{context}.branch_scene_lines[{branch_id}].lines"),
            )?;
            Ok(GalgamePipelineBranchSceneLineStaged { branch_id, lines })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    let mut branch_ids = std::collections::HashSet::new();
    for branch in &branch_scene_lines {
        if !branch_ids.insert(branch.branch_id.as_str()) {
            return Err(invalid(format!(
                "{context}.branch_scene_lines.branch_id 重复: {}",
                branch.branch_id
            )));
        }
    }
    Ok(GalgameCharacterStaging {
        scene_lines,
        branch_scene_lines,
    })
}
