use super::*;
use std::collections::BTreeMap;

fn snapshot(id: i64) -> project_source_snapshot::Model {
    project_source_snapshot::Model {
        id,
        root_project_id: 1,
        source_version_hash: format!("snapshot-{id}"),
        blueprint_version_id: None,
        chapter_scope_json: json!({ "start": 1, "end": 3 }),
        chapter_hashes_json: json!([]),
        profile_hashes_json: json!([]),
        created_at: chrono::Utc::now().naive_utc(),
    }
}

fn snapshot_chapter(
    id: i64,
    snapshot_id: i64,
    source_chapter_id: i64,
) -> source_snapshot_chapter::Model {
    source_snapshot_chapter::Model {
        id,
        snapshot_id,
        root_project_id: 1,
        source_chapter_id,
        chapter_number: 1,
        title: Some("第一章".to_string()),
        content: "正文".to_string(),
        summary: Some("摘要".to_string()),
        analysis_data_json: None,
        content_hash: "content-hash".to_string(),
        analysis_hash: None,
    }
}

fn family_with_snapshot_chapters(
    snapshot: Option<project_source_snapshot::Model>,
    chapters: BTreeMap<i64, source_snapshot_chapter::Model>,
) -> FamilyLineageContext {
    FamilyLineageContext {
        project_ids: vec![1, 2],
        contexts_by_project_id: BTreeMap::from([(
            2,
            ProjectLineageContext {
                project_id: 2,
                root_project_id: 1,
                source_mode: "derived".to_string(),
                snapshot,
                snapshot_chapters_by_source: chapters,
            },
        )]),
    }
}

#[test]
fn maps_legacy_manga_prompt_types_to_canonical_kinds() {
    assert_eq!(
        manga_generation_kinds(MANGA_PAGE_PROMPT_TYPE),
        Some(("manga_page", "image_prompt"))
    );
    assert_eq!(manga_generation_kinds("unknown"), None);
}

#[test]
fn rejects_removed_video_analysis_prompt_types() {
    assert_eq!(
        video_generation_kinds("removed_video_analysis", "boundary_frames"),
        None
    );
    assert_eq!(
        video_generation_kinds("frame_segment_plan", "first_frame_chain"),
        Some(("video_segment_plan", "video_segment_plan"))
    );
    assert_eq!(
        video_generation_kinds("first_frame_prompt", "first_frame_chain"),
        Some(("video_frame_prompt", "image_prompt"))
    );
    assert_eq!(
        video_generation_kinds("video_prompt", "chat_video"),
        Some(("video_segment", "chat_video_prompt"))
    );
    assert_eq!(
        video_generation_kinds("video_prompt", "first_frame_chain"),
        Some(("video_segment", "video_prompt"))
    );
    assert_eq!(
        video_generation_kinds("provider_prompt", "first_frame_chain"),
        None
    );
}

#[test]
fn normalizes_legacy_statuses_for_canonical_chain() {
    assert_eq!(canonical_spec_status("pending"), Some("ready"));
    assert_eq!(canonical_spec_status("generating"), Some("running"));
    assert_eq!(canonical_spec_status("completed"), Some("completed"));
    assert_eq!(canonical_spec_status("queued"), None);
    assert_eq!(canonical_media_status("generating"), Some("running"));
    assert_eq!(canonical_media_status("failed"), Some("failed"));
    assert_eq!(canonical_media_status("queued"), None);
}

#[test]
fn source_context_skips_generation_spec_when_snapshot_chapter_is_missing() {
    let family = family_with_snapshot_chapters(Some(snapshot(10)), BTreeMap::new());
    let mut summary = AssetLineageBackfillSummary::default();

    let context = source_context_for_row(
        &family,
        2,
        Some(99),
        "chapter_video_prompt",
        7,
        &mut summary,
    );

    assert!(context.is_none());
    assert_eq!(summary.warnings.len(), 1);
    assert!(summary.warnings[0].contains("无法写入 generation_spec"));
    assert!(summary.warnings[0].contains("已跳过"));
    assert!(!summary.warnings[0].contains("仍写入"));
}

