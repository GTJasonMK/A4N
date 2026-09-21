use std::collections::{BTreeMap, BTreeSet};

use a4n_error::AppError;
use serde_json::Value;

use super::super::{
    GalgameBranchDesign, GalgameLinePresentationDocumentV2, GalgameLinePresentationSchemaVersion,
    GalgameLinePresentationV2, GalgamePipelineBeatRefV3, GalgamePipelineBranchSceneLineStaged,
    GalgamePipelineChoicePlan, GalgamePipelineScene, LINE_TYPE_CHOICE, SCENE_ENTRY_BRANCH,
};
use super::contracts::*;
use super::rules::{
    invalid, participants_json, presentation_document_for_line, speaker_projection, to_i32,
    validate_persist_line,
};
use super::sources::text_anchor_id_for_line;

pub(super) fn branch_choices_by_beat_ref<'a>(
    branch_design: &'a GalgameBranchDesign,
    scenes: &[GalgamePipelineScene],
) -> Result<BTreeMap<GalgamePipelineBeatRefV3, &'a GalgamePipelineChoicePlan>, AppError> {
    let scene_indexes = scenes
        .iter()
        .map(|scene| scene.index)
        .collect::<BTreeSet<_>>();
    let valid_beats = scenes
        .iter()
        .flat_map(|scene| {
            scene
                .beats
                .iter()
                .map(move |beat| GalgamePipelineBeatRefV3 {
                    scene_index: scene.index,
                    shot_index: beat.shot_index,
                    beat_index: beat.beat_index,
                })
        })
        .collect::<BTreeSet<_>>();
    let mut branch_ids = BTreeSet::new();
    let mut result = BTreeMap::new();
    for choice in &branch_design.choices {
        if !valid_beats.contains(&choice.after_beat_ref) {
            return Err(invalid(format!(
                "Galgame branch_design after_beat_ref 不存在: scene_index={}, shot_index={}, beat_index={}",
                choice.after_beat_ref.scene_index,
                choice.after_beat_ref.shot_index,
                choice.after_beat_ref.beat_index
            )));
        }
        for option in &choice.options {
            if !branch_ids.insert(option.branch_id.as_str()) {
                return Err(invalid(format!(
                    "Galgame branch_design branch_id 重复: {}",
                    option.branch_id
                )));
            }
            let target_scene_index = option.returns_to_scene_index;
            if !scene_indexes.contains(&target_scene_index) {
                return Err(invalid(format!(
                    "Galgame branch_design option returns_to_scene_index 不存在: {target_scene_index}"
                )));
            }
        }
        if result.insert(choice.after_beat_ref, choice).is_some() {
            return Err(invalid(format!(
                "Galgame branch_design 同一拍摄节拍只能有一个选择点: scene_index={}, shot_index={}, beat_index={}",
                choice.after_beat_ref.scene_index,
                choice.after_beat_ref.shot_index,
                choice.after_beat_ref.beat_index
            )));
        }
    }
    Ok(result)
}

pub(super) fn branch_lines_by_id<'a>(
    branch_design: &GalgameBranchDesign,
    staged_branches: &'a [GalgamePipelineBranchSceneLineStaged],
) -> Result<BTreeMap<&'a str, &'a GalgamePipelineBranchSceneLineStaged>, AppError> {
    let expected = branch_design
        .choices
        .iter()
        .flat_map(|choice| {
            choice
                .options
                .iter()
                .map(|option| option.branch_id.as_str())
        })
        .collect::<BTreeSet<_>>();
    let actual = staged_branches
        .iter()
        .map(|branch| branch.branch_id.as_str())
        .collect::<BTreeSet<_>>();
    if expected != actual {
        return Err(invalid(format!(
            "Galgame character_staging.branch_scene_lines 必须与 branch_design branch_id 一一对应: expected={expected:?}, actual={actual:?}"
        )));
    }
    Ok(staged_branches
        .iter()
        .map(|branch| (branch.branch_id.as_str(), branch))
        .collect())
}

