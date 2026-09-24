use a4n_db::{
    chapter_visual_plan::chapter_visual_plans_for_sources,
    shooting_script::chapter_shooting_scripts_for_sources,
};
use a4n_domain::{chapter::ResolvedChapterSource, source_text_hash};

fn source(source_chapter_id: i64) -> ResolvedChapterSource {
    let content = format!("chapter content {source_chapter_id}");
    let content_sha256 = source_text_hash(&content);
    ResolvedChapterSource {
        requested_project_id: 10,
        root_project_id: 10,
        source_chapter_id,
        source_snapshot_id: Some(20),
        source_snapshot_chapter_id: Some(source_chapter_id + 100),
        chapter_number: source_chapter_id as i32,
        title: None,
        summary: None,
        analysis_data_json: None,
        content,
        content_sha256: content_sha256.clone(),
        source_revision_key: format!("snapshot:20:chapter:{}:{content_sha256}", source_chapter_id),
    }
}

#[tokio::test]
async fn galgame_batch_reads_preserve_input_slots() {
    let sources = vec![source(2), source(1)];
    let db = a4n_migration::test_support::connect_test_database().await;
    let scripts = chapter_shooting_scripts_for_sources(&db, &sources)
        .await
        .expect("empty shooting-script batch must succeed");
    assert_eq!(scripts, vec![None, None]);

    let plans = chapter_visual_plans_for_sources(&db, &sources)
        .await
        .expect("empty visual-plan batch must succeed");
    assert_eq!(plans.len(), 2);
    assert!(plans.iter().all(|plans| plans.confirmed.is_none()));
}
