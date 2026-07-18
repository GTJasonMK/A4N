use std::collections::BTreeMap;

use a4n_core::error::AppError;

use super::super::GalgamePipelineLineStaged;
use super::contracts::*;
use super::rules::invalid;

pub(super) fn source_snapshot_chapter_id_for_scene(
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

pub(super) fn text_anchor_id_for_line(
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
