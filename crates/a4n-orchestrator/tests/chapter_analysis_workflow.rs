use a4n_core::{
    models::{chapter::chapter, generation::workflow_run},
    source_spans::source_text_hash as source_content_sha256,
};
use a4n_orchestrator::{
    chapter_analysis::{
        ChapterAnalysisOutputError, ChapterAnalysisResumeBlockReason,
        chapter_analysis_foreshadowing_drafts, chapter_analysis_phase_fingerprint,
        chapter_analysis_resume_block_reason, chapter_analysis_run_input_matches,
        chapter_analysis_run_uses_replacement_commit, chapter_analysis_workflow_business_key,
        chapter_analysis_workflow_fingerprint, chapter_analysis_workflow_run_input,
        require_completed_chapter_analysis_phase,
    },
    workflow::{WORKFLOW_STATUS_COMPLETED, WORKFLOW_STATUS_FAILED, WORKFLOW_STATUS_RUNNING},
};
use a4n_test_support::workflow::WorkflowRunModelBuilder;
use serde_json::{Value, json};

fn chapter_row(chapter_number: i32, content: &str, status: &str, analyzed: bool) -> chapter::Model {
    let now = chrono::Utc::now().naive_utc();
    chapter::Model {
        id: i64::from(chapter_number),
        project_id: 1,
        outline_id: None,
        chapter_number,
        title: None,
        content: content.to_string(),
        word_count: content.chars().count() as i32,
        status: status.to_string(),
        summary: None,
        style_tags: None,
        analysis_data: analyzed.then(valid_analysis_data),
        created_at: now,
        updated_at: now,
    }
}

fn valid_analysis_data() -> Value {
    json!({
        "metadata": {
            "characters": [],
            "locations": [],
            "items": [],
            "tags": [],
            "tone": null,
            "timeline_marker": null
        },
        "summaries": {
            "compressed": "章节摘要",
            "one_line": "一句话摘要",
            "keywords": []
        },
        "character_states": {},
        "foreshadowing": {
            "planted": [],
            "resolved": [],
            "tensions": []
        },
        "key_events": []
    })
}

#[test]
fn workflow_input_uses_stable_business_key_and_content_fingerprint() {
    let chapter = chapter_row(3, "正文", chapter::STATUS_DRAFT, true);
    let input = chapter_analysis_workflow_run_input(
        &chapter,
        "chapter_analysis.derived_sync",
        Some("chapter-analysis-task-3"),
    );

    assert_eq!(input.business_key, "chapter_analysis:1:3");
    assert_eq!(input.workflow_kind, "chapter_analysis");
    assert_eq!(input.chapter_id, Some(3));
    assert_eq!(
        input.current_step_key.as_deref(),
        Some("chapter_analysis.derived_sync")
    );
    assert_eq!(input.status, WORKFLOW_STATUS_RUNNING);
    assert_eq!(
        input.active_task_id.as_deref(),
        Some("chapter-analysis-task-3")
    );
    assert!(!input.can_resume);
    let fingerprint = input
        .input_fingerprint_json
        .expect("workflow input should include fingerprint");
    assert_eq!(fingerprint["workflow_kind"], "chapter_analysis");
    assert_eq!(fingerprint["schema_version"], 2);
    assert!(fingerprint["analysis_revision"].as_str().is_some());
    assert_eq!(fingerprint["source_fingerprint"]["project_id"], 1);
    assert_eq!(fingerprint["source_fingerprint"]["chapter_id"], 3);
    assert_eq!(
        fingerprint["source_fingerprint"]["content_sha256"],
        source_content_sha256("正文")
    );
    assert!(chapter_analysis_run_input_matches(
        &Some(fingerprint),
        &chapter
    ));
}

#[test]
fn resume_input_matches_only_same_chapter_identity() {
    let chapter = chapter_row(3, "正文", chapter::STATUS_DRAFT, false);
    let fingerprint = Some(chapter_analysis_workflow_fingerprint(&chapter));

    assert!(chapter_analysis_run_input_matches(&fingerprint, &chapter));

    let changed_content = chapter_row(3, "正文已修改", chapter::STATUS_DRAFT, false);
    assert!(!chapter_analysis_run_input_matches(
        &fingerprint,
        &changed_content
    ));

    let mut changed_title = chapter.clone();
    changed_title.title = Some("新标题".to_string());
    assert!(!chapter_analysis_run_input_matches(
        &fingerprint,
        &changed_title
    ));

    let mut changed_project = chapter.clone();
    changed_project.project_id = 2;
    assert!(!chapter_analysis_run_input_matches(
        &fingerprint,
        &changed_project
    ));
    assert!(!chapter_analysis_run_input_matches(&None, &chapter));

    let mut changed_version = chapter.clone();
    changed_version.updated_at += chrono::Duration::seconds(1);
    assert!(!chapter_analysis_run_input_matches(
        &fingerprint,
        &changed_version
    ));
}

