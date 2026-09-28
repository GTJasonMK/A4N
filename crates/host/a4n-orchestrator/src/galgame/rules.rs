use a4n_error::AppError;
use serde_json::Value;

use super::{
    GALGAME_EXPRESSIONS, GalgameLinePresentationDocumentV3, LINE_TYPE_CHOICE, LINE_TYPE_DIALOGUE,
    LINE_TYPE_MONOLOGUE, LINE_TYPE_NARRATION, LINE_TYPE_STAGE_DIRECTION, SCENE_ENTRY_BRANCH,
    SCENE_ENTRY_MAIN, validate_staged_presentation,
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
    presentation: &GalgameLinePresentationDocumentV3,
    speaker_profile_id: Option<i64>,
    context: &str,
) -> Result<(Option<String>, Option<String>, Option<String>), AppError> {
    presentation
        .validate_source_identity(None)
        .map_err(AppError::InvalidArgument)?;
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
        Some(speaker.position.to_string()),
    ))
}

pub(crate) fn parse_and_validate_presentation_projection(
    presentation_json: &Value,
    speaker_profile_id: Option<i64>,
    expression: Option<&str>,
    pose: Option<&str>,
    position: Option<&str>,
    context: &str,
) -> Result<GalgameLinePresentationDocumentV3, AppError> {
    let presentation =
        serde_json::from_value::<GalgameLinePresentationDocumentV3>(presentation_json.clone())
            .map_err(|error| {
                AppError::InvalidArgument(format!(
                    "{context} presentation_json 不符合 v3 合同: {error}"
                ))
            })?;
    let projected = validate_and_project_presentation(&presentation, speaker_profile_id, context)?;
    if expression != projected.0.as_deref()
        || pose != projected.1.as_deref()
        || position != projected.2.as_deref()
    {
        return Err(AppError::InvalidArgument(format!(
            "{context} 的演出投影列与 presentation_json 不一致"
        )));
    }
    Ok(presentation)
}

fn validate_presentation_directives(
    presentation: &GalgameLinePresentationDocumentV3,
    context: &str,
) -> Result<(), AppError> {
    if presentation.cg == Some(true) {
        if presentation
            .cg_note
            .as_deref()
            .is_none_or(|note| note.trim().is_empty())
        {
            return Err(AppError::InvalidArgument(format!(
                "{context}.cg=true 时 cg_note 不能为空"
            )));
        }
        if presentation.cg_importance.is_none() {
            return Err(AppError::InvalidArgument(format!(
                "{context}.cg=true 时 cg_importance 不能为空"
            )));
        }
    } else if presentation.cg_note.is_some() || presentation.cg_importance.is_some() {
        return Err(AppError::InvalidArgument(format!(
            "{context}.cg 未启用时不能保留 cg_note/cg_importance"
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
    pose: Option<&str>,
) -> Result<(), AppError> {
    let expression = expression.map(str::trim).filter(|value| !value.is_empty());
    let pose = pose.map(str::trim).filter(|value| !value.is_empty());
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
        LINE_TYPE_DIALOGUE | LINE_TYPE_MONOLOGUE if pose.is_none() => Err(
            AppError::InvalidArgument(format!("{line_type} 行绑定角色时必须显式提供 pose")),
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

pub(super) fn to_i32(value: usize, label: &str) -> Result<i32, AppError> {
    i32::try_from(value)
        .map_err(|_| AppError::InvalidArgument(format!("{label} 超出 i32 可表示范围")))
}
