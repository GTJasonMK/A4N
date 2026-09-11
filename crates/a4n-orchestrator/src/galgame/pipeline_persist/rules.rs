use std::collections::BTreeMap;

use a4n_error::AppError;
use serde_json::{Value, json};

use super::super::{
    GalgameLinePresentationDocumentV2, GalgamePipelineEventCgPlan, GalgamePipelineLineStaged,
    LINE_TYPE_CHOICE, LINE_TYPE_DIALOGUE, LINE_TYPE_MONOLOGUE, LINE_TYPE_NARRATION,
    LINE_TYPE_STAGE_DIRECTION,
};

pub(super) fn validate_persist_line(line: &GalgamePipelineLineStaged) -> Result<(), AppError> {
    super::super::validate_staged_presentation(
        &line.presentation,
        line.speaker_profile_id,
        "Galgame persist",
    )?;
    match line.line_type.as_str() {
        LINE_TYPE_DIALOGUE | LINE_TYPE_MONOLOGUE if line.speaker_profile_id.is_none() => {
            Err(invalid(format!(
                "Galgame persist {} 行必须绑定 speaker_profile_id",
                line.line_type
            )))
        }
        LINE_TYPE_DIALOGUE | LINE_TYPE_MONOLOGUE
            if line
                .speaker_name
                .as_deref()
                .is_none_or(|name| name.trim().is_empty()) =>
        {
            Err(invalid(format!(
                "Galgame persist {} 行必须保留 speaker_name",
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

pub(super) fn to_i32(value: usize, label: &str) -> Result<i32, AppError> {
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

pub(super) fn presentation_document_for_line(
    line: &GalgamePipelineLineStaged,
    cg: Option<&GalgamePipelineEventCgPlan>,
) -> GalgameLinePresentationDocumentV2 {
    let mut value = GalgameLinePresentationDocumentV2::from(line.presentation.clone());
    if let Some(cg) = cg {
        value.cg = Some(true);
        value.cg_note = Some(cg.cg_note.clone());
        value.cg_importance = Some(cg.importance);
    }
    value
}

pub(super) fn participants_json(participants: BTreeMap<i64, String>) -> Value {
    let items = participants
        .into_iter()
        .map(|(profile_id, name)| {
            json!({
                "character_profile_id": profile_id,
                "name": name,
            })
        })
        .collect::<Vec<_>>();
    json!(items)
}

pub(super) fn speaker_projection(
    line: &GalgamePipelineLineStaged,
) -> Option<&super::super::GalgameVisibleCastMemberV2> {
    let speaker_id = line.speaker_profile_id?;
    line.presentation
        .visible_cast
        .iter()
        .find(|cast| cast.character_profile_id == speaker_id)
}

pub(super) fn invalid(message: impl Into<String>) -> AppError {
    AppError::InvalidArgument(message.into())
}
