use std::collections::{BTreeMap, BTreeSet};

use a4n_core::error::AppError;
use serde::Serialize;
use serde_json::{Value, json};

use super::{
    GALGAME_EXPRESSIONS, GalgameBranchDesign, GalgameCharacterStaging,
    GalgamePipelineBackgroundPlan, GalgamePipelineChoicePlan, GalgamePipelineEventCgPlan,
    GalgamePipelineLineStaged, GalgameSceneSegmentation, GalgameVisualPlan, LINE_TYPE_CHOICE,
    LINE_TYPE_DIALOGUE, LINE_TYPE_MONOLOGUE, LINE_TYPE_NARRATION, LINE_TYPE_STAGE_DIRECTION,
    SCENE_ENTRY_BRANCH, SCENE_ENTRY_MAIN,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgamePipelineSourceSpan {
    pub id: String,
    pub text_anchor_id: i64,
    pub source_snapshot_chapter_id: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GalgameScriptPersistPlan {
    pub scenes: Vec<GalgamePersistScene>,
}

struct GalgameBranchSceneContext {
    source_snapshot_chapter_id: i64,
    location: Option<String>,
    time_of_day: Option<String>,
    participants_json: Value,
    source_scene_summary: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GalgamePersistScene {
    pub scene_index: i32,
    pub source_snapshot_chapter_id: i64,
    pub title: String,
    pub location: Option<String>,
    pub time_of_day: Option<String>,
    pub participants_json: Value,
    pub background_note: Option<String>,
    pub entry_kind: String,
    pub next_scene_index: Option<i32>,
    pub lines: Vec<GalgamePersistLine>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GalgamePersistLine {
    pub line_index: i32,
    pub line_type: String,
    pub speaker_profile_id: Option<i64>,
    pub text: String,
    pub expression: Option<String>,
    pub pose: Option<String>,
    pub position: Option<String>,
    pub presentation_json: Value,
    pub text_anchor_id: Option<i64>,
    pub choices: Vec<GalgamePersistChoice>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GalgamePersistChoice {
    pub option_index: i32,
    pub option_text: String,
    pub target_scene_index: i32,
}

pub fn build_galgame_script_persist_plan(
    scenes: &GalgameSceneSegmentation,
    staging: &GalgameCharacterStaging,
    visual_plan: &GalgameVisualPlan,
    branch_design: Option<&GalgameBranchDesign>,
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
    let choices_by_scene_line = branch_design
        .map(|plan| {
            branch_choices_by_scene_line(plan, &scene_indexes, &staged_line_indexes_by_scene)
        })
        .transpose()?
        .unwrap_or_default();

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
    let mut next_branch_scene_index = scene_indexes.iter().next_back().copied().unwrap_or(0) + 1;
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
        let mut participants = BTreeMap::<i64, String>::new();
        let mut persist_lines = Vec::with_capacity(staged_scene.lines.len());
        for line in &staged_scene.lines {
            let text_anchor_id =
                text_anchor_id_for_line(line, &source_span_by_id, source_snapshot_chapter_id)?;
            let presentation_json =
                presentation_json_for_line(line, cg_by_scene_line.get(&(scene.index, line.index)));
            validate_persist_line(line)?;
            if let Some(profile_id) = line.speaker_profile_id {
                participants
                    .entry(profile_id)
                    .or_insert_with(|| line.speaker_name.clone().unwrap_or_default());
            }
            persist_lines.push(GalgamePersistLine {
                line_index: to_i32(persist_lines.len() + 1, "line_index")?,
                line_type: line.line_type.clone(),
                speaker_profile_id: line.speaker_profile_id,
                text: line.text.clone(),
                expression: line.expression.clone(),
                pose: line.pose.clone(),
                position: line.position.clone(),
                presentation_json,
                text_anchor_id,
                choices: Vec::new(),
            });
            if let Some(choice) = choices_by_scene_line.get(&(scene.index, line.index)) {
                let (choice_line, mut generated_branch_scenes) = choice_line_and_branch_scenes(
                    choice,
                    persist_lines.len() + 1,
                    &mut next_branch_scene_index,
                    GalgameBranchSceneContext {
                        source_snapshot_chapter_id,
                        location: scene.location.clone(),
                        time_of_day: scene.time_of_day.clone(),
                        participants_json: participants_json(participants.clone()),
                        source_scene_summary: scene.summary.clone(),
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

fn validate_staging_scene_indexes(
    staged_line_indexes_by_scene: &BTreeMap<i32, BTreeSet<i32>>,
    scene_indexes: &BTreeSet<i32>,
) -> Result<(), AppError> {
    for scene_index in staged_line_indexes_by_scene.keys() {
        if !scene_indexes.contains(scene_index) {
            return Err(invalid(format!(
                "Galgame character_staging 引用不存在的 scene_index: {scene_index}"
            )));
        }
    }
    Ok(())
}

type VisualPlanMappings<'a> = (
    BTreeMap<i32, &'a GalgamePipelineBackgroundPlan>,
    BTreeMap<(i32, i32), &'a GalgamePipelineEventCgPlan>,
);

fn visual_plan_mappings<'a>(
    visual_plan: &'a GalgameVisualPlan,
    scene_indexes: &BTreeSet<i32>,
    staged_line_indexes_by_scene: &BTreeMap<i32, BTreeSet<i32>>,
    source_span_by_id: &BTreeMap<&str, &GalgamePipelineSourceSpan>,
) -> Result<VisualPlanMappings<'a>, AppError> {
    let mut background_by_scene = BTreeMap::new();
    for background in &visual_plan.backgrounds {
        validate_visual_source_spans(
            &background.source_span_ids,
            source_span_by_id,
            "Galgame visual_planning background",
        )?;
        if !scene_indexes.contains(&background.scene_index) {
            return Err(invalid(format!(
                "Galgame visual_planning background 引用不存在的 scene_index: {}",
                background.scene_index
            )));
        }
        if background_by_scene
            .insert(background.scene_index, background)
            .is_some()
        {
            return Err(invalid(format!(
                "Galgame visual_planning 同一场只能有一个 background: scene_index={}",
                background.scene_index
            )));
        }
    }
    for scene_index in scene_indexes {
        if !background_by_scene.contains_key(scene_index) {
            return Err(invalid(format!(
                "Galgame visual_planning 缺少 scene_index={} 的 background",
                scene_index
            )));
        }
    }

    let mut cg_by_scene_line = BTreeMap::new();
    for cg in &visual_plan.event_cgs {
        validate_visual_source_spans(
            &cg.source_span_ids,
            source_span_by_id,
            "Galgame visual_planning event_cg",
        )?;
        let Some(line_indexes) = staged_line_indexes_by_scene.get(&cg.scene_index) else {
            return Err(invalid(format!(
                "Galgame visual_planning event_cg 引用不存在的 scene_index: {}",
                cg.scene_index
            )));
        };
        if !line_indexes.contains(&cg.line_index) {
            return Err(invalid(format!(
                "Galgame visual_planning event_cg line_index 不存在: scene_index={}, line_index={}",
                cg.scene_index, cg.line_index
            )));
        }
        if cg_by_scene_line
            .insert((cg.scene_index, cg.line_index), cg)
            .is_some()
        {
            return Err(invalid(format!(
                "Galgame visual_planning 同一行只能有一个 event_cg: scene_index={}, line_index={}",
                cg.scene_index, cg.line_index
            )));
        }
    }
    Ok((background_by_scene, cg_by_scene_line))
}

fn validate_visual_source_spans(
    source_span_ids: &[String],
    source_span_by_id: &BTreeMap<&str, &GalgamePipelineSourceSpan>,
    label: &str,
) -> Result<(), AppError> {
    for source_span_id in source_span_ids {
        if !source_span_by_id.contains_key(source_span_id.as_str()) {
            return Err(invalid(format!(
                "{label} source_span_id 不存在: {source_span_id}"
            )));
        }
    }
    Ok(())
}

fn branch_choices_by_scene_line<'a>(
    branch_design: &'a GalgameBranchDesign,
    scene_indexes: &BTreeSet<i32>,
    staged_line_indexes_by_scene: &BTreeMap<i32, BTreeSet<i32>>,
) -> Result<BTreeMap<(i32, i32), &'a GalgamePipelineChoicePlan>, AppError> {
    let mut result = BTreeMap::new();
    for choice in &branch_design.choices {
        if !scene_indexes.contains(&choice.scene_index) {
            return Err(invalid(format!(
                "Galgame branch_design choice 引用不存在的 scene_index: {}",
                choice.scene_index
            )));
        }
        let Some(staged_line_indexes) = staged_line_indexes_by_scene.get(&choice.scene_index)
        else {
            return Err(invalid(format!(
                "Galgame branch_design choice 引用缺少 staged lines 的 scene_index: {}",
                choice.scene_index
            )));
        };
        if !staged_line_indexes.contains(&choice.after_line_index) {
            return Err(invalid(format!(
                "Galgame branch_design choice after_line_index 不存在: scene_index={}, after_line_index={}",
                choice.scene_index, choice.after_line_index
            )));
        }
        for option in &choice.options {
            let target_scene_index = option.returns_to_scene_index.ok_or_else(|| {
                invalid(format!(
                    "Galgame branch_design 选项必须提供回收敛主线场，choice scene_index={} after_line_index={} 缺少 returns_to_scene_index",
                    choice.scene_index, choice.after_line_index
                ))
            })?;
            if !scene_indexes.contains(&target_scene_index) {
                return Err(invalid(format!(
                    "Galgame branch_design option returns_to_scene_index 不存在: {target_scene_index}"
                )));
            }
        }
        if result
            .insert((choice.scene_index, choice.after_line_index), choice)
            .is_some()
        {
            return Err(invalid(format!(
                "Galgame branch_design 同一行只能有一个 choice: scene_index={}, after_line_index={}",
                choice.scene_index, choice.after_line_index
            )));
        }
    }
    Ok(result)
}

fn choice_line_and_branch_scenes(
    choice: &GalgamePipelineChoicePlan,
    line_index: usize,
    next_branch_scene_index: &mut i32,
    context: GalgameBranchSceneContext,
) -> Result<(GalgamePersistLine, Vec<GalgamePersistScene>), AppError> {
    let GalgameBranchSceneContext {
        source_snapshot_chapter_id,
        location,
        time_of_day,
        participants_json,
        source_scene_summary,
    } = context;
    let mut branch_scenes = Vec::with_capacity(choice.options.len());
    let choices = choice
        .options
        .iter()
        .enumerate()
        .map(|(index, option)| {
            let branch_scene_index = *next_branch_scene_index;
            *next_branch_scene_index += 1;
            let return_scene_index = option
                .returns_to_scene_index
                .ok_or_else(|| invalid("Galgame branch option 缺少 returns_to_scene_index"))?;
            branch_scenes.push(GalgamePersistScene {
                scene_index: branch_scene_index,
                source_snapshot_chapter_id,
                title: option.branch_title.clone(),
                location: location.clone(),
                time_of_day: time_of_day.clone(),
                participants_json: participants_json.clone(),
                background_note: source_scene_summary.clone(),
                entry_kind: SCENE_ENTRY_BRANCH.to_string(),
                next_scene_index: Some(return_scene_index),
                lines: vec![GalgamePersistLine {
                    line_index: 1,
                    line_type: LINE_TYPE_NARRATION.to_string(),
                    speaker_profile_id: None,
                    text: option.branch_summary.clone(),
                    expression: None,
                    pose: None,
                    position: None,
                    presentation_json: json!({
                        "branch": true,
                        "branch_option_text": option.option_text,
                        "returns_to_scene_index": return_scene_index,
                    }),
                    text_anchor_id: None,
                    choices: Vec::new(),
                }],
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
            presentation_json: json!({
                "branch": true,
            }),
            text_anchor_id: None,
            choices,
        },
        branch_scenes,
    ))
}

fn source_snapshot_chapter_id_for_scene(
    source_span_ids: &[String],
    source_span_by_id: &BTreeMap<&str, &GalgamePipelineSourceSpan>,
) -> Result<i64, AppError> {
    let Some(first_span_id) = source_span_ids.first() else {
        return Err(invalid("Galgame persist 场缺少 source_span_ids"));
    };
    let first_span = source_span_by_id
        .get(first_span_id.as_str())
        .ok_or_else(|| {
            invalid(format!(
                "Galgame persist source_span_id 不存在: {first_span_id}"
            ))
        })?;
    for source_span_id in source_span_ids {
        let span = source_span_by_id
            .get(source_span_id.as_str())
            .ok_or_else(|| {
                invalid(format!(
                    "Galgame persist source_span_id 不存在: {source_span_id}"
                ))
            })?;
        if span.source_snapshot_chapter_id != first_span.source_snapshot_chapter_id {
            return Err(invalid(format!(
                "Galgame persist 单场不能跨 source_snapshot_chapter: {first_span_id} 与 {source_span_id}"
            )));
        }
    }
    Ok(first_span.source_snapshot_chapter_id)
}

fn text_anchor_id_for_line(
    line: &GalgamePipelineLineStaged,
    source_span_by_id: &BTreeMap<&str, &GalgamePipelineSourceSpan>,
    source_snapshot_chapter_id: i64,
) -> Result<Option<i64>, AppError> {
    let Some(first_span_id) = line.source_span_ids.first() else {
        return Err(invalid(format!(
            "Galgame persist 行缺少 source_span_ids: line_index={}",
            line.index
        )));
    };
    let span = source_span_by_id
        .get(first_span_id.as_str())
        .ok_or_else(|| {
            invalid(format!(
                "Galgame persist source_span_id 不存在: {first_span_id}"
            ))
        })?;
    for source_span_id in &line.source_span_ids {
        let span = source_span_by_id
            .get(source_span_id.as_str())
            .ok_or_else(|| {
                invalid(format!(
                    "Galgame persist source_span_id 不存在: {source_span_id}"
                ))
            })?;
        if span.source_snapshot_chapter_id != source_snapshot_chapter_id {
            return Err(invalid(format!(
                "Galgame persist 行 source_span_id 不属于当前场 source_snapshot_chapter: line_index={}, source_span_id={}, expected_chapter_id={}, actual_chapter_id={}",
                line.index,
                source_span_id,
                source_snapshot_chapter_id,
                span.source_snapshot_chapter_id
            )));
        }
    }
    Ok(Some(span.text_anchor_id))
}

fn validate_persist_line(line: &GalgamePipelineLineStaged) -> Result<(), AppError> {
    if let Some(expression) = line.expression.as_deref()
        && !GALGAME_EXPRESSIONS.contains(&expression)
    {
        return Err(invalid(format!(
            "Galgame persist expression 无效: {expression}"
        )));
    }
    match line.line_type.as_str() {
        LINE_TYPE_DIALOGUE | LINE_TYPE_MONOLOGUE if line.speaker_profile_id.is_none() => {
            Err(invalid(format!(
                "Galgame persist {} 行必须绑定 speaker_profile_id",
                line.line_type
            )))
        }
        LINE_TYPE_DIALOGUE | LINE_TYPE_MONOLOGUE if line.expression.is_none() => {
            Err(invalid(format!(
                "Galgame persist {} 行绑定角色时必须显式提供 expression",
                line.line_type
            )))
        }
        LINE_TYPE_DIALOGUE | LINE_TYPE_MONOLOGUE => Ok(()),
        LINE_TYPE_NARRATION | LINE_TYPE_STAGE_DIRECTION | LINE_TYPE_CHOICE
            if line.speaker_profile_id.is_some() =>
        {
            Err(invalid(format!(
                "Galgame persist {} 行不能绑定 speaker_profile_id",
                line.line_type
            )))
        }
        LINE_TYPE_NARRATION | LINE_TYPE_STAGE_DIRECTION | LINE_TYPE_CHOICE => Ok(()),
        other => Err(invalid(format!("Galgame persist 行类型无效: {other}"))),
    }
}

fn to_i32(value: usize, label: &str) -> Result<i32, AppError> {
    i32::try_from(value)
        .map_err(|_| invalid(format!("{label} 超出 i32 范围: {value}")))
        .and_then(|value| {
            if value > 0 {
                Ok(value)
            } else {
                Err(invalid(format!("{label} 必须大于 0")))
            }
        })
}

fn presentation_json_for_line(
    line: &GalgamePipelineLineStaged,
    cg: Option<&&super::GalgamePipelineEventCgPlan>,
) -> Value {
    let mut value = json!({});
    if let Some(cg) = cg {
        value = json!({
            "cg": true,
            "cg_note": cg.cg_note,
            "cg_importance": cg.importance,
        });
    }
    if line.line_type == LINE_TYPE_STAGE_DIRECTION && line.text.contains("转场") {
        value["transition"] = json!("cut");
    }
    value
}

fn participants_json(participants: BTreeMap<i64, String>) -> Value {
    let mut seen = BTreeSet::new();
    let items = participants
        .into_iter()
        .filter_map(|(profile_id, name)| {
            if !seen.insert(profile_id) {
                return None;
            }
            Some(json!({
                "character_profile_id": profile_id,
                "name": name,
            }))
        })
        .collect::<Vec<_>>();
    json!(items)
}

fn invalid(message: impl Into<String>) -> AppError {
    AppError::InvalidArgument(message.into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::galgame::{
        GalgamePipelineSceneLineStaged, branch_design_from_value, character_staging_from_value,
        scene_segmentation_from_value, visual_plan_from_value,
    };
    use serde_json::json;

    #[test]
    fn builds_persist_plan_from_staged_pipeline_outputs() {
        let scenes = scene_segmentation_from_value(
            json!({
                "scenes": [{
                    "index": 1,
                    "title": "雨夜重逢",
                    "location": "车站",
                    "time_of_day": "夜晚",
                    "participants": ["林晓雨"],
                    "summary": "两人重逢",
                    "source_span_ids": ["s1"]
                }]
            }),
            "galgame_scene_segmentation",
        )
        .expect("scenes");
        let staging = character_staging_from_value(
            json!({
                "scene_lines": [{
                    "scene_index": 1,
                    "lines": [
                        {
                            "index": 1,
                            "line_type": "dialogue",
                            "speaker_profile_id": 7,
                            "speaker_name": "林晓雨",
                            "text": "你终于来了。",
                            "expression": "smile",
                            "pose": "standing",
                            "position": "left",
                            "source_span_ids": ["s1"]
                        },
                        {
                            "index": 2,
                            "line_type": "narration",
                            "speaker_name": "",
                            "text": "雨声压过脚步。",
                            "expression": "",
                            "pose": "",
                            "position": "",
                            "source_span_ids": ["s2"]
                        }
                    ]
                }]
            }),
            "galgame_character_staging",
        )
        .expect("staging");
        let visual_plan = visual_plan_from_value(
            json!({
                "backgrounds": [{
                    "scene_index": 1,
                    "background_note": "雨夜车站，冷色灯光",
                    "scene_profile_id": null,
                    "source_span_ids": ["s1"]
                }],
                "event_cgs": [{
                    "scene_index": 1,
                    "line_index": 1,
                    "cg_note": "少女在雨中回头",
                    "importance": "high",
                    "source_span_ids": ["s1"]
                }]
            }),
            "galgame_visual_planning",
        )
        .expect("visual");
        let source_spans = vec![
            GalgamePipelineSourceSpan {
                id: "s1".to_string(),
                text_anchor_id: 100,
                source_snapshot_chapter_id: 20,
            },
            GalgamePipelineSourceSpan {
                id: "s2".to_string(),
                text_anchor_id: 101,
                source_snapshot_chapter_id: 20,
            },
        ];

        let plan =
            build_galgame_script_persist_plan(&scenes, &staging, &visual_plan, None, &source_spans)
                .expect("persist plan");

        assert_eq!(plan.scenes.len(), 1);
        assert_eq!(plan.scenes[0].source_snapshot_chapter_id, 20);
        assert_eq!(
            plan.scenes[0].background_note.as_deref(),
            Some("雨夜车站，冷色灯光")
        );
        assert_eq!(plan.scenes[0].lines[0].text_anchor_id, Some(100));
        assert_eq!(plan.scenes[0].lines[0].presentation_json["cg"], json!(true));
        assert_eq!(
            plan.scenes[0].participants_json,
            json!([{"character_profile_id": 7, "name": "林晓雨"}])
        );
    }

    #[test]
    fn rejects_persist_plan_when_scene_source_span_is_unknown() {
        let scenes = scene_segmentation_from_value(
            json!({
                "scenes": [{
                    "index": 1,
                    "title": "雨夜重逢",
                    "source_span_ids": ["missing"]
                }]
            }),
            "galgame_scene_segmentation",
        )
        .expect("scenes");
        let staging = character_staging_from_value(
            json!({
                "scene_lines": [{
                    "scene_index": 1,
                    "lines": [{
                        "index": 1,
                        "line_type": "narration",
                        "text": "雨声压过脚步。",
                        "source_span_ids": ["missing"]
                    }]
                }]
            }),
            "galgame_character_staging",
        )
        .expect("staging");

        let err = build_galgame_script_persist_plan(
            &scenes,
            &staging,
            &visual_plan_for_scene(1, "missing"),
            None,
            &[],
        )
        .expect_err("unknown source span");

        assert!(err.to_string().contains("source_span_id 不存在"));
    }

    #[test]
    fn rejects_persist_plan_when_staged_line_has_no_source_span() {
        let scenes = single_scene_segmentation();
        let staging = GalgameCharacterStaging {
            scene_lines: vec![GalgamePipelineSceneLineStaged {
                scene_index: 1,
                lines: vec![GalgamePipelineLineStaged {
                    index: 1,
                    line_type: LINE_TYPE_NARRATION.to_string(),
                    speaker_profile_id: None,
                    speaker_name: None,
                    text: "雨声压过脚步。".to_string(),
                    expression: None,
                    pose: None,
                    position: None,
                    source_span_ids: Vec::new(),
                }],
            }],
        };
        let source_spans = single_source_span();

        let err = build_galgame_script_persist_plan(
            &scenes,
            &staging,
            &single_visual_plan(),
            None,
            &source_spans,
        )
        .expect_err("external staged lines must keep source anchor evidence");

        assert!(err.to_string().contains("行缺少 source_span_ids"));
        assert!(err.to_string().contains("line_index=1"));
    }

    #[test]
    fn rejects_persist_plan_when_secondary_line_source_span_is_unknown() {
        let scenes = single_scene_segmentation();
        let staging = GalgameCharacterStaging {
            scene_lines: vec![GalgamePipelineSceneLineStaged {
                scene_index: 1,
                lines: vec![GalgamePipelineLineStaged {
                    index: 1,
                    line_type: LINE_TYPE_NARRATION.to_string(),
                    speaker_profile_id: None,
                    speaker_name: None,
                    text: "雨声压过脚步。".to_string(),
                    expression: None,
                    pose: None,
                    position: None,
                    source_span_ids: vec!["s1".to_string(), "missing".to_string()],
                }],
            }],
        };
        let source_spans = single_source_span();

        let err = build_galgame_script_persist_plan(
            &scenes,
            &staging,
            &single_visual_plan(),
            None,
            &source_spans,
        )
        .expect_err("all line source_span_ids must be validated");

        assert!(err.to_string().contains("source_span_id 不存在: missing"));
    }

    #[test]
    fn rejects_persist_plan_when_line_source_span_crosses_scene_chapter() {
        let scenes = single_scene_segmentation();
        let staging = GalgameCharacterStaging {
            scene_lines: vec![GalgamePipelineSceneLineStaged {
                scene_index: 1,
                lines: vec![GalgamePipelineLineStaged {
                    index: 1,
                    line_type: LINE_TYPE_NARRATION.to_string(),
                    speaker_profile_id: None,
                    speaker_name: None,
                    text: "雨声压过脚步。".to_string(),
                    expression: None,
                    pose: None,
                    position: None,
                    source_span_ids: vec!["s1".to_string(), "s2".to_string()],
                }],
            }],
        };
        let source_spans = vec![
            GalgamePipelineSourceSpan {
                id: "s1".to_string(),
                text_anchor_id: 100,
                source_snapshot_chapter_id: 20,
            },
            GalgamePipelineSourceSpan {
                id: "s2".to_string(),
                text_anchor_id: 101,
                source_snapshot_chapter_id: 21,
            },
        ];

        let err = build_galgame_script_persist_plan(
            &scenes,
            &staging,
            &single_visual_plan(),
            None,
            &source_spans,
        )
        .expect_err("line source spans must stay inside the scene snapshot chapter");

        assert!(
            err.to_string()
                .contains("行 source_span_id 不属于当前场 source_snapshot_chapter")
        );
        assert!(err.to_string().contains("source_span_id=s2"));
    }

    #[test]
    fn rejects_persist_plan_when_staging_references_unknown_scene() {
        let scenes = single_scene_segmentation();
        let staging = character_staging_from_value(
            json!({
                "scene_lines": [
                    {
                        "scene_index": 1,
                        "lines": [{
                            "index": 1,
                            "line_type": "narration",
                            "text": "雨声压过脚步。",
                            "source_span_ids": ["s1"]
                        }]
                    },
                    {
                        "scene_index": 2,
                        "lines": [{
                            "index": 1,
                            "line_type": "narration",
                            "text": "不存在的场。",
                            "source_span_ids": ["s1"]
                        }]
                    }
                ]
            }),
            "galgame_character_staging",
        )
        .expect("staging");
        let source_spans = single_source_span();

        let err = build_galgame_script_persist_plan(
            &scenes,
            &staging,
            &single_visual_plan(),
            None,
            &source_spans,
        )
        .expect_err("extra staged scene must fail visibly");

        assert!(
            err.to_string()
                .contains("character_staging 引用不存在的 scene_index")
        );
    }

    #[test]
    fn rejects_visual_plan_missing_scene_background() {
        let scenes = scene_segmentation_from_value(
            json!({
                "scenes": [
                    {
                        "index": 1,
                        "title": "雨夜重逢",
                        "source_span_ids": ["s1"]
                    },
                    {
                        "index": 2,
                        "title": "站台尽头",
                        "source_span_ids": ["s2"]
                    }
                ]
            }),
            "galgame_scene_segmentation",
        )
        .expect("scenes");
        let staging = character_staging_from_value(
            json!({
                "scene_lines": [
                    {
                        "scene_index": 1,
                        "lines": [{
                            "index": 1,
                            "line_type": "narration",
                            "text": "雨声压过脚步。",
                            "source_span_ids": ["s1"]
                        }]
                    },
                    {
                        "scene_index": 2,
                        "lines": [{
                            "index": 1,
                            "line_type": "narration",
                            "text": "站台尽头亮起灯。",
                            "source_span_ids": ["s2"]
                        }]
                    }
                ]
            }),
            "galgame_character_staging",
        )
        .expect("staging");
        let visual_plan = visual_plan_from_value(
            json!({
                "backgrounds": [{
                    "scene_index": 1,
                    "background_note": "雨夜车站",
                    "source_span_ids": ["s1"]
                }],
                "event_cgs": []
            }),
            "galgame_visual_planning",
        )
        .expect("visual plan");
        let source_spans = vec![
            GalgamePipelineSourceSpan {
                id: "s1".to_string(),
                text_anchor_id: 100,
                source_snapshot_chapter_id: 20,
            },
            GalgamePipelineSourceSpan {
                id: "s2".to_string(),
                text_anchor_id: 101,
                source_snapshot_chapter_id: 20,
            },
        ];

        let err =
            build_galgame_script_persist_plan(&scenes, &staging, &visual_plan, None, &source_spans)
                .expect_err("missing background must fail visibly");

        assert!(err.to_string().contains("缺少 scene_index=2 的 background"));
    }

    #[test]
    fn rejects_visual_plan_event_cg_for_missing_line() {
        let scenes = single_scene_segmentation();
        let staging = single_narration_staging();
        let visual_plan = visual_plan_from_value(
            json!({
                "backgrounds": [{
                    "scene_index": 1,
                    "background_note": "雨夜车站",
                    "source_span_ids": ["s1"]
                }],
                "event_cgs": [{
                    "scene_index": 1,
                    "line_index": 2,
                    "cg_note": "不存在的高潮行",
                    "importance": "high",
                    "source_span_ids": ["s1"]
                }]
            }),
            "galgame_visual_planning",
        )
        .expect("visual plan");
        let source_spans = single_source_span();

        let err =
            build_galgame_script_persist_plan(&scenes, &staging, &visual_plan, None, &source_spans)
                .expect_err("missing CG target line must fail visibly");

        assert!(err.to_string().contains("event_cg line_index 不存在"));
    }

    #[test]
    fn rejects_visual_plan_unknown_source_span() {
        let scenes = single_scene_segmentation();
        let staging = single_narration_staging();
        let visual_plan = visual_plan_from_value(
            json!({
                "backgrounds": [{
                    "scene_index": 1,
                    "background_note": "雨夜车站",
                    "source_span_ids": ["missing"]
                }],
                "event_cgs": []
            }),
            "galgame_visual_planning",
        )
        .expect("visual plan");
        let source_spans = single_source_span();

        let err =
            build_galgame_script_persist_plan(&scenes, &staging, &visual_plan, None, &source_spans)
                .expect_err("unknown visual source span must fail visibly");

        assert!(err.to_string().contains("source_span_id 不存在"));
    }

    #[test]
    fn rejects_persist_plan_when_dialogue_has_no_profile() {
        let scenes = scene_segmentation_from_value(
            json!({
                "scenes": [{
                    "index": 1,
                    "title": "雨夜重逢",
                    "source_span_ids": ["s1"]
                }]
            }),
            "galgame_scene_segmentation",
        )
        .expect("scenes");
        let staging = character_staging_from_value(
            json!({
                "scene_lines": [{
                    "scene_index": 1,
                    "lines": [{
                        "index": 1,
                        "line_type": "dialogue",
                        "speaker_name": "林晓雨",
                        "text": "你终于来了。",
                        "source_span_ids": ["s1"]
                    }]
                }]
            }),
            "galgame_character_staging",
        )
        .expect("staging");
        let source_spans = vec![GalgamePipelineSourceSpan {
            id: "s1".to_string(),
            text_anchor_id: 100,
            source_snapshot_chapter_id: 20,
        }];

        let err = build_galgame_script_persist_plan(
            &scenes,
            &staging,
            &single_visual_plan(),
            None,
            &source_spans,
        )
        .expect_err("missing profile");

        assert!(err.to_string().contains("必须绑定 speaker_profile_id"));
    }

    #[test]
    fn rejects_persist_plan_when_dialogue_has_no_expression() {
        let scenes = single_scene_segmentation();
        let staging = GalgameCharacterStaging {
            scene_lines: vec![GalgamePipelineSceneLineStaged {
                scene_index: 1,
                lines: vec![GalgamePipelineLineStaged {
                    index: 1,
                    line_type: LINE_TYPE_DIALOGUE.to_string(),
                    speaker_profile_id: Some(7),
                    speaker_name: Some("林晓雨".to_string()),
                    text: "你终于来了。".to_string(),
                    expression: None,
                    pose: None,
                    position: Some("left".to_string()),
                    source_span_ids: vec!["s1".to_string()],
                }],
            }],
        };
        let source_spans = single_source_span();

        let err = build_galgame_script_persist_plan(
            &scenes,
            &staging,
            &single_visual_plan(),
            None,
            &source_spans,
        )
        .expect_err("persist layer must require dialogue expression");

        assert!(err.to_string().contains("必须显式提供 expression"));
    }

    #[test]
    fn rejects_persist_plan_when_expression_is_invalid() {
        let scenes = single_scene_segmentation();
        let staging = GalgameCharacterStaging {
            scene_lines: vec![GalgamePipelineSceneLineStaged {
                scene_index: 1,
                lines: vec![GalgamePipelineLineStaged {
                    index: 1,
                    line_type: LINE_TYPE_DIALOGUE.to_string(),
                    speaker_profile_id: Some(7),
                    speaker_name: Some("林晓雨".to_string()),
                    text: "你终于来了。".to_string(),
                    expression: Some("wink".to_string()),
                    pose: None,
                    position: Some("left".to_string()),
                    source_span_ids: vec!["s1".to_string()],
                }],
            }],
        };
        let source_spans = single_source_span();

        let err = build_galgame_script_persist_plan(
            &scenes,
            &staging,
            &single_visual_plan(),
            None,
            &source_spans,
        )
        .expect_err("persist layer must reject invalid expression");

        assert!(err.to_string().contains("expression 无效: wink"));
    }

    #[test]
    fn rejects_branch_design_choice_after_missing_line() {
        let scenes = scene_segmentation_from_value(
            json!({
                "scenes": [
                    {
                        "index": 1,
                        "title": "分岔口",
                        "source_span_ids": ["s1"]
                    },
                    {
                        "index": 2,
                        "title": "回收点",
                        "source_span_ids": ["s2"]
                    }
                ]
            }),
            "galgame_scene_segmentation",
        )
        .expect("scenes");
        let staging = character_staging_from_value(
            json!({
                "scene_lines": [
                    {
                        "scene_index": 1,
                        "lines": [{
                            "index": 1,
                            "line_type": "narration",
                            "text": "路灯闪了一下。",
                            "source_span_ids": ["s1"]
                        }]
                    },
                    {
                        "scene_index": 2,
                        "lines": [{
                            "index": 1,
                            "line_type": "narration",
                            "text": "两人重新会合。",
                            "source_span_ids": ["s2"]
                        }]
                    }
                ]
            }),
            "galgame_character_staging",
        )
        .expect("staging");
        let branch = branch_design_from_value(
            json!({
                "choices": [{
                    "scene_index": 1,
                    "after_line_index": 99,
                    "question": "要走哪条路？",
                    "options": [
                        {
                            "option_text": "走左侧小巷",
                            "branch_title": "左侧小巷",
                            "branch_summary": "短暂调查后回到主线。",
                            "returns_to_scene_index": 2
                        },
                        {
                            "option_text": "留在原地",
                            "branch_title": "原地等待",
                            "branch_summary": "等待同伴后回到主线。",
                            "returns_to_scene_index": 2
                        }
                    ]
                }]
            }),
            "galgame_branch_design",
        )
        .expect("branch");
        let source_spans = vec![
            GalgamePipelineSourceSpan {
                id: "s1".to_string(),
                text_anchor_id: 100,
                source_snapshot_chapter_id: 20,
            },
            GalgamePipelineSourceSpan {
                id: "s2".to_string(),
                text_anchor_id: 101,
                source_snapshot_chapter_id: 21,
            },
        ];

        let err = build_galgame_script_persist_plan(
            &scenes,
            &staging,
            &two_scene_visual_plan(),
            Some(&branch),
            &source_spans,
        )
        .expect_err("branch choice after_line_index must reference an existing staged line");

        assert!(err.to_string().contains("after_line_index 不存在"));
    }

    #[test]
    fn inserts_choice_line_from_branch_design() {
        let scenes = scene_segmentation_from_value(
            json!({
                "scenes": [
                    {
                        "index": 1,
                        "title": "分岔口",
                        "source_span_ids": ["s1"]
                    },
                    {
                        "index": 2,
                        "title": "回收点",
                        "source_span_ids": ["s2"]
                    }
                ]
            }),
            "galgame_scene_segmentation",
        )
        .expect("scenes");
        let staging = character_staging_from_value(
            json!({
                "scene_lines": [
                    {
                        "scene_index": 1,
                        "lines": [{
                            "index": 1,
                            "line_type": "narration",
                            "text": "路灯闪了一下。",
                            "source_span_ids": ["s1"]
                        }]
                    },
                    {
                        "scene_index": 2,
                        "lines": [{
                            "index": 1,
                            "line_type": "narration",
                            "text": "两人重新会合。",
                            "source_span_ids": ["s2"]
                        }]
                    }
                ]
            }),
            "galgame_character_staging",
        )
        .expect("staging");
        let branch = branch_design_from_value(
            json!({
                "choices": [{
                    "scene_index": 1,
                    "after_line_index": 1,
                    "question": "要走哪条路？",
                    "options": [
                        {
                            "option_text": "走左侧小巷",
                            "branch_title": "左侧小巷",
                            "branch_summary": "短暂调查后回到主线。",
                            "returns_to_scene_index": 2
                        },
                        {
                            "option_text": "留在原地",
                            "branch_title": "原地等待",
                            "branch_summary": "等待同伴后回到主线。",
                            "returns_to_scene_index": 2
                        }
                    ]
                }]
            }),
            "galgame_branch_design",
        )
        .expect("branch");
        let source_spans = vec![
            GalgamePipelineSourceSpan {
                id: "s1".to_string(),
                text_anchor_id: 100,
                source_snapshot_chapter_id: 20,
            },
            GalgamePipelineSourceSpan {
                id: "s2".to_string(),
                text_anchor_id: 101,
                source_snapshot_chapter_id: 21,
            },
        ];

        let plan = build_galgame_script_persist_plan(
            &scenes,
            &staging,
            &two_scene_visual_plan(),
            Some(&branch),
            &source_spans,
        )
        .expect("persist plan");

        assert_eq!(plan.scenes.len(), 4);
        assert_eq!(plan.scenes[0].next_scene_index, Some(2));
        assert_eq!(plan.scenes[1].next_scene_index, None);

        let choice_line = &plan.scenes[0].lines[1];
        assert_eq!(choice_line.line_index, 2);
        assert_eq!(choice_line.line_type, LINE_TYPE_CHOICE);
        assert_eq!(choice_line.text, "要走哪条路？");
        assert_eq!(choice_line.choices.len(), 2);
        assert_eq!(choice_line.choices[0].option_index, 1);
        assert_eq!(choice_line.choices[0].target_scene_index, 3);
        assert_eq!(choice_line.choices[1].target_scene_index, 4);

        assert_eq!(plan.scenes[2].scene_index, 3);
        assert_eq!(plan.scenes[2].entry_kind, SCENE_ENTRY_BRANCH);
        assert_eq!(plan.scenes[2].title, "左侧小巷");
        assert_eq!(plan.scenes[2].next_scene_index, Some(2));
        assert_eq!(plan.scenes[2].lines[0].text, "短暂调查后回到主线。");
    }

    fn single_scene_segmentation() -> GalgameSceneSegmentation {
        scene_segmentation_from_value(
            json!({
                "scenes": [{
                    "index": 1,
                    "title": "雨夜重逢",
                    "source_span_ids": ["s1"]
                }]
            }),
            "galgame_scene_segmentation",
        )
        .expect("single scene segmentation")
    }

    fn single_source_span() -> Vec<GalgamePipelineSourceSpan> {
        vec![GalgamePipelineSourceSpan {
            id: "s1".to_string(),
            text_anchor_id: 100,
            source_snapshot_chapter_id: 20,
        }]
    }

    fn single_narration_staging() -> GalgameCharacterStaging {
        character_staging_from_value(
            json!({
                "scene_lines": [{
                    "scene_index": 1,
                    "lines": [{
                        "index": 1,
                        "line_type": "narration",
                        "text": "雨声压过脚步。",
                        "source_span_ids": ["s1"]
                    }]
                }]
            }),
            "galgame_character_staging",
        )
        .expect("single narration staging")
    }

    fn single_visual_plan() -> GalgameVisualPlan {
        visual_plan_for_scene(1, "s1")
    }

    fn visual_plan_for_scene(scene_index: i32, source_span_id: &str) -> GalgameVisualPlan {
        visual_plan_from_value(
            json!({
                "backgrounds": [{
                    "scene_index": scene_index,
                    "background_note": format!("场 {scene_index} 背景"),
                    "source_span_ids": [source_span_id]
                }],
                "event_cgs": []
            }),
            "galgame_visual_planning",
        )
        .expect("visual plan")
    }

    fn two_scene_visual_plan() -> GalgameVisualPlan {
        visual_plan_from_value(
            json!({
                "backgrounds": [
                    {
                        "scene_index": 1,
                        "background_note": "分岔口背景",
                        "source_span_ids": ["s1"]
                    },
                    {
                        "scene_index": 2,
                        "background_note": "回收点背景",
                        "source_span_ids": ["s2"]
                    }
                ],
                "event_cgs": []
            }),
            "galgame_visual_planning",
        )
        .expect("two scene visual plan")
    }
}
