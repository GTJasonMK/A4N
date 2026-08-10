use super::super::story_records::{ingest_outlines_inner, ingest_part_outlines_inner};
use super::super::sync::ExpectedSyncStats as IngestStats;
use super::errors::rag_ingest_error_message;
use crate::state::AppState;
use a4n_core::models::chapter::{chapter_outline, part_outline};

pub async fn sync_outline_rag_records(
    state: &AppState,
    project_id: i64,
) -> Result<serde_json::Value, String> {
    let trace_id = crate::sidecar_clients::rag_ingest_operation_trace_id(project_id, None)?;
    crate::sidecar_clients::scope_rag_operation_trace(
        trace_id,
        sync_outline_rag_records_inner(state, project_id),
    )
    .await
}

async fn sync_outline_rag_records_inner(
    state: &AppState,
    project_id: i64,
) -> Result<serde_json::Value, String> {
    let (expected_outlines, expected_parts) = outline_rag_sources(state, project_id).await?;
    let mut total_stats = IngestStats::default();

    let outline_stats = ingest_outlines_inner(state, project_id, None).await?;
    let outline_count = outline_stats.item_count;
    total_stats.merge(outline_stats)?;

    let part_outline_stats = ingest_part_outlines_inner(state, project_id, None).await?;
    let part_outline_count = part_outline_stats.item_count;
    total_stats.merge(part_outline_stats)?;

    if !total_stats.errors.is_empty() {
        return Err(rag_ingest_error_message(
            "大纲 RAG 同步",
            &total_stats.errors,
        ));
    }
    let (current_outlines, current_parts) = outline_rag_sources(state, project_id).await?;
    ensure_outline_rag_sources_unchanged(
        &expected_outlines,
        &current_outlines,
        &expected_parts,
        &current_parts,
    )?;

    Ok(serde_json::json!({
        "outline_count": outline_count,
        "part_outline_count": part_outline_count,
        "chunk_count": total_stats.chunk_count,
        "added_count": total_stats.added_count,
        "updated_count": total_stats.updated_count,
        "skipped_count": total_stats.skipped_count,
        "deleted_count": total_stats.deleted_count,
    }))
}

async fn outline_rag_sources(
    state: &AppState,
    project_id: i64,
) -> Result<(Vec<chapter_outline::Model>, Vec<part_outline::Model>), String> {
    let mut outlines = state
        .rag()
        .chapter_outlines_for_rag_ingest(project_id)
        .await
        .map_err(|error| error.to_string())?;
    let mut parts = state
        .rag()
        .part_outlines_for_rag_ingest(project_id)
        .await
        .map_err(|error| error.to_string())?;
    sort_outline_rag_sources(&mut outlines, &mut parts);
    Ok((outlines, parts))
}

fn sort_outline_rag_sources(
    outlines: &mut [chapter_outline::Model],
    parts: &mut [part_outline::Model],
) {
    outlines.sort_by_key(|outline| (outline.chapter_number, outline.id));
    parts.sort_by_key(|part| (part.part_number, part.id));
}

fn ensure_outline_rag_sources_unchanged(
    expected_outlines: &[chapter_outline::Model],
    current_outlines: &[chapter_outline::Model],
    expected_parts: &[part_outline::Model],
    current_parts: &[part_outline::Model],
) -> Result<(), String> {
    if expected_outlines == current_outlines && expected_parts == current_parts {
        return Ok(());
    }
    Err("大纲正式资料在 RAG 同步期间发生变化，请重新同步资料索引".to_string())
}

#[cfg(test)]
mod tests {
    use a4n_core::models::chapter::{chapter_outline, part_outline};
    use chrono::NaiveDateTime;

    use super::{ensure_outline_rag_sources_unchanged, sort_outline_rag_sources};

    fn outline(id: i64, chapter_number: i32, summary: &str) -> chapter_outline::Model {
        chapter_outline::Model {
            id,
            project_id: 2,
            part_id: None,
            chapter_number,
            title: Some(format!("第 {chapter_number} 章")),
            summary: Some(summary.to_string()),
            beats: None,
            key_scenes: None,
            pacing_score: None,
            foreshadowing_setup: None,
            foreshadowing_payoff: None,
            characters_involved: None,
            word_count_target: None,
            status: chapter_outline::STATUS_COMPLETED.to_string(),
            created_at: NaiveDateTime::MIN,
            updated_at: NaiveDateTime::MIN,
        }
    }

    fn part(id: i64, part_number: i32) -> part_outline::Model {
        part_outline::Model {
            id,
            project_id: 2,
            part_number,
            title: format!("第 {part_number} 部"),
            start_chapter: part_number,
            end_chapter: part_number,
            summary: None,
            theme: None,
            key_events: None,
            character_focus: None,
            created_at: NaiveDateTime::MIN,
            updated_at: NaiveDateTime::MIN,
        }
    }

    #[test]
    fn outline_rag_sync_compares_ordered_current_formal_sources() {
        let mut expected_outlines = vec![outline(2, 2, "二"), outline(1, 1, "一")];
        let mut expected_parts = vec![part(2, 2), part(1, 1)];
        let mut current_outlines = vec![outline(1, 1, "一"), outline(2, 2, "二")];
        let mut current_parts = vec![part(1, 1), part(2, 2)];
        sort_outline_rag_sources(&mut expected_outlines, &mut expected_parts);
        sort_outline_rag_sources(&mut current_outlines, &mut current_parts);

        ensure_outline_rag_sources_unchanged(
            &expected_outlines,
            &current_outlines,
            &expected_parts,
            &current_parts,
        )
        .expect("same formal sources in different query order must match");

        current_outlines[0].summary = Some("已修改".to_string());
        let error = ensure_outline_rag_sources_unchanged(
            &expected_outlines,
            &current_outlines,
            &expected_parts,
            &current_parts,
        )
        .expect_err("changed formal outline must reject stale sync");
        assert!(error.contains("正式资料"));
    }
}
