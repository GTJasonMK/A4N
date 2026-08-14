mod summaries;

pub use a4n_api_contract::outline::{
    ApplyBeatMappingInput, BatchGeneratePartOutlinesInput, ChapterOutlinePacing,
    ContinuePartOutlinesInput, DeleteLatestOutlineRecordsInput,
    GenerateChapterOutlinesByCountInput, GeneratePartChapterOutlinesInput, GeneratePartsInput,
    OutlineGenerationProgressSummary, OutlineMutationResult, RegenerateChapterOutlineInput,
    RegenerateLastPartInput, RegeneratePartOutlineInput, RegeneratePartsInput,
    SaveChapterOutlineInput, SaveChapterOutlineResult, SavePartOutlineInput, SavePartOutlineResult,
    UpdateChapterOutlineInput, UpdateChapterOutlineResult,
};
pub use summaries::{ChapterOutlineSummary, PartOutlineSummary};
pub(super) use summaries::{chapter_outline_summary, part_outline_summary};