#[test]
fn source_context_keeps_complete_source_snapshot_chapter_pair() {
    let chapter = snapshot_chapter(20, 10, 99);
    let family = family_with_snapshot_chapters(
        Some(snapshot(10)),
        BTreeMap::from([(chapter.source_chapter_id, chapter)]),
    );
    let mut summary = AssetLineageBackfillSummary::default();

    let context = source_context_for_row(
        &family,
        2,
        Some(99),
        "chapter_video_prompt",
        7,
        &mut summary,
    )
    .expect("complete source chapter pair should be accepted");

    assert_eq!(context.source_chapter_id, Some(99));
    assert_eq!(context.source_snapshot_chapter_id, Some(20));
    assert!(summary.warnings.is_empty());
}

#[test]
fn media_source_context_skips_media_asset_when_snapshot_chapter_is_missing() {
    let family = family_with_snapshot_chapters(Some(snapshot(10)), BTreeMap::new());
    let project_context = family.contexts_by_project_id.get(&2).unwrap();
    let mut summary = AssetLineageBackfillSummary::default();

    let source = media_source_context_for_row(
        project_context,
        Some(99),
        "generated_video",
        8,
        &mut summary,
    );

    assert!(source.is_none());
    assert_eq!(summary.warnings.len(), 1);
    assert!(summary.warnings[0].contains("无法写入 media_asset"));
    assert!(summary.warnings[0].contains("已跳过"));
}

#[test]
fn media_source_context_keeps_snapshot_for_snapshot_backed_media_without_chapter_source() {
    let family = family_with_snapshot_chapters(Some(snapshot(10)), BTreeMap::new());
    let project_context = family.contexts_by_project_id.get(&2).unwrap();
    let mut summary = AssetLineageBackfillSummary::default();

    let source =
        media_source_context_for_row(project_context, None, "generated_video", 8, &mut summary)
            .expect("snapshot-backed media without chapter source should remain importable");

    assert_eq!(source.source_snapshot_id, Some(10));
    assert_eq!(source.source_chapter_id, None);
    assert_eq!(source.source_snapshot_chapter_id, None);
    assert!(summary.warnings.is_empty());
}

#[test]
fn media_source_context_rejects_snapshot_backed_media_without_snapshot() {
    let family = family_with_snapshot_chapters(None, BTreeMap::new());
    let project_context = family.contexts_by_project_id.get(&2).unwrap();
    let mut summary = AssetLineageBackfillSummary::default();

    let source =
        media_source_context_for_row(project_context, None, "generated_video", 8, &mut summary);

    assert!(source.is_none());
    assert_eq!(summary.warnings.len(), 1);
    assert!(summary.warnings[0].contains("缺少源文本快照"));
    assert!(summary.warnings[0].contains("无法写入 media_asset"));
}

#[test]
fn media_source_context_allows_freeform_media_without_snapshot() {
    let mut family = family_with_snapshot_chapters(None, BTreeMap::new());
    family
        .contexts_by_project_id
        .get_mut(&2)
        .unwrap()
        .source_mode = SOURCE_MODE_FREEFORM.to_string();
    let project_context = family.contexts_by_project_id.get(&2).unwrap();
    let mut summary = AssetLineageBackfillSummary::default();

    let source =
        media_source_context_for_row(project_context, None, "generated_video", 8, &mut summary)
            .expect("freeform media should not require a source snapshot");

    assert_eq!(source.source_snapshot_id, None);
    assert_eq!(source.source_chapter_id, None);
    assert_eq!(source.source_snapshot_chapter_id, None);
    assert!(summary.warnings.is_empty());
}

#[test]
fn video_prompt_artifact_text_keeps_motion_and_negative_sections() {
    let now = chrono::Utc::now().naive_utc();
    let row = chapter_video_prompt::Model {
        id: 1,
        project_id: 2,
        chapter_id: 3,
        style: None,
        prompt_type: "video_prompt".to_string(),
        workflow_mode: "first_frame_chain".to_string(),
        shot_number: 1,
        title: None,
        scene_summary: None,
        visual_prompt: "画面主体".to_string(),
        motion_prompt: Some("从左向右走".to_string()),
        negative_prompt: Some("不要跳切".to_string()),
        duration_secs: 6,
        aspect_ratio: Some("16:9".to_string()),
        reference_image_ids_json: None,
        status: "pending".to_string(),
        generated_video_id: None,
        metadata_json: None,
        created_at: now,
        updated_at: now,
    };

    let text = video_prompt_artifact_text(&row);

    assert!(text.contains("画面主体"));
    assert!(text.contains("【运动提示】"));
    assert!(text.contains("从左向右走"));
    assert!(text.contains("【负面提示】"));
    assert!(text.contains("不要跳切"));
}
