use std::collections::BTreeSet;

use a4n_core::error::AppError;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::{
    GALGAME_EXPRESSIONS, LINE_TYPE_DIALOGUE, LINE_TYPE_MONOLOGUE, LINE_TYPE_NARRATION,
    LINE_TYPE_STAGE_DIRECTION,
};

const SCRIPT_LINE_TYPES: &[&str] = &[
    LINE_TYPE_DIALOGUE,
    LINE_TYPE_NARRATION,
    LINE_TYPE_MONOLOGUE,
    LINE_TYPE_STAGE_DIRECTION,
];
const POSITIONS: &[&str] = &["left", "center", "right"];
const CG_IMPORTANCE: &[&str] = &["normal", "high", "critical"];

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgameSceneSegmentation {
    pub scenes: Vec<GalgamePipelineScene>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgamePipelineScene {
    pub index: i32,
    pub title: String,
    pub location: Option<String>,
    pub time_of_day: Option<String>,
    pub participants: Vec<String>,
    pub summary: Option<String>,
    pub source_span_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgameScriptAdaptation {
    pub scene_lines: Vec<GalgamePipelineSceneLineDraft>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgamePipelineSceneLineDraft {
    pub scene_index: i32,
    pub lines: Vec<GalgamePipelineLineDraft>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgamePipelineLineDraft {
    pub index: i32,
    pub line_type: String,
    pub speaker_name: Option<String>,
    pub text: String,
    pub source_span_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgameCharacterStaging {
    pub scene_lines: Vec<GalgamePipelineSceneLineStaged>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgamePipelineSceneLineStaged {
    pub scene_index: i32,
    pub lines: Vec<GalgamePipelineLineStaged>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgamePipelineLineStaged {
    pub index: i32,
    pub line_type: String,
    pub speaker_profile_id: Option<i64>,
    pub speaker_name: Option<String>,
    pub text: String,
    pub expression: Option<String>,
    pub pose: Option<String>,
    pub position: Option<String>,
    pub source_span_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgameVisualPlan {
    pub backgrounds: Vec<GalgamePipelineBackgroundPlan>,
    pub event_cgs: Vec<GalgamePipelineEventCgPlan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgamePipelineBackgroundPlan {
    pub scene_index: i32,
    pub background_note: String,
    pub scene_profile_id: Option<i64>,
    pub source_span_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgamePipelineEventCgPlan {
    pub scene_index: i32,
    pub line_index: i32,
    pub cg_note: String,
    pub importance: String,
    pub source_span_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgameBranchDesign {
    pub choices: Vec<GalgamePipelineChoicePlan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgamePipelineChoicePlan {
    pub scene_index: i32,
    pub after_line_index: i32,
    pub question: String,
    pub options: Vec<GalgamePipelineChoiceOptionPlan>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct GalgamePipelineChoiceOptionPlan {
    pub option_text: String,
    pub branch_title: String,
    pub branch_summary: String,
    pub returns_to_scene_index: Option<i32>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SceneSegmentationRaw {
    scenes: Vec<SceneRaw>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SceneRaw {
    index: i32,
    title: String,
    #[serde(default)]
    location: Option<String>,
    #[serde(default)]
    time_of_day: Option<String>,
    #[serde(default)]
    participants: Vec<String>,
    #[serde(default)]
    summary: Option<String>,
    source_span_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ScriptAdaptationRaw {
    scene_lines: Vec<SceneLineDraftRaw>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SceneLineDraftRaw {
    scene_index: i32,
    lines: Vec<LineDraftRaw>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LineDraftRaw {
    index: i32,
    line_type: String,
    #[serde(default)]
    speaker_name: Option<String>,
    text: String,
    #[serde(default)]
    source_span_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CharacterStagingRaw {
    scene_lines: Vec<SceneLineStagedRaw>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct SceneLineStagedRaw {
    scene_index: i32,
    lines: Vec<LineStagedRaw>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct LineStagedRaw {
    index: i32,
    line_type: String,
    #[serde(default)]
    speaker_profile_id: Option<i64>,
    #[serde(default)]
    speaker_name: Option<String>,
    text: String,
    #[serde(default)]
    expression: Option<String>,
    #[serde(default)]
    pose: Option<String>,
    #[serde(default)]
    position: Option<String>,
    #[serde(default)]
    source_span_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct VisualPlanRaw {
    backgrounds: Vec<BackgroundPlanRaw>,
    #[serde(default)]
    event_cgs: Vec<EventCgPlanRaw>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BackgroundPlanRaw {
    scene_index: i32,
    background_note: String,
    #[serde(default)]
    scene_profile_id: Option<i64>,
    source_span_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct EventCgPlanRaw {
    scene_index: i32,
    line_index: i32,
    cg_note: String,
    importance: String,
    source_span_ids: Vec<String>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct BranchDesignRaw {
    #[serde(default)]
    choices: Vec<ChoicePlanRaw>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ChoicePlanRaw {
    scene_index: i32,
    after_line_index: i32,
    question: String,
    options: Vec<ChoiceOptionPlanRaw>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ChoiceOptionPlanRaw {
    option_text: String,
    branch_title: String,
    branch_summary: String,
    #[serde(default)]
    returns_to_scene_index: Option<i32>,
}

pub fn parse_galgame_json_object(raw: &str, context: &str) -> Result<Value, AppError> {
    let raw = raw.trim();
    if raw.is_empty() {
        return Err(AppError::InvalidArgument(format!("{context} 返回空内容")));
    }
    if raw.starts_with("```") {
        return Err(AppError::InvalidArgument(format!(
            "{context} 不能用 Markdown 代码围栏包裹 JSON"
        )));
    }
    let value: Value = serde_json::from_str(raw).map_err(|err| {
        let reason = if err.is_eof() {
            "不完整 JSON"
        } else {
            "JSON 解析失败"
        };
        AppError::InvalidArgument(format!("{context} {reason}: {err}"))
    })?;
    require_object_value(value, context)
}

pub fn scene_segmentation_from_value(
    value: Value,
    context: &str,
) -> Result<GalgameSceneSegmentation, AppError> {
    let raw: SceneSegmentationRaw = from_value(value, context)?;
    require_nonempty(raw.scenes.as_slice(), &format!("{context}.scenes"))?;
    let mut seen = BTreeSet::new();
    let mut scenes = Vec::with_capacity(raw.scenes.len());
    for scene in raw.scenes {
        require_positive(scene.index, &format!("{context}.scenes[].index"))?;
        if !seen.insert(scene.index) {
            return Err(invalid(format!(
                "{context}.scenes.index 重复: {}",
                scene.index
            )));
        }
        scenes.push(GalgamePipelineScene {
            index: scene.index,
            title: required_text(scene.title, &format!("{context}.scenes[].title"))?,
            location: optional_text(scene.location),
            time_of_day: optional_text(scene.time_of_day),
            participants: normalize_text_vec(
                scene.participants,
                &format!("{context}.scenes[].participants"),
            )?,
            summary: optional_text(scene.summary),
            source_span_ids: normalize_source_span_ids(
                scene.source_span_ids,
                &format!("{context}.scenes[].source_span_ids"),
                true,
            )?,
        });
    }
    ensure_contiguous_indexes(
        scenes.iter().map(|scene| scene.index),
        &format!("{context}.scenes"),
    )?;
    Ok(GalgameSceneSegmentation { scenes })
}

pub fn script_adaptation_from_value(
    value: Value,
    context: &str,
) -> Result<GalgameScriptAdaptation, AppError> {
    let raw: ScriptAdaptationRaw = from_value(value, context)?;
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
                .map(|line| line_draft_from_raw(line, context))
                .collect::<Result<Vec<_>, _>>()?;
            ensure_contiguous_indexes(
                lines.iter().map(|line| line.index),
                &format!("{context}.scene_lines[{}].lines", scene.scene_index),
            )?;
            Ok(GalgamePipelineSceneLineDraft {
                scene_index: scene.scene_index,
                lines,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    ensure_unique_scene_indexes(scene_lines.iter().map(|item| item.scene_index), context)?;
    Ok(GalgameScriptAdaptation { scene_lines })
}

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
    Ok(GalgameCharacterStaging { scene_lines })
}

pub fn visual_plan_from_value(value: Value, context: &str) -> Result<GalgameVisualPlan, AppError> {
    let raw: VisualPlanRaw = from_value(value, context)?;
    require_nonempty(
        raw.backgrounds.as_slice(),
        &format!("{context}.backgrounds"),
    )?;
    let backgrounds = raw
        .backgrounds
        .into_iter()
        .map(|background| {
            require_positive(
                background.scene_index,
                &format!("{context}.backgrounds[].scene_index"),
            )?;
            if let Some(scene_profile_id) = background.scene_profile_id {
                require_positive_i64(
                    scene_profile_id,
                    &format!("{context}.backgrounds[].scene_profile_id"),
                )?;
            }
            Ok(GalgamePipelineBackgroundPlan {
                scene_index: background.scene_index,
                background_note: required_text(
                    background.background_note,
                    &format!("{context}.backgrounds[].background_note"),
                )?,
                scene_profile_id: background.scene_profile_id,
                source_span_ids: normalize_source_span_ids(
                    background.source_span_ids,
                    &format!("{context}.backgrounds[].source_span_ids"),
                    true,
                )?,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    ensure_unique_indexes(
        backgrounds.iter().map(|background| background.scene_index),
        &format!("{context}.backgrounds.scene_index"),
    )?;
    let event_cgs = raw
        .event_cgs
        .into_iter()
        .map(|cg| {
            require_positive(
                cg.scene_index,
                &format!("{context}.event_cgs[].scene_index"),
            )?;
            require_positive(cg.line_index, &format!("{context}.event_cgs[].line_index"))?;
            let importance =
                required_text(cg.importance, &format!("{context}.event_cgs[].importance"))?;
            if !CG_IMPORTANCE.contains(&importance.as_str()) {
                return Err(invalid(format!(
                    "{context}.event_cgs[].importance 无效: {importance}"
                )));
            }
            Ok(GalgamePipelineEventCgPlan {
                scene_index: cg.scene_index,
                line_index: cg.line_index,
                cg_note: required_text(cg.cg_note, &format!("{context}.event_cgs[].cg_note"))?,
                importance,
                source_span_ids: normalize_source_span_ids(
                    cg.source_span_ids,
                    &format!("{context}.event_cgs[].source_span_ids"),
                    true,
                )?,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(GalgameVisualPlan {
        backgrounds,
        event_cgs,
    })
}

pub fn branch_design_from_value(
    value: Value,
    context: &str,
) -> Result<GalgameBranchDesign, AppError> {
    let raw: BranchDesignRaw = from_value(value, context)?;
    let choices = raw
        .choices
        .into_iter()
        .map(|choice| {
            require_positive(
                choice.scene_index,
                &format!("{context}.choices[].scene_index"),
            )?;
            require_positive(
                choice.after_line_index,
                &format!("{context}.choices[].after_line_index"),
            )?;
            if !(2..=3).contains(&choice.options.len()) {
                return Err(invalid(format!(
                    "{context}.choices[].options 必须包含 2 到 3 个选项"
                )));
            }
            let options = choice
                .options
                .into_iter()
                .map(|option| {
                    let returns_to_scene_index =
                        option.returns_to_scene_index.ok_or_else(|| {
                            invalid(format!(
                                "{context}.choices[].options[].returns_to_scene_index 不能为空"
                            ))
                        })?;
                    require_positive(
                        returns_to_scene_index,
                        &format!("{context}.choices[].options[].returns_to_scene_index"),
                    )?;
                    Ok(GalgamePipelineChoiceOptionPlan {
                        option_text: required_text(
                            option.option_text,
                            &format!("{context}.choices[].options[].option_text"),
                        )?,
                        branch_title: required_text(
                            option.branch_title,
                            &format!("{context}.choices[].options[].branch_title"),
                        )?,
                        branch_summary: required_text(
                            option.branch_summary,
                            &format!("{context}.choices[].options[].branch_summary"),
                        )?,
                        returns_to_scene_index: Some(returns_to_scene_index),
                    })
                })
                .collect::<Result<Vec<_>, AppError>>()?;
            Ok(GalgamePipelineChoicePlan {
                scene_index: choice.scene_index,
                after_line_index: choice.after_line_index,
                question: required_text(choice.question, &format!("{context}.choices[].question"))?,
                options,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(GalgameBranchDesign { choices })
}

fn line_draft_from_raw(
    line: LineDraftRaw,
    context: &str,
) -> Result<GalgamePipelineLineDraft, AppError> {
    require_positive(line.index, &format!("{context}.lines[].index"))?;
    let line_type =
        normalize_script_line_type(line.line_type, &format!("{context}.lines[].line_type"))?;
    let speaker_name = optional_text(line.speaker_name);
    if matches!(line_type.as_str(), LINE_TYPE_DIALOGUE | LINE_TYPE_MONOLOGUE)
        && speaker_name.is_none()
    {
        return Err(invalid(format!("{context}.lines[].speaker_name 不能为空")));
    }
    Ok(GalgamePipelineLineDraft {
        index: line.index,
        line_type,
        speaker_name,
        text: required_text(line.text, &format!("{context}.lines[].text"))?,
        source_span_ids: normalize_source_span_ids(
            line.source_span_ids,
            &format!("{context}.lines[].source_span_ids"),
            true,
        )?,
    })
}

fn line_staged_from_raw(
    line: LineStagedRaw,
    context: &str,
) -> Result<GalgamePipelineLineStaged, AppError> {
    require_positive(line.index, &format!("{context}.lines[].index"))?;
    let line_type =
        normalize_script_line_type(line.line_type, &format!("{context}.lines[].line_type"))?;
    let speaker_name = optional_text(line.speaker_name);
    let speaker_profile_id = match line.speaker_profile_id {
        Some(id) if id <= 0 => {
            return Err(invalid(format!(
                "{context}.lines[].speaker_profile_id 必须是正整数"
            )));
        }
        other => other,
    };
    if matches!(line_type.as_str(), LINE_TYPE_DIALOGUE | LINE_TYPE_MONOLOGUE)
        && speaker_name.is_none()
        && speaker_profile_id.is_none()
    {
        return Err(invalid(format!(
            "{context}.lines[].speaker_name 或 speaker_profile_id 至少提供一个"
        )));
    }
    if !matches!(line_type.as_str(), LINE_TYPE_DIALOGUE | LINE_TYPE_MONOLOGUE)
        && speaker_profile_id.is_some()
    {
        return Err(invalid(format!(
            "{context}.lines[].speaker_profile_id 只能用于 dialogue/monologue"
        )));
    }
    let expression = optional_text(line.expression);
    if let Some(expression) = expression.as_deref()
        && !GALGAME_EXPRESSIONS.contains(&expression)
    {
        return Err(invalid(format!(
            "{context}.lines[].expression 无效: {expression}"
        )));
    }
    if matches!(line_type.as_str(), LINE_TYPE_DIALOGUE | LINE_TYPE_MONOLOGUE)
        && speaker_profile_id.is_some()
        && expression.is_none()
    {
        return Err(invalid(format!(
            "{context}.lines[].expression 绑定角色行必须显式提供；普通表情请使用 neutral"
        )));
    }
    let position = optional_text(line.position);
    if let Some(position) = position.as_deref()
        && !POSITIONS.contains(&position)
    {
        return Err(invalid(format!(
            "{context}.lines[].position 无效: {position}"
        )));
    }
    Ok(GalgamePipelineLineStaged {
        index: line.index,
        line_type,
        speaker_profile_id,
        speaker_name,
        text: required_text(line.text, &format!("{context}.lines[].text"))?,
        expression,
        pose: optional_text(line.pose),
        position,
        source_span_ids: normalize_source_span_ids(
            line.source_span_ids,
            &format!("{context}.lines[].source_span_ids"),
            true,
        )?,
    })
}

fn require_object_value(value: Value, context: &str) -> Result<Value, AppError> {
    if value.as_object().is_some() {
        Ok(value)
    } else {
        Err(invalid(format!("{context} 不是 JSON 对象")))
    }
}

fn from_value<T>(value: Value, context: &str) -> Result<T, AppError>
where
    T: for<'de> Deserialize<'de>,
{
    let object = value
        .as_object()
        .ok_or_else(|| invalid(format!("{context} 不是 JSON 对象")))?;
    reject_unknown_null_root(object, context)?;
    serde_json::from_value(value)
        .map_err(|err| invalid(format!("{context} JSON 契约不匹配: {err}")))
}

fn reject_unknown_null_root(object: &Map<String, Value>, context: &str) -> Result<(), AppError> {
    if object.is_empty() {
        return Err(invalid(format!("{context} 不能是空对象")));
    }
    Ok(())
}

fn normalize_script_line_type(value: String, label: &str) -> Result<String, AppError> {
    let value = required_text(value, label)?;
    if SCRIPT_LINE_TYPES.contains(&value.as_str()) {
        Ok(value)
    } else {
        Err(invalid(format!("{label} 无效: {value}")))
    }
}

fn normalize_source_span_ids(
    values: Vec<String>,
    label: &str,
    require_nonempty: bool,
) -> Result<Vec<String>, AppError> {
    if require_nonempty && values.is_empty() {
        return Err(invalid(format!("{label} 不能为空")));
    }
    let mut seen = BTreeSet::new();
    let mut normalized = Vec::new();
    for value in values {
        let value = required_text(value, label)?;
        if seen.insert(value.clone()) {
            normalized.push(value);
        }
    }
    if require_nonempty && normalized.is_empty() {
        return Err(invalid(format!("{label} 不能为空")));
    }
    Ok(normalized)
}

fn normalize_text_vec(values: Vec<String>, label: &str) -> Result<Vec<String>, AppError> {
    let mut seen = BTreeSet::new();
    let mut normalized = Vec::new();
    for value in values {
        let value = required_text(value, label)?;
        if seen.insert(value.clone()) {
            normalized.push(value);
        }
    }
    Ok(normalized)
}

fn required_text(value: String, label: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        Err(invalid(format!("{label} 不能为空")))
    } else {
        Ok(value.to_string())
    }
}

fn optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

fn require_nonempty<T>(values: &[T], label: &str) -> Result<(), AppError> {
    if values.is_empty() {
        Err(invalid(format!("{label} 不能为空")))
    } else {
        Ok(())
    }
}

fn require_positive(value: i32, label: &str) -> Result<(), AppError> {
    if value > 0 {
        Ok(())
    } else {
        Err(invalid(format!("{label} 必须是正整数")))
    }
}

fn require_positive_i64(value: i64, label: &str) -> Result<(), AppError> {
    if value > 0 {
        Ok(())
    } else {
        Err(invalid(format!("{label} 必须是正整数")))
    }
}

fn ensure_contiguous_indexes<I>(indexes: I, label: &str) -> Result<(), AppError>
where
    I: IntoIterator<Item = i32>,
{
    let mut indexes = indexes.into_iter().collect::<Vec<_>>();
    indexes.sort_unstable();
    for (offset, index) in indexes.iter().enumerate() {
        let expected = i32::try_from(offset + 1)
            .map_err(|_| invalid(format!("{label} 索引超出 i32 可表示范围")))?;
        if *index != expected {
            return Err(invalid(format!(
                "{label} index 必须从 1 开始连续递增，期望 {expected}，实际 {index}"
            )));
        }
    }
    Ok(())
}

fn ensure_unique_scene_indexes<I>(indexes: I, context: &str) -> Result<(), AppError>
where
    I: IntoIterator<Item = i32>,
{
    ensure_unique_indexes(indexes, &format!("{context}.scene_lines.scene_index"))
}

fn ensure_unique_indexes<I>(indexes: I, label: &str) -> Result<(), AppError>
where
    I: IntoIterator<Item = i32>,
{
    let mut seen = BTreeSet::new();
    for index in indexes {
        if !seen.insert(index) {
            return Err(invalid(format!("{label} 重复: {index}")));
        }
    }
    Ok(())
}

fn invalid(message: String) -> AppError {
    AppError::InvalidArgument(message)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_scene_segmentation_contract() {
        let parsed = scene_segmentation_from_value(
            json!({
                "scenes": [
                    {
                        "index": 1,
                        "title": "雨夜重逢",
                        "location": "车站",
                        "time_of_day": "夜晚",
                        "participants": ["林晓雨", "林晓雨"],
                        "summary": "两人重逢",
                        "source_span_ids": ["ch1:p1", "ch1:p1"]
                    }
                ]
            }),
            "galgame_scene_segmentation",
        )
        .expect("scene segmentation");

        assert_eq!(parsed.scenes.len(), 1);
        assert_eq!(parsed.scenes[0].participants, vec!["林晓雨"]);
        assert_eq!(parsed.scenes[0].source_span_ids, vec!["ch1:p1"]);
    }

    #[test]
    fn rejects_scene_segmentation_with_gap_indexes() {
        let err = scene_segmentation_from_value(
            json!({
                "scenes": [
                    {"index": 2, "title": "第二场", "source_span_ids": ["s1"]}
                ]
            }),
            "galgame_scene_segmentation",
        )
        .expect_err("gap indexes");

        assert!(err.to_string().contains("从 1 开始连续递增"));
    }

    #[test]
    fn parses_script_adaptation_contract() {
        let parsed = script_adaptation_from_value(
            json!({
                "scene_lines": [
                    {
                        "scene_index": 1,
                        "lines": [
                            {
                                "index": 1,
                                "line_type": "dialogue",
                                "speaker_name": "林晓雨",
                                "text": "你终于来了。",
                                "source_span_ids": ["s1"]
                            },
                            {
                                "index": 2,
                                "line_type": "narration",
                                "speaker_name": "",
                                "text": "雨声压过了脚步声。",
                                "source_span_ids": ["s1"]
                            }
                        ]
                    }
                ]
            }),
            "galgame_script_adaptation",
        )
        .expect("script adaptation");

        assert_eq!(parsed.scene_lines[0].lines.len(), 2);
        assert_eq!(parsed.scene_lines[0].lines[1].speaker_name, None);
    }

    #[test]
    fn rejects_script_adaptation_dialogue_without_speaker() {
        let err = script_adaptation_from_value(
            json!({
                "scene_lines": [{
                    "scene_index": 1,
                    "lines": [{
                        "index": 1,
                        "line_type": "dialogue",
                        "speaker_name": "",
                        "text": "没有说话人",
                        "source_span_ids": ["s1"]
                    }]
                }]
            }),
            "galgame_script_adaptation",
        )
        .expect_err("dialogue speaker");

        assert!(err.to_string().contains("speaker_name"));
    }

    #[test]
    fn rejects_script_adaptation_line_without_source_span() {
        let err = script_adaptation_from_value(
            json!({
                "scene_lines": [{
                    "scene_index": 1,
                    "lines": [{
                        "index": 1,
                        "line_type": "narration",
                        "speaker_name": "",
                        "text": "雨声压过了脚步声。",
                        "source_span_ids": []
                    }]
                }]
            }),
            "galgame_script_adaptation",
        )
        .expect_err("script adaptation lines must keep source evidence");

        assert!(err.to_string().contains("source_span_ids 不能为空"));
    }

    #[test]
    fn parses_character_staging_contract() {
        let parsed = character_staging_from_value(
            json!({
                "scene_lines": [{
                    "scene_index": 1,
                    "lines": [{
                        "index": 1,
                        "line_type": "dialogue",
                        "speaker_profile_id": 7,
                        "speaker_name": "林晓雨",
                        "text": "你终于来了。",
                        "expression": "smile",
                        "pose": "standing",
                        "position": "left",
                        "source_span_ids": ["s1"]
                    }]
                }]
            }),
            "galgame_character_staging",
        )
        .expect("character staging");

        assert_eq!(parsed.scene_lines[0].lines[0].speaker_profile_id, Some(7));
        assert_eq!(
            parsed.scene_lines[0].lines[0].expression.as_deref(),
            Some("smile")
        );
    }

    #[test]
    fn rejects_character_staging_invalid_expression() {
        let err = character_staging_from_value(
            json!({
                "scene_lines": [{
                    "scene_index": 1,
                    "lines": [{
                        "index": 1,
                        "line_type": "dialogue",
                        "speaker_profile_id": 7,
                        "text": "你终于来了。",
                        "expression": "ecstatic",
                        "position": "left",
                        "source_span_ids": ["s1"]
                    }]
                }]
            }),
            "galgame_character_staging",
        )
        .expect_err("invalid expression");

        assert!(err.to_string().contains("expression 无效"));
    }

    #[test]
    fn rejects_character_staging_bound_character_without_expression() {
        let err = character_staging_from_value(
            json!({
                "scene_lines": [{
                    "scene_index": 1,
                    "lines": [{
                        "index": 1,
                        "line_type": "dialogue",
                        "speaker_profile_id": 7,
                        "speaker_name": "林晓雨",
                        "text": "你终于来了。",
                        "pose": "standing",
                        "position": "left",
                        "source_span_ids": ["s1"]
                    }]
                }]
            }),
            "galgame_character_staging",
        )
        .expect_err("bound character line should require expression");

        assert!(err.to_string().contains("必须显式提供"));
    }

    #[test]
    fn rejects_character_staging_line_without_source_span() {
        let err = character_staging_from_value(
            json!({
                "scene_lines": [{
                    "scene_index": 1,
                    "lines": [{
                        "index": 1,
                        "line_type": "narration",
                        "speaker_profile_id": null,
                        "speaker_name": "",
                        "text": "雨声压过了脚步声。",
                        "source_span_ids": []
                    }]
                }]
            }),
            "galgame_character_staging",
        )
        .expect_err("staged lines must keep source evidence");

        assert!(err.to_string().contains("source_span_ids 不能为空"));
    }

    #[test]
    fn parses_visual_plan_contract() {
        let parsed = visual_plan_from_value(
            json!({
                "backgrounds": [{
                    "scene_index": 1,
                    "background_note": "雨夜车站，冷色灯光",
                    "scene_profile_id": null,
                    "source_span_ids": ["s1"]
                }],
                "event_cgs": [{
                    "scene_index": 1,
                    "line_index": 2,
                    "cg_note": "两人在雨中对视",
                    "importance": "high",
                    "source_span_ids": ["s2"]
                }]
            }),
            "galgame_visual_planning",
        )
        .expect("visual plan");

        assert_eq!(parsed.backgrounds.len(), 1);
        assert_eq!(parsed.event_cgs[0].importance, "high");
    }

    #[test]
    fn rejects_branch_design_with_single_option() {
        let err = branch_design_from_value(
            json!({
                "choices": [{
                    "scene_index": 1,
                    "after_line_index": 3,
                    "question": "怎么回应？",
                    "options": [{
                        "option_text": "沉默",
                        "branch_title": "沉默",
                        "branch_summary": "短暂沉默",
                        "returns_to_scene_index": 2
                    }]
                }]
            }),
            "galgame_branch_design",
        )
        .expect_err("option count");

        assert!(err.to_string().contains("2 到 3"));
    }

    #[test]
    fn rejects_branch_design_option_without_return_scene() {
        let err = branch_design_from_value(
            json!({
                "choices": [{
                    "scene_index": 1,
                    "after_line_index": 3,
                    "question": "怎么回应？",
                    "options": [
                        {
                            "option_text": "沉默",
                            "branch_title": "沉默",
                            "branch_summary": "短暂沉默"
                        },
                        {
                            "option_text": "追问",
                            "branch_title": "追问",
                            "branch_summary": "继续追问",
                            "returns_to_scene_index": 2
                        }
                    ]
                }]
            }),
            "galgame_branch_design",
        )
        .expect_err("missing returns_to_scene_index");

        assert!(err.to_string().contains("returns_to_scene_index 不能为空"));
    }

    #[test]
    fn parse_galgame_json_object_rejects_markdown_fence() {
        let err =
            parse_galgame_json_object("```json\n{}\n```", "galgame").expect_err("markdown fence");

        assert!(err.to_string().contains("代码围栏"));
    }
}
