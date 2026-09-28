use std::collections::{BTreeMap, BTreeSet};

use a4n_error::AppError;

use crate::galgame::{
    GalgameAdaptationProjection, GalgameBranchDesign, GalgameCharacterStaging,
    GalgamePipelineLineDraft, GalgamePipelineLineStaged, GalgamePipelineSourceSpan,
    GalgameScriptAdaptation, GalgameSpeakerProfile,
};

use super::{
    branches::branch_choices_by_beat_ref,
    rules::{invalid, validate_persist_line},
    sources::{source_snapshot_chapter_id_for_scene, source_snapshot_chapter_id_for_spans},
};

pub(crate) fn validate_galgame_branch_design(
    branch: &GalgameBranchDesign,
    scenes: &GalgameAdaptationProjection,
) -> Result<(), AppError> {
    branch_choices_by_beat_ref(branch, &scenes.scenes).map(drop)
}

pub(in crate::galgame) fn validate_galgame_script_adaptation(
    script: &GalgameScriptAdaptation,
    scenes: &GalgameAdaptationProjection,
    branch: &GalgameBranchDesign,
    source_spans: &[GalgamePipelineSourceSpan],
) -> Result<(), AppError> {
    let scenes = scenes
        .scenes
        .iter()
        .map(|scene| (scene.index, scene))
        .collect::<BTreeMap<_, _>>();
    let source_spans = source_spans
        .iter()
        .map(|span| (span.id.as_str(), span))
        .collect::<BTreeMap<_, _>>();
    let mut branches = BTreeMap::new();
    for choice in &branch.choices {
        let scene = scenes
            .values()
            .find(|scene| scene.contains_beat(&choice.after_beat_ref))
            .ok_or_else(|| invalid("Galgame 分支引用了不存在的改编单元"))?;
        for option in &choice.options {
            branches.insert(option.branch_id.as_str(), scene.index);
        }
    }
    if script
        .scene_lines
        .iter()
        .map(|scene| scene.scene_index)
        .collect::<BTreeSet<_>>()
        != scenes.keys().copied().collect()
        || script
            .branch_scene_lines
            .iter()
            .map(|scene| scene.branch_id.as_str())
            .collect::<BTreeSet<_>>()
            != branches.keys().copied().collect()
    {
        return Err(invalid(
            "Galgame script_adaptation 主线场景和 branch_id 必须与输入逐一对应",
        ));
    }
    let validate_lines = |scene_index, lines: &[GalgamePipelineLineDraft], ordered| {
        let scene = scenes.get(&scene_index).ok_or_else(|| {
            invalid(format!(
                "Galgame script_adaptation scene_index 不存在: {scene_index}"
            ))
        })?;
        let chapter_id = source_snapshot_chapter_id_for_scene(scene, &source_spans)?;
        let beats = scene
            .beats
            .iter()
            .enumerate()
            .map(|(position, beat)| (scene.beat_ref(beat), position))
            .collect::<BTreeMap<_, _>>();
        let mut previous = None;
        for line in lines {
            let position = beats
                .get(&line.beat_ref)
                .copied()
                .ok_or_else(|| invalid(format!("Galgame 行 {} 引用未知改编节拍", line.index)))?;
            if ordered && previous.is_some_and(|previous| previous > position) {
                return Err(invalid(format!(
                    "Galgame script_adaptation line {} 引用了乱序的改编节拍",
                    line.index
                )));
            }
            let beat = &scene.beats[position];
            if (beat.origin == a4n_domain::media_authoring::AdaptationOrigin::Added
                && line.origin != beat.origin)
                || (line.origin == a4n_domain::media_authoring::AdaptationOrigin::Historical
                    && line.origin != beat.origin)
            {
                return Err(invalid("剧本行的内容来源与改编节拍不一致"));
            }
            a4n_domain::media_authoring::validate_adaptation_evidence(
                line.origin,
                &line.source_span_ids,
                &beat.source_span_ids.iter().cloned().collect(),
            )
            .map_err(invalid)?;
            if !line.source_span_ids.is_empty()
                && source_snapshot_chapter_id_for_spans(&line.source_span_ids, &source_spans)?
                    != chapter_id
            {
                return Err(invalid(format!(
                    "Galgame script_adaptation line {} 来源不属于当前场章节",
                    line.index
                )));
            }
            previous = Some(position);
        }
        Ok(())
    };
    for scene in &script.scene_lines {
        validate_lines(scene.scene_index, &scene.lines, true)?;
    }
    for branch in &script.branch_scene_lines {
        validate_lines(branches[branch.branch_id.as_str()], &branch.lines, false)?;
    }
    let used_beats = script
        .scene_lines
        .iter()
        .flat_map(|scene| scene.lines.iter().map(|line| line.beat_ref.clone()))
        .collect::<BTreeSet<_>>();
    for scene in scenes.values() {
        for beat in &scene.beats {
            if beat.importance == a4n_domain::media_authoring::AdaptationImportance::Required
                && !used_beats.contains(&scene.beat_ref(beat))
            {
                return Err(invalid(format!(
                    "Galgame 剧本遗漏必须保留的改编节拍: {} / {}",
                    scene.unit_id, beat.id
                )));
            }
        }
    }
    if branch
        .choices
        .iter()
        .any(|choice| !used_beats.contains(&choice.after_beat_ref))
    {
        return Err(invalid(
            "Galgame script_adaptation 遗漏了分支选择点对应的拍摄节拍",
        ));
    }
    Ok(())
}

