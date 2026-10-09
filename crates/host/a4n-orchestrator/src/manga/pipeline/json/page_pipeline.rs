use std::collections::{HashMap, HashSet};

use a4n_domain::{
    manga::{MangaPageAdaptationV1, MangaPageContentV1, MangaPageSlotsV1, MangaPageV4},
    media_authoring::{AdaptationBeatRef, AdaptationImportance},
    source_spans::SourceSpan,
};
use a4n_error::AppError;
use serde_json::Value;

use crate::media_authoring::PreparedAdaptation;

pub(crate) fn parse_manga_page_slots(
    value: &Value,
    adaptation: &PreparedAdaptation,
    source_spans: &[SourceSpan],
    context: &str,
) -> Result<MangaPageSlotsV1, AppError> {
    let slots: MangaPageSlotsV1 = serde_json::from_value(value.clone()).map_err(|error| {
        AppError::InvalidArgument(format!(
            "{context} 不符合 manga_page_slots_v1 合同: {error}"
        ))
    })?;
    slots
        .validate()
        .map_err(|error| AppError::InvalidArgument(format!("{context}: {error}")))?;
    let planned_page_count = adaptation
        .plan
        .validate_manga_page_count()
        .map_err(AppError::InvalidArgument)? as usize;
    if slots.pages.len() != planned_page_count {
        return Err(AppError::InvalidArgument(format!(
            "{context} 必须生成 {planned_page_count} 页，实际为 {} 页",
            slots.pages.len()
        )));
    }

    let available_spans = source_spans
        .iter()
        .map(|span| span.id.as_str())
        .collect::<HashSet<_>>();
    let mut available_beats = HashMap::new();
    for unit in &adaptation.plan.units {
        for beat in &unit.beats {
            available_beats.insert(
                AdaptationBeatRef {
                    unit_id: unit.id.clone(),
                    beat_id: beat.id.clone(),
                },
                beat.importance == AdaptationImportance::Required,
            );
        }
    }
    let mut used_beats = HashSet::new();
    for (page_index, page) in slots.pages.iter().enumerate() {
        for span_id in &page.content.source_span_ids {
            if !available_spans.contains(span_id.as_str()) {
                return Err(AppError::InvalidArgument(format!(
                    "{context} pages[{page_index}] 引用了不存在的 source_span_id: {span_id}"
                )));
            }
        }
        for beat_ref in &page.content.beat_refs {
            if !available_beats.contains_key(beat_ref) {
                return Err(AppError::InvalidArgument(format!(
                    "{context} pages[{page_index}] 引用了不存在的改编节拍: unit_id={}, beat_id={}",
                    beat_ref.unit_id, beat_ref.beat_id
                )));
            }
            used_beats.insert(beat_ref.clone());
        }
    }
    if let Some((beat_ref, _)) = available_beats
        .iter()
        .find(|(beat_ref, required)| **required && !used_beats.contains(*beat_ref))
    {
        return Err(AppError::InvalidArgument(format!(
            "{context} 遗漏关键改编节拍: unit_id={}, beat_id={}",
            beat_ref.unit_id, beat_ref.beat_id
        )));
    }
    Ok(slots)
}

pub(crate) fn parse_manga_page_adaptation(
    value: &Value,
    content: &MangaPageContentV1,
    context: &str,
) -> Result<MangaPageAdaptationV1, AppError> {
    let adaptation: MangaPageAdaptationV1 =
        serde_json::from_value(value.clone()).map_err(|error| {
            AppError::InvalidArgument(format!("{context} 不符合页面改编计划合同: {error}"))
        })?;
    adaptation
        .validate(content)
        .map_err(|error| AppError::InvalidArgument(format!("{context}: {error}")))?;
    if adaptation.beat_refs.iter().collect::<HashSet<_>>()
        != content.beat_refs.iter().collect::<HashSet<_>>()
    {
        return Err(AppError::InvalidArgument(format!(
            "{context} 必须覆盖页面内容中的全部 beat_refs"
        )));
    }
    Ok(adaptation)
}

pub(crate) fn parse_manga_page_storyboard(
    value: &Value,
    page_number: u32,
    content: &MangaPageContentV1,
    adaptation: &MangaPageAdaptationV1,
    context: &str,
) -> Result<MangaPageV4, AppError> {
    let page: MangaPageV4 = serde_json::from_value(value.clone()).map_err(|error| {
        AppError::InvalidArgument(format!(
            "{context} 不符合 manga_page_storyboard 合同: {error}"
        ))
    })?;
    page.validate()
        .map_err(|error| AppError::InvalidArgument(format!("{context}: {error}")))?;
    if page.page_number != page_number {
        return Err(AppError::InvalidArgument(format!(
            "{context} page_number 必须等于 {page_number}"
        )));
    }
    if page.entry_state != content.entry_state || page.exit_state != content.exit_state {
        return Err(AppError::InvalidArgument(format!(
            "{context} entry_state 和 exit_state 必须沿用页面内容合同"
        )));
    }
    if page.beat_refs.iter().collect::<HashSet<_>>()
        != adaptation.beat_refs.iter().collect::<HashSet<_>>()
    {
        return Err(AppError::InvalidArgument(format!(
            "{context} 必须覆盖页面改编计划中的全部 beat_refs"
        )));
    }
    Ok(page)
}
