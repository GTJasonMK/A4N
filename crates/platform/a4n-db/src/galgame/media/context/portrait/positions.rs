use std::collections::BTreeSet;

use crate::galgame::GalgameGenerationRecordError;

pub(super) fn normalized_galgame_position(value: Option<&str>) -> Option<&str> {
    value.map(str::trim).filter(|value| !value.is_empty())
}

pub(super) fn validate_galgame_position(
    position: &str,
    spec_id: i64,
    line_id: i64,
) -> Result<(), GalgameGenerationRecordError> {
    if matches!(position, "left" | "center" | "right") {
        Ok(())
    } else {
        Err(GalgameGenerationRecordError::InvalidArgument(format!(
            "Galgame 立绘差分 source_line_ids 行 position 无效: spec_id={}, line_id={}, position={position}",
            spec_id, line_id
        )))
    }
}

pub fn validate_galgame_portrait_position_samples(
    spec_id: i64,
    position_samples: &[String],
    source_positions: &BTreeSet<String>,
) -> Result<(), GalgameGenerationRecordError> {
    let mut sample_positions = BTreeSet::new();
    for position in position_samples {
        if !matches!(position.as_str(), "left" | "center" | "right") {
            return Err(GalgameGenerationRecordError::InvalidArgument(format!(
                "Galgame 立绘差分 position_samples 包含无效站位: spec_id={}, position={position}",
                spec_id
            )));
        }
        sample_positions.insert(position.clone());
    }
    if sample_positions != *source_positions {
        return Err(GalgameGenerationRecordError::InvalidArgument(format!(
            "Galgame 立绘差分 position_samples 与 source_line_ids 行站位不一致: spec_id={}, position_samples={:?}, source_positions={:?}",
            spec_id, sample_positions, source_positions
        )));
    }
    Ok(())
}
