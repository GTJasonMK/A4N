use a4n_core::chapter_analysis::{
    CHAPTER_ANALYSIS_CHARACTER_STATE_FIELDS, CHAPTER_ANALYSIS_FACT_FIELDS,
    CHAPTER_ANALYSIS_FORESHADOWING_FIELDS, CHAPTER_ANALYSIS_STAGE_CHARACTER_STATE,
    CHAPTER_ANALYSIS_STAGE_FACTS, CHAPTER_ANALYSIS_STAGE_FORESHADOWING_EVENTS,
    CHAPTER_ANALYSIS_STAGE_SUMMARY, CHAPTER_ANALYSIS_STEP_CHARACTER_STATES,
    CHAPTER_ANALYSIS_STEP_FACTS, CHAPTER_ANALYSIS_STEP_FORESHADOWING_EVENTS,
    CHAPTER_ANALYSIS_STEP_SUMMARY, CHAPTER_ANALYSIS_SUMMARY_FIELDS,
};
use a4n_core::structured_progress::{FieldMilestone, ProgressSpan};
use serde_json::Value;

use super::super::execution::ChapterAnalysisStageTrace;

pub(in crate::chapter_analysis) struct ExecutedChapterAnalysisStage {
    pub(in crate::chapter_analysis) output: Value,
    pub(in crate::chapter_analysis) trace: ChapterAnalysisStageTrace,
}

#[derive(Clone, Copy)]
pub(super) struct ChapterAnalysisStageSpec {
    pub(super) stage: &'static str,
    pub(super) step_key: &'static str,
    pub(super) step_order: i32,
    pub(super) context: &'static str,
    pub(super) restored_progress: u32,
    pub(super) fields: &'static [FieldMilestone],
    pub(super) progress_span: ProgressSpan,
}

pub(super) const FACTS_STAGE: ChapterAnalysisStageSpec = ChapterAnalysisStageSpec {
    stage: CHAPTER_ANALYSIS_STAGE_FACTS,
    step_key: CHAPTER_ANALYSIS_STEP_FACTS,
    step_order: 1,
    context: "事实抽取阶段",
    restored_progress: 35,
    fields: CHAPTER_ANALYSIS_FACT_FIELDS,
    progress_span: ProgressSpan::new(0, 35),
};

pub(super) const SUMMARY_STAGE: ChapterAnalysisStageSpec = ChapterAnalysisStageSpec {
    stage: CHAPTER_ANALYSIS_STAGE_SUMMARY,
    step_key: CHAPTER_ANALYSIS_STEP_SUMMARY,
    step_order: 2,
    context: "摘要综合阶段",
    restored_progress: 55,
    fields: CHAPTER_ANALYSIS_SUMMARY_FIELDS,
    progress_span: ProgressSpan::new(35, 55),
};

pub(super) const CHARACTER_STATE_STAGE: ChapterAnalysisStageSpec = ChapterAnalysisStageSpec {
    stage: CHAPTER_ANALYSIS_STAGE_CHARACTER_STATE,
    step_key: CHAPTER_ANALYSIS_STEP_CHARACTER_STATES,
    step_order: 3,
    context: "角色状态阶段",
    restored_progress: 75,
    fields: CHAPTER_ANALYSIS_CHARACTER_STATE_FIELDS,
    progress_span: ProgressSpan::new(55, 75),
};

pub(super) const FORESHADOWING_STAGE: ChapterAnalysisStageSpec = ChapterAnalysisStageSpec {
    stage: CHAPTER_ANALYSIS_STAGE_FORESHADOWING_EVENTS,
    step_key: CHAPTER_ANALYSIS_STEP_FORESHADOWING_EVENTS,
    step_order: 4,
    context: "伏笔事件阶段",
    restored_progress: 95,
    fields: CHAPTER_ANALYSIS_FORESHADOWING_FIELDS,
    progress_span: ProgressSpan::new(75, 95),
};
