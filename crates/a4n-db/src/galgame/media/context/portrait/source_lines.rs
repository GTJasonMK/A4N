use std::collections::BTreeSet;

use a4n_core::models::{
    galgame::{galgame_line, galgame_scene, galgame_script},
    generation::generation_spec,
    project::{novel_project, source_snapshot_chapter},
};
use sea_orm::{DatabaseConnection, EntityTrait};

use super::positions::{
    normalized_galgame_position, validate_galgame_portrait_position_samples,
    validate_galgame_position,
};
use crate::galgame::{GalgameGenerationRecordError, GalgamePortraitSpecTargetInput};

pub(in crate::galgame) async fn validate_galgame_portrait_spec_source_lines(
    db: &DatabaseConnection,
    spec: &generation_spec::Model,
    input: &GalgamePortraitSpecTargetInput,
    project: &novel_project::Model,
    root_project_id: i64,
    source_snapshot_id: i64,
) -> Result<(), GalgameGenerationRecordError> {
    let mut source_positions = BTreeSet::new();
    for line_id in &input.source_line_ids {
        let line = galgame_line::Entity::find_by_id(*line_id)
            .one(db)
            .await
            .map_err(GalgameGenerationRecordError::Database)?
            .ok_or_else(|| {
                GalgameGenerationRecordError::NotFound(format!(
                    "Galgame 立绘差分 source_line_ids 引用了不存在的行: spec_id={}, line_id={line_id}",
                    spec.id
                ))
            })?;
        if line.speaker_profile_id != Some(input.profile_id) {
            return Err(GalgameGenerationRecordError::InvalidArgument(format!(
                "Galgame 立绘差分 source_line_ids 行角色与目标不一致: spec_id={}, line_id={}, speaker_profile_id={:?}, expected={}",
                spec.id, line.id, line.speaker_profile_id, input.profile_id
            )));
        }
        if line.expression.as_deref().map(str::trim) != Some(input.expression.as_str()) {
            return Err(GalgameGenerationRecordError::InvalidArgument(format!(
                "Galgame 立绘差分 source_line_ids 行 expression 与目标不一致: spec_id={}, line_id={}, expression={:?}, expected={}",
                spec.id, line.id, line.expression, input.expression
            )));
        }
        if line
            .pose
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            != Some(input.pose.as_str())
        {
            return Err(GalgameGenerationRecordError::InvalidArgument(format!(
                "Galgame 立绘差分 source_line_ids 行 pose 与目标不一致: spec_id={}, line_id={}, pose={:?}, expected={:?}",
                spec.id, line.id, line.pose, input.pose
            )));
        }
        if let Some(position) = normalized_galgame_position(line.position.as_deref()) {
            validate_galgame_position(position, spec.id, line.id)?;
            source_positions.insert(position.to_string());
        }
        let scene = galgame_scene::Entity::find_by_id(line.scene_id)
            .one(db)
            .await
            .map_err(GalgameGenerationRecordError::Database)?
            .ok_or_else(|| {
                GalgameGenerationRecordError::NotFound(format!(
                    "Galgame 立绘差分 source_line_ids 行所属场不存在: spec_id={}, line_id={}, scene_id={}",
                    spec.id, line.id, line.scene_id
                ))
            })?;
        let script = galgame_script::Entity::find_by_id(scene.script_id)
            .one(db)
            .await
            .map_err(GalgameGenerationRecordError::Database)?
            .ok_or_else(|| {
                GalgameGenerationRecordError::NotFound(format!(
                    "Galgame 立绘差分 source_line_ids 场所属剧本不存在: spec_id={}, scene_id={}, script_id={}",
                    spec.id, scene.id, scene.script_id
                ))
            })?;
        if script.project_id != project.id
            || script.root_project_id != root_project_id
            || script.source_snapshot_id != source_snapshot_id
        {
            return Err(GalgameGenerationRecordError::InvalidArgument(format!(
                "Galgame 立绘差分 source_line_ids 行上下文与项目不一致: spec_id={}, line_id={}, target_project_id={}, target_root_project_id={}, target_source_snapshot_id={}, spec_project_id={}, spec_root_project_id={}, spec_source_snapshot_id={:?}",
                spec.id,
                line.id,
                script.project_id,
                script.root_project_id,
                script.source_snapshot_id,
                spec.project_id,
                spec.root_project_id,
                spec.source_snapshot_id
            )));
        }
        let snapshot_chapter =
            source_snapshot_chapter::Entity::find_by_id(scene.source_snapshot_chapter_id)
                .one(db)
                .await
                .map_err(GalgameGenerationRecordError::Database)?
                .ok_or_else(|| {
                    GalgameGenerationRecordError::NotFound(format!(
                        "Galgame 立绘差分 source_line_ids 场源快照章节不存在: spec_id={}, scene_id={}, source_snapshot_chapter_id={}",
                        spec.id, scene.id, scene.source_snapshot_chapter_id
                    ))
                })?;
        if snapshot_chapter.snapshot_id != source_snapshot_id {
            return Err(GalgameGenerationRecordError::InvalidArgument(format!(
                "Galgame 立绘差分 source_line_ids 场源快照章节与项目不一致: spec_id={}, scene_id={}, source_snapshot_chapter_id={}, expected_source_snapshot_id={}, actual_source_snapshot_id={}",
                spec.id,
                scene.id,
                scene.source_snapshot_chapter_id,
                source_snapshot_id,
                snapshot_chapter.snapshot_id
            )));
        }
    }
    validate_galgame_portrait_position_samples(spec.id, &input.position_samples, &source_positions)
}