pub(super) fn choice_line_and_branch_scenes(
    choice: &GalgamePipelineChoicePlan,
    line_index: usize,
    next_branch_scene_index: &mut i32,
    context: GalgameBranchSceneContext<'_>,
) -> Result<(GalgamePersistLine, Vec<GalgamePersistScene>), AppError> {
    let mut branch_scenes = Vec::with_capacity(choice.options.len());
    let choices = choice
        .options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let staged_branch = context
                .branch_lines_by_id
                .get(option.branch_id.as_str())
                .ok_or_else(|| {
                    invalid(format!(
                        "Galgame 支线缺少 staged lines: {}",
                        option.branch_id
                    ))
                })?;
            let branch_scene_index = *next_branch_scene_index;
            *next_branch_scene_index = next_branch_scene_index
                .checked_add(1)
                .ok_or_else(|| invalid("Galgame branch scene_index 超出 i32 范围"))?;
            let return_scene_index = option.returns_to_scene_index;
            let mut participants = BTreeMap::<i64, String>::new();
            let lines = staged_branch
                .lines
                .iter()
                .enumerate()
                .map(|(line_offset, line)| {
                    validate_persist_line(line)?;
                    let text_anchor_id = text_anchor_id_for_line(
                        line,
                        context.source_span_by_id,
                        context.source_snapshot_chapter_id,
                    )?;
                    if let Some(profile_id) = line.speaker_profile_id {
                        let speaker_name = line.speaker_name.clone().ok_or_else(|| {
                            invalid(format!(
                                "Galgame persist 支线已绑定角色缺少 speaker_name: branch_id={}, profile_id={profile_id}",
                                option.branch_id
                            ))
                        })?;
                        participants
                            .entry(profile_id)
                            .or_insert(speaker_name);
                    }
                    let mut presentation = presentation_document_for_line(line, None);
                    presentation.branch = Some(true);
                    presentation.branch_id = Some(option.branch_id.clone());
                    presentation.returns_to_scene_index = Some(return_scene_index);
                    let presentation_json = serde_json::to_value(presentation)
                        .map_err(AppError::Serialization)?;
                    let speaker = speaker_projection(line);
                    Ok(GalgamePersistLine {
                        line_index: to_i32(line_offset + 1, "branch line_index")?,
                        line_type: line.line_type.clone(),
                        speaker_profile_id: line.speaker_profile_id,
                        text: line.text.clone(),
                        expression: speaker.map(|cast| cast.expression.clone()),
                        pose: speaker.map(|cast| cast.pose.clone()),
                        position: speaker.map(|cast| cast.position.to_string()),
                        presentation_json,
                        text_anchor_id,
                        choices: Vec::new(),
                    })
                })
                .collect::<Result<Vec<_>, AppError>>()?;
            branch_scenes.push(GalgamePersistScene {
                scene_index: branch_scene_index,
                source_snapshot_chapter_id: context.source_snapshot_chapter_id,
                title: option.branch_title.clone(),
                location: context.location.clone(),
                time_of_day: context.time_of_day.clone(),
                participants_json: participants_json(participants),
                background_note: context.source_scene_summary.clone(),
                entry_kind: SCENE_ENTRY_BRANCH.to_string(),
                next_scene_index: Some(return_scene_index),
                lines,
            });
            Ok(GalgamePersistChoice {
                option_index: to_i32(index + 1, "option_index")?,
                option_text: option.option_text.clone(),
                target_scene_index: branch_scene_index,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok((
        GalgamePersistLine {
            line_index: to_i32(line_index, "line_index")?,
            line_type: LINE_TYPE_CHOICE.to_string(),
            speaker_profile_id: None,
            text: choice.question.clone(),
            expression: None,
            pose: None,
            position: None,
            presentation_json: choice_presentation_json(&choice.after_beat_ref)?,
            text_anchor_id: None,
            choices,
        },
        branch_scenes,
    ))
}

fn choice_presentation_json(beat_ref: &GalgamePipelineBeatRefV3) -> Result<Value, AppError> {
    let mut presentation = GalgameLinePresentationDocumentV2::from(GalgameLinePresentationV2 {
        schema_version: GalgameLinePresentationSchemaVersion::V2,
        visible_cast: Vec::new(),
        enter: Vec::new(),
        exit: Vec::new(),
        focus_character_profile_id: None,
        hold: true,
        player_knowledge: Vec::new(),
    });
    presentation.branch = Some(true);
    presentation.after_beat_ref = Some(*beat_ref);
    serde_json::to_value(presentation).map_err(AppError::Serialization)
}