pub(in crate::galgame) fn validate_galgame_staging(
    staging: &GalgameCharacterStaging,
    script: &GalgameScriptAdaptation,
    profiles: &[GalgameSpeakerProfile],
) -> Result<(), AppError> {
    let expected_scenes = script
        .scene_lines
        .iter()
        .map(|scene| (scene.scene_index, &scene.lines))
        .collect::<BTreeMap<_, _>>();
    let expected_branches = script
        .branch_scene_lines
        .iter()
        .map(|scene| (scene.branch_id.as_str(), &scene.lines))
        .collect::<BTreeMap<_, _>>();
    if staging.scene_lines.len() != expected_scenes.len()
        || staging.branch_scene_lines.len() != expected_branches.len()
    {
        return Err(invalid("Galgame character_staging 不得增删主线或支线"));
    }
    let profile_ids = profiles
        .iter()
        .map(|profile| profile.id)
        .collect::<BTreeSet<_>>();
    let validate_lines = |lines: &[GalgamePipelineLineStaged],
                          expected: &[GalgamePipelineLineDraft]| {
        if lines.len() != expected.len() {
            return Err(invalid("Galgame character_staging 不得增删剧本行"));
        }
        for (line, draft) in lines.iter().zip(expected) {
            if line.index != draft.index
                || line.origin != draft.origin
                || line.beat_ref != draft.beat_ref
                || line.line_type != draft.line_type
                || line.text != draft.text
                || line.source_span_ids != draft.source_span_ids
            {
                return Err(invalid(format!(
                    "Galgame character_staging line {} 必须保留原剧本行和拍摄节拍",
                    line.index
                )));
            }
            validate_persist_line(line)?;
            if line
                .presentation
                .visible_cast
                .iter()
                .map(|cast| cast.character_profile_id)
                .chain(line.presentation.exit.iter().copied())
                .any(|profile_id| !profile_ids.contains(&profile_id))
            {
                return Err(invalid(format!(
                    "Galgame character_staging line {} 引用了输入人物档案以外的 profile_id",
                    line.index
                )));
            }
        }
        Ok(())
    };
    for scene in &staging.scene_lines {
        let expected = expected_scenes
            .get(&scene.scene_index)
            .ok_or_else(|| invalid("Galgame character_staging 引用了不存在的主线场景"))?;
        validate_lines(&scene.lines, expected)?;
    }
    for branch in &staging.branch_scene_lines {
        let expected = expected_branches
            .get(branch.branch_id.as_str())
            .ok_or_else(|| invalid("Galgame character_staging 引用了不存在的 branch_id"))?;
        validate_lines(&branch.lines, expected)?;
    }
    Ok(())
}