#[test]
fn repeated_analysis_uses_distinct_revisions_for_the_same_source() {
    let chapter = chapter_row(3, "正文", chapter::STATUS_DRAFT, true);
    let first = chapter_analysis_workflow_run_input(&chapter, "chapter_analysis.facts", None)
        .input_fingerprint_json
        .expect("first analysis fingerprint should exist");
    let second = chapter_analysis_workflow_run_input(&chapter, "chapter_analysis.facts", None)
        .input_fingerprint_json
        .expect("second analysis fingerprint should exist");

    assert_ne!(first, second);
    assert!(chapter_analysis_run_uses_replacement_commit(&Some(
        first.clone()
    )));
    assert!(chapter_analysis_run_input_matches(&Some(first), &chapter));
    assert!(chapter_analysis_run_input_matches(&Some(second), &chapter));
    assert!(!chapter_analysis_run_uses_replacement_commit(&Some(
        chapter_analysis_workflow_fingerprint(&chapter)
    )));
}

#[test]
fn completed_workflow_blocks_manual_resume_sync() {
    let chapter = chapter_row(3, "正文", chapter::STATUS_DRAFT, true);
    let run = workflow_run_row(&chapter, WORKFLOW_STATUS_COMPLETED, false, None);

    let reason = chapter_analysis_resume_block_reason(&run, &chapter)
        .expect("completed matching run should block resume sync");

    assert_eq!(reason, ChapterAnalysisResumeBlockReason::Completed);
    assert!(reason.to_string().contains("已同步完成"));
}

#[test]
fn running_workflow_blocks_manual_resume_sync() {
    let chapter = chapter_row(3, "正文", chapter::STATUS_DRAFT, true);
    let run = workflow_run_row(&chapter, WORKFLOW_STATUS_RUNNING, true, None);

    let reason = chapter_analysis_resume_block_reason(&run, &chapter)
        .expect("running matching run should block resume sync");

    assert_eq!(reason, ChapterAnalysisResumeBlockReason::Running);
    assert!(reason.to_string().contains("正在同步中"));
}

#[test]
fn resumable_failed_workflow_allows_manual_resume_sync() {
    let chapter = chapter_row(3, "正文", chapter::STATUS_DRAFT, true);
    let run = workflow_run_row(&chapter, WORKFLOW_STATUS_FAILED, true, None);

    assert!(chapter_analysis_resume_block_reason(&run, &chapter).is_none());
}

#[test]
fn stale_workflow_does_not_block_manual_resume_sync() {
    let chapter = chapter_row(3, "正文", chapter::STATUS_DRAFT, true);
    let mut run = workflow_run_row(&chapter, WORKFLOW_STATUS_COMPLETED, false, None);
    run.input_fingerprint_json = Some(chapter_analysis_workflow_fingerprint(&chapter_row(
        3,
        "旧正文",
        chapter::STATUS_DRAFT,
        true,
    )));

    assert!(chapter_analysis_resume_block_reason(&run, &chapter).is_none());
}

#[test]
fn non_resumable_workflow_preserves_persisted_reason() {
    let chapter = chapter_row(3, "正文", chapter::STATUS_DRAFT, true);
    let run = workflow_run_row(
        &chapter,
        WORKFLOW_STATUS_FAILED,
        false,
        Some("断点版本不兼容"),
    );

    let reason = chapter_analysis_resume_block_reason(&run, &chapter)
        .expect("non-resumable matching run should block resume sync");

    assert_eq!(
        reason,
        ChapterAnalysisResumeBlockReason::NotResumable("断点版本不兼容".to_string())
    );
    assert_eq!(reason.to_string(), "断点版本不兼容");
}

