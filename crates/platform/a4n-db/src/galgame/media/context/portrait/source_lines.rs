use std::collections::{BTreeSet, HashMap, HashSet};

use a4n_db_models::models::{
    galgame::{galgame_line, galgame_scene, galgame_script},
    generation::generation_spec,
    project::source_snapshot_chapter,
};
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, FromQueryResult, QueryFilter, QuerySelect,
};

use super::positions::{
    normalized_galgame_position, validate_galgame_portrait_position_samples,
    validate_galgame_position,
};
use crate::galgame::{GalgameGenerationRecordError, GalgamePortraitSpecTargetInput};

#[derive(Debug, FromQueryResult)]
struct PortraitSourceLineRecord {
    id: i64,
    speaker_profile_id: Option<i64>,
    expression: Option<String>,
    pose: Option<String>,
    position: Option<String>,
    scene_id: i64,
}

#[derive(Debug, FromQueryResult)]
struct PortraitSourceSceneRecord {
    id: i64,
    script_id: i64,
    source_snapshot_chapter_id: i64,
}

#[derive(Debug, FromQueryResult)]
struct PortraitSourceScriptRecord {
    id: i64,
    project_id: i64,
    root_project_id: i64,
    source_snapshot_id: i64,
}

#[derive(Debug, FromQueryResult)]
struct PortraitSourceSnapshotChapterRecord {
    id: i64,
    snapshot_id: i64,
}

pub(in crate::galgame) async fn validate_galgame_portrait_spec_source_lines(
    db: &DatabaseConnection,
    spec: &generation_spec::Model,
    input: &GalgamePortraitSpecTargetInput,
    project_id: i64,
    root_project_id: i64,
    source_snapshot_id: i64,
) -> Result<(), GalgameGenerationRecordError> {
    let mut source_positions = BTreeSet::new();
    if input.source_line_ids.is_empty() {
        return validate_galgame_portrait_position_samples(
            spec.id,
            &input.position_samples,
            &source_positions,
        );
    }
    let lines_by_id = galgame_line::Entity::find()
        .select_only()
        .column(galgame_line::Column::Id)
        .column(galgame_line::Column::SpeakerProfileId)
        .column(galgame_line::Column::Expression)
        .column(galgame_line::Column::Pose)
        .column(galgame_line::Column::Position)
        .column(galgame_line::Column::SceneId)
        .filter(galgame_line::Column::Id.is_in(input.source_line_ids.iter().copied()))
        .into_model::<PortraitSourceLineRecord>()
        .all(db)
        .await
        .map_err(GalgameGenerationRecordError::Database)?
        .into_iter()
        .map(|line| (line.id, line))
        .collect::<HashMap<_, _>>();
    let ordered_lines = input
        .source_line_ids
        .iter()
        .map(|line_id| {
            lines_by_id.get(line_id).ok_or_else(|| {
                GalgameGenerationRecordError::NotFound(format!(
                    "Galgame 立绘差分 source_line_ids 引用了不存在的行: spec_id={}, line_id={line_id}",
                    spec.id
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut scene_ids = HashSet::new();
    for line in &ordered_lines {
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
        scene_ids.insert(line.scene_id);
    }
    let scenes_by_id = galgame_scene::Entity::find()
        .select_only()
        .column(galgame_scene::Column::Id)
        .column(galgame_scene::Column::ScriptId)
        .column(galgame_scene::Column::SourceSnapshotChapterId)
        .filter(galgame_scene::Column::Id.is_in(scene_ids.iter().copied()))
        .into_model::<PortraitSourceSceneRecord>()
        .all(db)
        .await
        .map_err(GalgameGenerationRecordError::Database)?
        .into_iter()
        .map(|scene| (scene.id, scene))
        .collect::<HashMap<_, _>>();
    let ordered_scenes = ordered_lines
        .iter()
        .map(|line| {
            scenes_by_id.get(&line.scene_id).ok_or_else(|| {
                GalgameGenerationRecordError::NotFound(format!(
                    "Galgame 立绘差分 source_line_ids 行所属场不存在: spec_id={}, line_id={}, scene_id={}",
                    spec.id, line.id, line.scene_id
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut script_ids = HashSet::new();
    for scene in &ordered_scenes {
        script_ids.insert(scene.script_id);
    }
    let scripts_by_id = galgame_script::Entity::find()
        .select_only()
        .column(galgame_script::Column::Id)
        .column(galgame_script::Column::ProjectId)
        .column(galgame_script::Column::RootProjectId)
        .column(galgame_script::Column::SourceSnapshotId)
        .filter(galgame_script::Column::Id.is_in(script_ids.iter().copied()))
        .into_model::<PortraitSourceScriptRecord>()
        .all(db)
        .await
        .map_err(GalgameGenerationRecordError::Database)?
        .into_iter()
        .map(|script| (script.id, script))
        .collect::<HashMap<_, _>>();
    let ordered_scripts = ordered_scenes
        .iter()
        .map(|scene| {
            scripts_by_id.get(&scene.script_id).ok_or_else(|| {
                GalgameGenerationRecordError::NotFound(format!(
                    "Galgame 立绘差分 source_line_ids 场所属剧本不存在: spec_id={}, scene_id={}, script_id={}",
                    spec.id, scene.id, scene.script_id
                ))
            })
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut snapshot_chapter_ids = HashSet::new();
    for ((line, scene), script) in ordered_lines
        .iter()
        .zip(&ordered_scenes)
        .zip(&ordered_scripts)
    {
        if script.project_id != project_id
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
        snapshot_chapter_ids.insert(scene.source_snapshot_chapter_id);
    }
    let snapshots_by_id = source_snapshot_chapter::Entity::find()
        .select_only()
        .column(source_snapshot_chapter::Column::Id)
        .column(source_snapshot_chapter::Column::SnapshotId)
        .filter(source_snapshot_chapter::Column::Id.is_in(snapshot_chapter_ids.iter().copied()))
        .into_model::<PortraitSourceSnapshotChapterRecord>()
        .all(db)
        .await
        .map_err(GalgameGenerationRecordError::Database)?
        .into_iter()
        .map(|snapshot_chapter| (snapshot_chapter.id, snapshot_chapter))
        .collect::<HashMap<_, _>>();
    for (_line, scene) in ordered_lines.iter().zip(&ordered_scenes) {
        let snapshot_chapter_id = scene.source_snapshot_chapter_id;
        let snapshot_chapter = snapshots_by_id.get(&snapshot_chapter_id).ok_or_else(|| {
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
