//! Pure chapter input contracts used by chapter-analysis workflows.

pub trait ChapterAnalysisSource {
    fn has_available_analysis_content(&self) -> bool;
}

#[derive(Debug, Clone)]
pub struct ImportChapterContentInput {
    pub project_id: i64,
    pub chapter_number: i32,
    pub content: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone)]
pub struct EditChapterContentInput {
    pub chapter_id: i64,
    pub content: String,
    pub title: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChapterOutlineSyncDraft {
    pub chapter_number: i32,
    pub title: String,
    pub summary: String,
    pub beats: Option<serde_json::Value>,
    pub key_scenes: Option<serde_json::Value>,
    pub pacing_score: Option<f32>,
    pub foreshadowing_setup: Option<serde_json::Value>,
    pub foreshadowing_payoff: Option<serde_json::Value>,
    pub characters_involved: Option<serde_json::Value>,
    pub word_count_target: Option<i32>,
}

pub fn required_outline_pacing_score(
    pacing_score: Option<f32>,
    chapter_number: i32,
) -> Result<f32, String> {
    let score = pacing_score
        .ok_or_else(|| format!("第 {chapter_number} 章缺少 pacing_score，无法分析节奏"))?;
    if score.is_finite() {
        Ok(score)
    } else {
        Err(format!(
            "第 {chapter_number} 章 pacing_score 不是有限数字，无法分析节奏"
        ))
    }
}
