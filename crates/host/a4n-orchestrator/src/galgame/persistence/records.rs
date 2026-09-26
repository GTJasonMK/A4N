use a4n_db::galgame::{
    PersistGalgameChoiceRecordInput, PersistGalgameLineRecordInput, PersistGalgameSceneRecordInput,
    PersistGalgameScriptRecordInput,
};
use a4n_error::AppError;
use serde_json::json;

use crate::galgame::{
    GalgameChapterShootingScript, GalgamePersistChoice, GalgamePersistLine, GalgamePersistScene,
    GalgameScriptPersistPlan, SCRIPT_STATUS_READY, normalize_optional_text, normalize_position,
    required_text,
};
use a4n_db::project::ProjectFamilyContext;

pub(crate) fn galgame_script_record_input(
    context: &ProjectFamilyContext,
    plan: GalgameScriptPersistPlan,
    shooting_scripts: &[GalgameChapterShootingScript],
) -> Result<PersistGalgameScriptRecordInput, AppError> {
    let scenes = plan
        .scenes
        .into_iter()
        .map(|scene| galgame_scene_record_input(scene, shooting_scripts))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PersistGalgameScriptRecordInput {
        project_id: context.project.id,
        root_project_id: context.root_project.id,
        source_snapshot_id: context.source_snapshot.id,
        title: context.project.title.clone(),
        style_config_json: json!({
            "pipeline": "galgame_v1",
        }),
        status: SCRIPT_STATUS_READY.to_string(),
        scenes,
    })
}

fn galgame_scene_record_input(
    scene: GalgamePersistScene,
    shooting_scripts: &[GalgameChapterShootingScript],
) -> Result<PersistGalgameSceneRecordInput, AppError> {
    let shooting_scene_identity = match &scene.shooting_source {
        Some(source) => {
            let shooting_scene = shooting_scripts
                .iter()
                .find(|script| {
                    script.shooting_script_id == source.script_id
                        && script.shooting_script_revision == source.script_revision
                        && script.source_snapshot_chapter_id == scene.source_snapshot_chapter_id
                })
                .and_then(|script| {
                    script
                        .script
                        .scenes
                        .iter()
                        .find(|row| row.scene_index == source.scene_index)
                })
                .ok_or_else(|| {
                    AppError::InvalidArgument(format!(
                        "Galgame 场 {} 的拍摄剧本来源不在本次输入中",
                        scene.scene_index
                    ))
                })?;
            Some(shooting_scene.scene_identity.clone())
        }
        None if scene.entry_kind == crate::galgame::SCENE_ENTRY_BRANCH => None,
        None => {
            return Err(AppError::InvalidArgument(format!(
                "Galgame 主线场 {} 缺少拍摄剧本来源",
                scene.scene_index
            )));
        }
    };
    let lines = scene
        .lines
        .into_iter()
        .map(galgame_line_record_input)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PersistGalgameSceneRecordInput {
        scene_index: scene.scene_index,
        source_snapshot_chapter_id: scene.source_snapshot_chapter_id,
        shooting_scene_identity,
        title: required_text(&scene.title, "scene.title")?.to_string(),
        location: normalize_optional_text(scene.location),
        time_of_day: normalize_optional_text(scene.time_of_day),
        participants_json: scene.participants_json,
        background_note: normalize_optional_text(scene.background_note),
        entry_kind: scene.entry_kind,
        next_scene_index: scene.next_scene_index,
        lines,
    })
}

fn galgame_line_record_input(
    line: GalgamePersistLine,
) -> Result<PersistGalgameLineRecordInput, AppError> {
    let choices = line
        .choices
        .into_iter()
        .map(galgame_choice_record_input)
        .collect::<Result<Vec<_>, _>>()?;
    Ok(PersistGalgameLineRecordInput {
        line_index: line.line_index,
        line_type: line.line_type,
        speaker_profile_id: line.speaker_profile_id,
        text: required_text(&line.text, "line.text")?.to_string(),
        expression: normalize_optional_text(line.expression),
        pose: normalize_optional_text(line.pose),
        position: normalize_position(line.position)?,
        presentation_json: line.presentation_json,
        text_anchor_id: line.text_anchor_id,
        choices,
    })
}

fn galgame_choice_record_input(
    choice: GalgamePersistChoice,
) -> Result<PersistGalgameChoiceRecordInput, AppError> {
    Ok(PersistGalgameChoiceRecordInput {
        option_index: choice.option_index,
        option_text: required_text(&choice.option_text, "choice.option_text")?.to_string(),
        target_scene_index: choice.target_scene_index,
    })
}
