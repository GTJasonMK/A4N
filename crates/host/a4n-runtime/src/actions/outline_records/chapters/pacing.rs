use crate::AppState;
use crate::actions::access as project_access;
use crate::actions::outline_records::ChapterOutlinePacing;
use a4n_api_contract::outline::{ChapterOutlinePacingAnalysis, ChapterOutlineSummary};
use a4n_domain::chapter::required_outline_pacing_score;
use a4n_error::AppError;

pub async fn get_chapter_outlines_pacing(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<ChapterOutlinePacing, AppError> {
    project_access::verify_project_editor(state, user_id, project_id).await?;
    let outlines = state
        .outline_read_application()
        .list_chapters(user_id, project_id)
        .await?;
    build_chapter_outlines_pacing(&outlines)
}

pub(in crate::actions::outline_records) fn build_chapter_outlines_pacing(
    outlines: &[ChapterOutlineSummary],
) -> Result<ChapterOutlinePacing, AppError> {
    if outlines.is_empty() {
        return Ok(ChapterOutlinePacing {
            pacing_score: 0.0,
            chapter_count: 0,
            summary: None,
            analysis: Vec::new(),
        });
    }

    let mut scores: Vec<f64> = Vec::new();
    let mut analysis = Vec::new();

    for outline in outlines {
        let pacing = required_outline_pacing_score(outline.pacing_score, outline.chapter_number)
            .map_err(AppError::InvalidArgument)? as f64;
        scores.push(pacing);
        analysis.push(ChapterOutlinePacingAnalysis {
            chapter_number: outline.chapter_number,
            title: outline.title.clone(),
            pacing_score: pacing,
            word_count_target: outline.word_count_target,
            has_beats: outline.beats.is_some(),
            has_key_scenes: outline.key_scenes.is_some(),
        });
    }

    let avg_pacing: f64 = scores.iter().sum::<f64>() / scores.len() as f64;
    Ok(ChapterOutlinePacing {
        pacing_score: (avg_pacing * 100.0).round() / 100.0,
        chapter_count: outlines.len(),
        summary: Some(
            if avg_pacing < 0.3 {
                "节奏偏慢"
            } else if avg_pacing > 0.7 {
                "节奏偏快"
            } else {
                "节奏适中"
            }
            .to_string(),
        ),
        analysis,
    })
}
