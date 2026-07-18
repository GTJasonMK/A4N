use a4n_core::{
    shooting_script::{ShootingDirectingStatementV1, ShootingEditingModeV1},
    source_spans::source_spans_for_content,
};
use a4n_db::project::{ResolvedChapterSource, ResolvedChapterSourceKind};

use crate::shooting_script::contracts::{
    ChapterScreenplaySkeletonV1, PreparedChapterShootingScriptRequest, ScreenplaySkeletonUnitV1,
};

use super::validation::validate_skeleton;

fn prepared() -> PreparedChapterShootingScriptRequest {
    let content = "甲进入房间。乙从窗边回头。".to_string();
    PreparedChapterShootingScriptRequest {
        source: ResolvedChapterSource {
            source_kind: ResolvedChapterSourceKind::Live,
            requested_project_id: 1,
            root_project_id: 1,
            source_chapter_id: 3,
            source_snapshot_id: None,
            source_snapshot_chapter_id: None,
            chapter_number: 1,
            title: None,
            summary: None,
            analysis_data_json: None,
            content: content.clone(),
            content_sha256: "hash".into(),
            source_revision_key: "revision".into(),
        },
        source_spans: source_spans_for_content(3, 1, &content),
        input_fingerprint_sha256: "fingerprint".into(),
        existing: None,
        skeleton_system_prompt: String::new(),
        layout_unit_system_prompt: String::new(),
        unit_adaptation_system_prompt: String::new(),
        design_unit_system_prompt: String::new(),
    }
}

#[test]
fn skeleton_rejects_duplicate_timeline_positions_before_checkpoint_completion() {
    let prepared = prepared();
    let span_ids = prepared
        .source_spans
        .iter()
        .map(|span| span.id.clone())
        .collect::<Vec<_>>();
    let skeleton = ChapterScreenplaySkeletonV1 {
        schema_version: "chapter_screenplay_skeleton_v1".into(),
        directing_statement: ShootingDirectingStatementV1 {
            central_dramatic_question: "问题".into(),
            protagonist_throughline: "主线".into(),
            audience_experience: "体验".into(),
            viewpoint_strategy: "视点".into(),
            visual_strategy: "视觉".into(),
            spatial_strategy: "空间".into(),
            sound_strategy: "声音".into(),
            rhythm_strategy: "节奏".into(),
        },
        units: vec![
            ScreenplaySkeletonUnitV1 {
                scene_identity: "room-a".into(),
                sequence_index: 1,
                narrative_unit_index: 1,
                timeline_position: 1,
                intercut_group_id: None,
                editing_mode: ShootingEditingModeV1::Continuity,
                source_span_ids: span_ids.clone(),
            },
            ScreenplaySkeletonUnitV1 {
                scene_identity: "room-b".into(),
                sequence_index: 2,
                narrative_unit_index: 2,
                timeline_position: 1,
                intercut_group_id: None,
                editing_mode: ShootingEditingModeV1::Continuity,
                source_span_ids: span_ids,
            },
        ],
    };

    let error = validate_skeleton(&skeleton, &prepared)
        .expect_err("duplicate timeline positions must be rejected");

    assert!(error.to_string().contains("timeline_position=1 重复"));
}