#[test]
fn phase_fingerprint_tracks_saved_analysis_data() {
    let chapter = chapter_row(3, "正文", chapter::STATUS_DRAFT, true);
    let mut edited = chapter.clone();
    edited.analysis_data = Some(json!({
        "metadata": {
            "characters": ["新角色"],
            "locations": [],
            "items": [],
            "tags": [],
            "tone": null,
            "timeline_marker": null
        },
        "summaries": {
            "compressed": "编辑后的章节摘要",
            "one_line": "一句话摘要",
            "keywords": []
        },
        "character_states": {},
        "foreshadowing": {
            "planted": [],
            "resolved": [],
            "tensions": []
        },
        "key_events": []
    }));

    assert_ne!(
        chapter_analysis_phase_fingerprint(&chapter, "chapter_analysis.object_discovery"),
        chapter_analysis_phase_fingerprint(&edited, "chapter_analysis.object_discovery")
    );
}

#[test]
fn completed_phase_contract_requires_true_marker() {
    require_completed_chapter_analysis_phase(
        &json!({"completed": true}),
        "chapter_analysis.object_discovery",
    )
    .expect("completed=true should satisfy phase contract");

    let error = require_completed_chapter_analysis_phase(
        &json!({"completed": false}),
        "chapter_analysis.object_discovery",
    )
    .expect_err("missing completed=true must fail explicitly");
    assert_eq!(
        error.to_string(),
        "chapter_analysis.object_discovery 已完成 checkpoint 缺少 completed=true"
    );
}

#[test]
fn foreshadowing_drafts_preserve_stable_identity_and_importance() {
    let mut analysis = valid_analysis_data();
    analysis["foreshadowing"]["planted"] = json!([{
        "description": "暗号指向后续叛徒身份",
        "original_text": "墙上的半枚印记",
        "category": "plot_twist",
        "priority": "high",
        "related_entities": ["林岚"]
    }]);

    let drafts = chapter_analysis_foreshadowing_drafts(3, &analysis)
        .expect("valid saved analysis should produce foreshadowing drafts");

    assert_eq!(drafts.len(), 1);
    assert_eq!(
        drafts[0].description,
        "暗号指向后续叛徒身份 [plot_twist] 原文: 墙上的半枚印记"
    );
    assert_eq!(drafts[0].importance, 5);
    assert_eq!(drafts[0].related_characters, Some(json!(["林岚"])));
}

#[test]
fn foreshadowing_drafts_reject_duplicate_stable_identity() {
    let mut analysis = valid_analysis_data();
    let item = json!({
        "description": "重复伏笔",
        "original_text": "同一处原文",
        "category": "mystery",
        "priority": "medium",
        "related_entities": []
    });
    analysis["foreshadowing"]["planted"] = json!([item.clone(), item]);

    let error = chapter_analysis_foreshadowing_drafts(3, &analysis)
        .expect_err("duplicate stable identity must fail before persistence");

    assert!(matches!(
        error,
        ChapterAnalysisOutputError::DuplicateForeshadowing {
            chapter_id: 3,
            description
        } if description == "重复伏笔 [mystery] 原文: 同一处原文"
    ));
}

#[test]
fn foreshadowing_drafts_keep_invalid_priority_structured() {
    let mut analysis = valid_analysis_data();
    analysis["foreshadowing"]["planted"] = json!([{
        "description": "优先级异常伏笔",
        "original_text": "异常原文",
        "category": "mystery",
        "priority": "urgent",
        "related_entities": []
    }]);

    let error = chapter_analysis_foreshadowing_drafts(3, &analysis)
        .expect_err("unknown priority must fail before persistence");

    assert!(matches!(
        &error,
        ChapterAnalysisOutputError::InvalidForeshadowingPriority {
            chapter_id: 3,
            description,
            ..
        } if description == "优先级异常伏笔 [mystery] 原文: 异常原文"
    ));
    assert!(error.to_string().contains("伏笔优先级非法: urgent"));
}

fn workflow_run_row(
    chapter: &chapter::Model,
    status: &str,
    can_resume: bool,
    resume_block_reason: Option<&str>,
) -> workflow_run::Model {
    let now = chrono::Utc::now().naive_utc();
    WorkflowRunModelBuilder::new(
        1,
        chapter_analysis_workflow_business_key(chapter.project_id, chapter.id),
        "chapter_analysis",
        status,
        now,
    )
    .with_project_scope(Some(chapter.project_id), None)
    .with_scope("chapter", chapter.id)
    .with_chapter_id(chapter.id)
    .with_input_fingerprint(chapter_analysis_workflow_fingerprint(chapter))
    .with_current_step("chapter_analysis.derived_sync")
    .with_resumability(can_resume, resume_block_reason.map(str::to_string))
    .with_completed_at((status == WORKFLOW_STATUS_COMPLETED).then_some(now))
    .build()
}
