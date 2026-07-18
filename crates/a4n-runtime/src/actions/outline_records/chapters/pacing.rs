use super::super::required_outline_pacing_score;
use crate::AppState;
use crate::actions::access as project_access;
use a4n_core::error::AppError;
use a4n_core::models::chapter::chapter_outline;
use serde_json::{Value, json};

pub async fn get_chapter_outlines_pacing(
    state: &AppState,
    user_id: i64,
    project_id: i64,
) -> Result<Value, AppError> {
    project_access::verify_project_editor(state, user_id, project_id).await?;
    let outlines = state
        .orchestrator
        .chapter_outlines_for_project(project_id)
        .await?;
    build_chapter_outlines_pacing(&outlines)
}

pub(in crate::actions::outline_records) fn build_chapter_outlines_pacing(
    outlines: &[chapter_outline::Model],
) -> Result<Value, AppError> {
    if outlines.is_empty() {
        return Ok(json!({
            "pacing_score": 0.0,
            "analysis": [],
            "chapter_count": 0
        }));
    }

    let mut scores: Vec<f64> = Vec::new();
    let mut analysis = Vec::new();

    for outline in outlines {
        let pacing =
            required_outline_pacing_score(outline.pacing_score, outline.chapter_number)? as f64;
        scores.push(pacing);
        analysis.push(json!({
            "chapter_number": outline.chapter_number,
            "title": outline.title,
            "pacing_score": pacing,
            "word_count_target": outline.word_count_target,
            "has_beats": outline.beats.is_some(),
            "has_key_scenes": outline.key_scenes.is_some(),
        }));
    }

    let avg_pacing: f64 = scores.iter().sum::<f64>() / scores.len() as f64;
    Ok(json!({
        "pacing_score": (avg_pacing * 100.0).round() / 100.0,
        "chapter_count": outlines.len(),
        "analysis": analysis,
        "summary": if avg_pacing < 0.3 {
            "节奏偏慢"
        } else if avg_pacing > 0.7 {
            "节奏偏快"
        } else {
            "节奏适中"
        }
    }))
}
