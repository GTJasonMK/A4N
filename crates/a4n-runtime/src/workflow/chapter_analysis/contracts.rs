use super::prelude::*;

pub const CHAPTER_ANALYSIS_PROGRESS_TOTAL: u32 = 100;
pub(super) const CHAPTER_ANALYSIS_STEP_FACTS: &str = "chapter_analysis.facts";
pub(super) const CHAPTER_ANALYSIS_STEP_DERIVED_SYNC: &str = "chapter_analysis.derived_sync";
pub(super) const CHAPTER_ANALYSIS_STEP_SYNC_OUTPUTS: &str = "chapter_analysis.sync_outputs";
pub(super) const CHAPTER_ANALYSIS_STEP_OBJECT_DISCOVERY: &str = "chapter_analysis.object_discovery";
pub(super) const CHAPTER_ANALYSIS_STEP_CHARACTER_DISCOVERY: &str =
    "chapter_analysis.character_discovery";
pub(super) const CHAPTER_ANALYSIS_STEP_SCENE_SYNC: &str = "chapter_analysis.scene_sync";
pub(super) const CHAPTER_ANALYSIS_STEP_CHARACTER_SYNC: &str = "chapter_analysis.character_sync";
pub(super) const CHAPTER_ANALYSIS_STEP_REFRESH_STATUS: &str = "chapter_analysis.refresh_status";
pub type ChapterAnalysisProgressReporter<'a> = &'a (dyn Fn(u32, u32, &str) + Send + Sync);

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct GenerateChapterAnalysisInput {
    pub chapter_id: i64,
}

pub struct ChapterAnalysisProgress<'a> {
    pub(super) reporter: Option<ChapterAnalysisProgressReporter<'a>>,
    pub(super) cancel_token: Option<&'a CancellationToken>,
}

impl<'a> ChapterAnalysisProgress<'a> {
    pub fn new(
        reporter: Option<ChapterAnalysisProgressReporter<'a>>,
        cancel_token: Option<&'a CancellationToken>,
    ) -> Self {
        Self {
            reporter,
            cancel_token,
        }
    }

    pub(super) fn report(
        &self,
        current: u32,
        total: u32,
        message: &str,
        cancelled_message: &str,
    ) -> Result<(), String> {
        if let Some(reporter) = self.reporter {
            reporter(current, total.max(1), message);
        }
        if self
            .cancel_token
            .is_some_and(CancellationToken::is_cancelled)
        {
            Err(cancelled_message.to_string())
        } else {
            Ok(())
        }
    }
}
