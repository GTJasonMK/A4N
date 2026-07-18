use a4n_core::error::AppError;
use serde_json::Value;

use super::{
    GALGAME_EXPRESSIONS, GalgameLinePresentationDocumentV2, GalgameStagePositionV2,
    LINE_TYPE_CHOICE, LINE_TYPE_DIALOGUE, LINE_TYPE_MONOLOGUE, LINE_TYPE_NARRATION,
    LINE_TYPE_STAGE_DIRECTION, SCENE_ENTRY_BRANCH, SCENE_ENTRY_MAIN, validate_staged_presentation,
};

const LINE_TYPES: &[&str] = &[
    LINE_TYPE_NARRATION,
    LINE_TYPE_DIALOGUE,
    LINE_TYPE_MONOLOGUE,
    LINE_TYPE_STAGE_DIRECTION,
    LINE_TYPE_CHOICE,
];
const POSITIONS: &[&str] = &["left", "center", "right"];

pub(crate) fn normalize_line_type(value: &str) -> Result<&str, AppError> {
    let value = value.trim();
    if LINE_TYPES.contains(&value) {
        Ok(value)
    } else {
        Err(AppError::InvalidArgument(format!(
            "Galgame 行类型无效: {value}"
        )))
    }
}

pub(crate) fn validate_and_project_presentation(
    presentation: &GalgameLinePresentationDocumentV2,
    speaker_profile_id: Option<i64>,
    context: &str,
) -> Result<(Option<String>, Option<String>, Option<String>), AppError> {
    let directing = presentation.directing_presentation();
    validate_staged_presentation(&directing, speaker_profile_id, context)?;
    validate_presentation_directives(presentation, context)?;
    let Some(speaker_profile_id) = speaker_profile_id else {
        return Ok((None, None, None));
    };
    let speaker = presentation
        .visible_cast
        .iter()
        .find(|cast| cast.character_profile_id == speaker_profile_id)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "{context}.visible_cast 必须包含 speaker_profile_id={speaker_profile_id}"
            ))
        })?;
    Ok((
        Some(speaker.expression.clone()),
        Some(speaker.pose.clone()),
        Some(
            match speaker.position {
                GalgameStagePositionV2::Left => "left",
                GalgameStagePositionV2::Center => "center",
                GalgameStagePositionV2::Right => "right",
            }
            .to_string(),
        ),
    ))
}

fn validate_presentation_directives(
    presentation: &GalgameLinePresentationDocumentV2,
    context: &str,
) -> Result<(), AppError> {
    if let Some(importance) = presentation.cg_importance.as_deref()
        && !["normal", "high", "critical"].contains(&importance)
    {
        return Err(AppError::InvalidArgument(format!(
            "{context}.cg_importance 无效: {importance}"
        )));
    }
    if presentation.cg == Some(true)
        && presentation
            .cg_note
            .as_deref()
            .is_none_or(|note| note.trim().is_empty())
    {
        return Err(AppError::InvalidArgument(format!(
            "{context}.cg=true 时 cg_note 不能为空"
        )));
    }
    if let Some(transition) = presentation.transition.as_deref()
        && !["cut", "fade", "dissolve"].contains(&transition)
    {
        return Err(AppError::InvalidArgument(format!(
            "{context}.transition 无效: {transition}"
        )));
    }
    if let Some(scene_index) = presentation.returns_to_scene_index
        && scene_index <= 0
    {
        return Err(AppError::InvalidArgument(format!(
            "{context}.returns_to_scene_index 必须是正整数"
        )));
    }
    Ok(())
}

pub(super) fn validate_scene_entry_kind(value: &str) -> Result<(), AppError> {
    match value.trim() {
        SCENE_ENTRY_MAIN | SCENE_ENTRY_BRANCH => Ok(()),
        other => Err(AppError::InvalidArgument(format!(
            "Galgame 场 entry_kind 无效: {other}"
        ))),
    }
}

pub(crate) fn normalize_position(value: Option<String>) -> Result<Option<String>, AppError> {
    let Some(value) = normalize_optional_text(value) else {
        return Ok(None);
    };
    if POSITIONS.contains(&value.as_str()) {
        Ok(Some(value))
    } else {
        Err(AppError::InvalidArgument(format!(
            "Galgame 立绘站位无效: {value}"
        )))
    }
}

pub(super) fn normalize_optional_text(value: Option<String>) -> Option<String> {
    value
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

pub(super) fn normalize_optional_positive_id(
    value: Option<i64>,
    label: &str,
) -> Result<Option<i64>, AppError> {
    match value {
        Some(id) => {
            validate_positive_id(id, label)?;
            Ok(Some(id))
        }
        None => Ok(None),
    }
}

pub(crate) fn validate_line_semantics(
    line_type: &str,
    speaker_profile_id: Option<i64>,
    expression: Option<&str>,
) -> Result<(), AppError> {
    let expression = expression.map(str::trim).filter(|value| !value.is_empty());
    if let Some(expression) = expression
        && !is_valid_galgame_expression(expression)
    {
        return Err(AppError::InvalidArgument(format!(
            "Galgame 表情无效: {expression}"
        )));
    }
    match line_type {
        LINE_TYPE_DIALOGUE | LINE_TYPE_MONOLOGUE if speaker_profile_id.is_none() => Err(
            AppError::InvalidArgument(format!("{line_type} 行必须绑定 speaker_profile_id")),
        ),
        LINE_TYPE_DIALOGUE | LINE_TYPE_MONOLOGUE if expression.is_none() => Err(
            AppError::InvalidArgument(format!("{line_type} 行绑定角色时必须显式提供 expression")),
        ),
        LINE_TYPE_DIALOGUE | LINE_TYPE_MONOLOGUE => Ok(()),
        _ if speaker_profile_id.is_some() => Err(AppError::InvalidArgument(format!(
            "{line_type} 行不能绑定 speaker_profile_id"
        ))),
        _ => Ok(()),
    }
}

fn is_valid_galgame_expression(value: &str) -> bool {
    GALGAME_EXPRESSIONS.contains(&value)
}

pub(super) fn validate_positive_id(id: i64, label: &str) -> Result<(), AppError> {
    if id > 0 {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(format!("{label} 必须是正整数")))
    }
}

pub(super) fn required_text<'a>(value: &'a str, label: &str) -> Result<&'a str, AppError> {
    let value = value.trim();
    if value.is_empty() {
        Err(AppError::InvalidArgument(format!("{label} 不能为空")))
    } else {
        Ok(value)
    }
}

pub(super) fn ensure_json_object(value: &Value, label: &str) -> Result<(), AppError> {
    if value.is_object() {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(format!(
            "{label} 必须是 JSON 对象"
        )))
    }
}

pub(super) fn to_i32(value: usize, label: &str) -> Result<i32, AppError> {
    i32::try_from(value)
        .map_err(|_| AppError::InvalidArgument(format!("{label} 超出 i32 可表示范围")))
}
