use std::collections::{BTreeMap, BTreeSet};

use super::super::super::{
    GalgameBranchDesign, GalgameCharacterStaging, GalgameShootingScriptProjection,
    GalgameVisualPlan, SCENE_ENTRY_MAIN,
};
use super::branches::*;
use super::contracts::*;
use super::rules::*;
use super::sources::*;
use super::visual::*;
use a4n_error::AppError;

pub fn build_galgame_script_persist_plan(
    scenes: &GalgameShootingScriptProjection,
    staging: &GalgameCharacterStaging,
    visual_plan: &GalgameVisualPlan,
    branch_design: &GalgameBranchDesign,
    source_spans: &[GalgamePipelineSourceSpan],
) -> Result<GalgameScriptPersistPlan, AppError> {
    if scenes.scenes.is_empty() {
        return Err(invalid("Galgame persist scenes 不能为空"));
    }
    let source_span_by_id = source_spans
        .iter()
        .map(|span| (span.id.as_str(), span))
        .collect::<BTreeMap<_, _>>();
    let staged_by_scene = staging
        .scene_lines
        .iter()
        .map(|scene| (scene.scene_index, scene))
        .collect::<BTreeMap<_, _>>();
    let staged_line_indexes_by_scene = staging
        .scene_lines
        .iter()
        .map(|scene| {
            (
                scene.scene_index,
                scene
                    .lines
                    .iter()
                    .map(|line| line.index)
                    .collect::<BTreeSet<_>>(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let scene_indexes = scenes
        .scenes
        .iter()
        .map(|scene| scene.index)
        .collect::<BTreeSet<_>>();
    validate_staging_scene_indexes(&staged_line_indexes_by_scene, &scene_indexes)?;
    let (background_by_scene, cg_by_scene_line) = visual_plan_mappings(
        visual_plan,
        &scene_indexes,
        &staged_line_indexes_by_scene,
        &source_span_by_id,
    )?;
    let choices_by_beat_ref = branch_choices_by_beat_ref(branch_design, &scenes.scenes)?;
    let branch_lines_by_id = branch_lines_by_id(branch_design, &staging.branch_scene_lines)?;

    let main_scene_indexes = scenes
        .scenes
        .iter()
        .map(|scene| scene.index)
        .collect::<Vec<_>>();
    let main_next_scene_index = main_scene_indexes
        .iter()
        .enumerate()
        .map(|(index, scene_index)| (*scene_index, main_scene_indexes.get(index + 1).copied()))
        .collect::<BTreeMap<_, _>>();
    let max_main_scene_index = scene_indexes
        .iter()
        .next_back()
        .copied()
        .ok_or_else(|| invalid("Galgame persist scenes 不能为空"))?;
    let mut next_branch_scene_index = max_main_scene_index
        .checked_add(1)
        .ok_or_else(|| invalid("Galgame branch scene_index 超出 i32 范围"))?;
    let mut persist_scenes = Vec::with_capacity(scenes.scenes.len());
    let mut branch_scenes = Vec::new();
    for scene in &scenes.scenes {
        let source_snapshot_chapter_id = source_snapshot_chapter_id_for_scene(
            scene.source_span_ids.as_slice(),
            &source_span_by_id,
        )?;
        let Some(staged_scene) = staged_by_scene.get(&scene.index) else {
            return Err(invalid(format!(
                "Galgame persist 缺少 scene_index={} 的 staged lines",
                scene.index
            )));
        };
        if staged_scene.lines.is_empty() {
            return Err(invalid(format!(
                "Galgame persist scene_index={} 没有可写入的行",
                scene.index
            )));
        }
        let last_line_position_by_beat = staged_scene
            .lines
            .iter()
            .enumerate()
            .map(|(position, line)| (line.beat_ref, position))
            .collect::<BTreeMap<_, _>>();
        let mut participants = BTreeMap::<i64, String>::new();
        let mut persist_lines = Vec::with_capacity(staged_scene.lines.len());
        for (line_position, line) in staged_scene.lines.iter().enumerate() {
            if line.beat_ref.scene_index != scene.index {
                return Err(invalid(format!(
                    "Galgame staged line beat_ref.scene_index 与所在主线场不一致: scene_index={}, line_index={}, beat_scene_index={}",
                    scene.index, line.index, line.beat_ref.scene_index
                )));
            }
            let text_anchor_id =
                text_anchor_id_for_line(line, &source_span_by_id, source_snapshot_chapter_id)?;
            let presentation_json = serde_json::to_value(presentation_document_for_line(
                line,
                cg_by_scene_line.get(&(scene.index, line.index)).copied(),
            ))
            .map_err(AppError::Serialization)?;
            validate_persist_line(line)?;
            if let Some(profile_id) = line.speaker_profile_id {
                let speaker_name = line.speaker_name.clone().ok_or_else(|| {
                    invalid(format!(
                        "Galgame persist 已绑定角色缺少 speaker_name: profile_id={profile_id}"
                    ))
                })?;
                participants.entry(profile_id).or_insert(speaker_name);
            }
            let speaker = speaker_projection(line);
            persist_lines.push(GalgamePersistLine {
                line_index: to_i32(persist_lines.len() + 1, "line_index")?,
                line_type: line.line_type.clone(),
                speaker_profile_id: line.speaker_profile_id,
                text: line.text.clone(),
                expression: speaker.map(|cast| cast.expression.clone()),
                pose: speaker.map(|cast| cast.pose.clone()),
                position: speaker.map(|cast| cast.position.to_string()),
                presentation_json,
                text_anchor_id,
                choices: Vec::new(),
            });
            if last_line_position_by_beat.get(&line.beat_ref) == Some(&line_position)
                && let Some(choice) = choices_by_beat_ref.get(&line.beat_ref)
            {
                let (choice_line, mut generated_branch_scenes) = choice_line_and_branch_scenes(
                    choice,
                    persist_lines.len() + 1,
                    &mut next_branch_scene_index,
                    GalgameBranchSceneContext {
                        source_snapshot_chapter_id,
                        location: scene.location.clone(),
                        time_of_day: scene.time_of_day.clone(),
                        source_scene_summary: scene.summary.clone(),
                        branch_lines_by_id: &branch_lines_by_id,
                        source_span_by_id: &source_span_by_id,
                    },
                )?;
                persist_lines.push(choice_line);
                branch_scenes.append(&mut generated_branch_scenes);
            }
        }
        let background = background_by_scene.get(&scene.index).ok_or_else(|| {
            invalid(format!(
                "Galgame visual_planning 缺少 scene_index={} 的 background",
                scene.index
            ))
        })?;
        persist_scenes.push(GalgamePersistScene {
            scene_index: scene.index,
            source_snapshot_chapter_id,
            title: scene.title.clone(),
            location: scene.location.clone(),
            time_of_day: scene.time_of_day.clone(),
            participants_json: participants_json(participants),
            background_note: Some(background.background_note.clone()),
            entry_kind: SCENE_ENTRY_MAIN.to_string(),
            next_scene_index: main_next_scene_index.get(&scene.index).copied().flatten(),
            lines: persist_lines,
        });
    }
    persist_scenes.extend(branch_scenes);

    Ok(GalgameScriptPersistPlan {
        scenes: persist_scenes,
    })
}
