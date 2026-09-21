use a4n_llm_provider_contract::provider::ChatRequest;
use serde_json::Value;

#[derive(Debug, Clone)]
pub(in crate::chapter_analysis) struct ChapterAnalysisRun {
    pub(in crate::chapter_analysis) analysis: Value,
    pub(in crate::chapter_analysis) stage_traces: Vec<ChapterAnalysisStageTrace>,
}

#[derive(Debug, Clone)]
pub(in crate::chapter_analysis) struct ChapterAnalysisStageTrace {
    pub(in crate::chapter_analysis) stage: &'static str,
    pub(in crate::chapter_analysis) request: ChatRequest,
    pub(in crate::chapter_analysis) output: Value,
}
