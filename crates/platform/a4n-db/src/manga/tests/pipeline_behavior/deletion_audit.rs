use super::*;
use a4n_db_models::models::generation::{workflow_checkpoint, workflow_run};
use a4n_domain::manga::{MangaModelCall, MangaPreprocessingStep};

async fn failed_workflow_with_checkpoints(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
) -> i64 {
    let handle = crate::DbHandle::new(std::sync::Arc::new(db.clone()));
    let run = crate::workflow::upsert_workflow_run(db, serde_json::from_value(json!({
        "business_key":format!("manga_pipeline:{project_id}:{chapter_id}"), "workflow_kind":"manga_pipeline",
        "project_id":project_id,"chapter_id":chapter_id,"status":"running","can_resume":true
    })).unwrap()).await.unwrap();
    for (order, call) in [
        MangaModelCall::Preprocessing(MangaPreprocessingStep::Facts),
        MangaModelCall::Outline,
        MangaModelCall::PageContent(1),
        MangaModelCall::PageContent(10),
        MangaModelCall::PageAdaptation(1),
        MangaModelCall::Storyboard(1),
        MangaModelCall::Storyboard(10),
        MangaModelCall::ReferenceDiscovery(1),
        MangaModelCall::DrawingPrompt(1),
    ]
    .into_iter()
    .enumerate()
    {
        crate::workflow::save_completed_llm_call_checkpoint(
            &handle,
            crate::workflow::CompletedLlmCallCheckpointInput {
                workflow_run_id: run.id,
                call_key: call.call_key(chapter_id),
                call_order: order as i32,
                input_fingerprint_json: json!({"call":call.call_key(chapter_id)}),
                checkpoint: crate::workflow::CompletedLlmCallCheckpoint {
                    parsed_output: json!({"saved":true}),
                    response_content: "{\"saved\":true}".into(),
                    usage: None,
                    finish_reason: Some("stop".into()),
                    artifacts_json: None,
                },
            },
        )
        .await
        .unwrap();
    }
    crate::workflow::mark_workflow_status(db, run.id, "failed", true, Some("fixture failure"))
        .await
        .unwrap();
    run.id
}

async fn checkpoint_keys(db: &DatabaseConnection, run_id: i64) -> Vec<String> {
    use sea_orm::QuerySelect;
    workflow_checkpoint::Entity::find()
        .select_only()
        .column(workflow_checkpoint::Column::StepKey)
        .filter(workflow_checkpoint::Column::WorkflowRunId.eq(run_id))
        .order_by_asc(workflow_checkpoint::Column::StepKey)
        .into_tuple::<String>()
        .all(db)
        .await
        .unwrap()
}

#[tokio::test]
async fn chapter_stage_deletion_is_atomic_and_preserves_upstream_and_other_projects() {
    let db = setup_db().await;
    let (project_id, chapter_id) = project_and_chapter(&db, "本章当前阶段批量删除").await;
    let (other_project, other_chapter) = project_and_chapter(&db, "保留其他项目").await;
    let run_id = failed_workflow_with_checkpoints(&db, project_id, chapter_id).await;
    let before_keys = checkpoint_keys(&db, run_id).await;
    for (project, chapter) in [(project_id, chapter_id), (other_project, other_chapter)] {
        for (stage, page) in [
            (MangaPipelineArtifactStage::Preprocessing, None),
            (MangaPipelineArtifactStage::PageSlots, None),
            (MangaPipelineArtifactStage::PageContent, Some(1)),
            (MangaPipelineArtifactStage::PageContent, Some(10)),
            (MangaPipelineArtifactStage::Storyboard, Some(1)),
            (MangaPipelineArtifactStage::Storyboard, Some(10)),
            (MangaPipelineArtifactStage::ReferenceDiscovery, Some(1)),
            (MangaPipelineArtifactStage::DrawingPrompt, Some(10)),
            (MangaPipelineArtifactStage::Image, Some(10)),
        ] {
            upsert_manga_pipeline_artifact(
                &db,
                MangaPipelineArtifactDraft {
                    project_id: project,
                    chapter_id: chapter,
                    stage,
                    page_number: page,
                    input_fingerprint: "original".into(),
                    payload: json!({"fixture": stage.key()}),
                    stale: false,
                },
                fixed_time(1),
            )
            .await
            .unwrap();
        }
    }
    let other_before = manga_pipeline_artifacts_by_chapter(&db, other_project, other_chapter)
        .await
        .unwrap();
    let stage = MangaPipelineArtifactStage::Storyboard;
    for (stage, page) in [
        (stage, Some(0)),
        (stage, Some(u32::MAX)),
        (MangaPipelineArtifactStage::Preprocessing, Some(1)),
    ] {
        assert!(
            manga_pipeline_artifact_deletion_plan(&db, project_id, chapter_id, stage, page)
                .await
                .is_err()
        );
    }
    let preview = manga_pipeline_artifact_deletion_plan(&db, project_id, chapter_id, stage, None)
        .await
        .unwrap();
    assert_eq!(preview.artifacts.len(), 5);
    upsert_manga_pipeline_artifact(
        &db,
        MangaPipelineArtifactDraft {
            project_id,
            chapter_id,
            stage,
            page_number: Some(10),
            input_fingerprint: "concurrent-edit".into(),
            payload: json!({"fixture": "edited during confirmation"}),
            stale: false,
        },
        fixed_time(2),
    )
    .await
    .unwrap();
    let before_rejection = manga_pipeline_artifacts_by_chapter(&db, project_id, chapter_id)
        .await
        .unwrap();
    assert!(
        delete_manga_pipeline_artifact(
            &db,
            project_id,
            chapter_id,
            stage,
            None,
            &preview.fingerprint,
            &[],
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("范围已变化")
    );
    assert_eq!(
        manga_pipeline_artifacts_by_chapter(&db, project_id, chapter_id)
            .await
            .unwrap(),
        before_rejection
    );
    assert_eq!(checkpoint_keys(&db, run_id).await, before_keys);
    let preview = manga_pipeline_artifact_deletion_plan(&db, project_id, chapter_id, stage, None)
        .await
        .unwrap();
    delete_manga_pipeline_artifact(
        &db,
        project_id,
        chapter_id,
        stage,
        None,
        &preview.fingerprint,
        &[],
    )
    .await
    .unwrap();
    let retained = manga_pipeline_artifacts_by_chapter(&db, project_id, chapter_id)
        .await
        .unwrap();
    assert_eq!(retained.len(), 4);
    assert!(retained.iter().all(|artifact| matches!(
        artifact.stage,
        MangaPipelineArtifactStage::Preprocessing
            | MangaPipelineArtifactStage::PageSlots
            | MangaPipelineArtifactStage::PageContent
    )));
    let keys = checkpoint_keys(&db, run_id).await;
    assert!(keys.contains(&MangaModelCall::PageContent(10).call_key(chapter_id)));
    assert!(!keys.contains(&MangaModelCall::Storyboard(1).call_key(chapter_id)));
    assert!(!keys.contains(&MangaModelCall::Storyboard(10).call_key(chapter_id)));
    assert!(!keys.contains(&MangaModelCall::DrawingPrompt(1).call_key(chapter_id)));
    assert_eq!(
        manga_pipeline_artifacts_by_chapter(&db, other_project, other_chapter)
            .await
            .unwrap(),
        other_before
    );
}

async fn resume_workflow(db: &DatabaseConnection, run_id: i64) {
    let run = workflow_run::Entity::find_by_id(run_id)
        .one(db)
        .await
        .unwrap()
        .unwrap();
    let resumed = crate::workflow::resume_or_upsert_workflow_run(
        db,
        serde_json::from_value(json!({
            "business_key":run.business_key, "workflow_kind":run.workflow_kind,
            "project_id":run.project_id, "chapter_id":run.chapter_id, "mode":run.mode,
            "status":"running", "can_resume":false
        }))
        .unwrap(),
    )
    .await
    .unwrap();
    assert_eq!(resumed.id, run_id);
}

async fn failed_image_workflow(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
    page: u32,
    kind: &str,
) -> i64 {
    let run = crate::workflow::upsert_workflow_run(db, serde_json::from_value(json!({
        "business_key":format!("{kind}:{project_id}:{chapter_id}:{page}"), "workflow_kind":kind,
        "project_id":project_id, "chapter_id":chapter_id, "mode":"manga_page", "status":"running", "can_resume":false
    })).unwrap()).await.unwrap();
    let handle = crate::DbHandle::new(std::sync::Arc::new(db.clone()));
    crate::workflow::upsert_workflow_checkpoint(&handle, crate::workflow::UpsertWorkflowCheckpointInput {
        workflow_run_id:run.id, step_key:"image_generation.image_page".into(), step_order:1, status:"completed".into(),
        input_fingerprint_json:Some(json!({"image_type":"manga_page", "chapter_id":chapter_id, "panel_id":format!("page{page}")})),
        input_fingerprint_sha256:None, checkpoint_json:Some(json!({"generated_image_id":1})),
        artifacts_json:None, generation_run_ids_json:None, error_message:None, completed:true,
    }).await.unwrap();
    crate::workflow::mark_workflow_status(db, run.id, "failed", true, Some("fixture failure"))
        .await
        .unwrap();
    run.id
}

#[tokio::test]
async fn image_deletion_blocks_independent_generation_and_clears_only_matching_image_checkpoints() {
    let db = setup_db().await;
    let (project_id, chapter_id) = project_and_chapter(&db, "单页生图的运行保护与恢复").await;
    let owned = failed_image_workflow(&db, project_id, chapter_id, 1, "image_generation").await;
    let other_page =
        failed_image_workflow(&db, project_id, chapter_id, 10, "image_generation").await;
    let attached = failed_image_workflow(&db, project_id, chapter_id, 1, "manga_pipeline").await;
    upsert_manga_pipeline_artifact(
        &db,
        MangaPipelineArtifactDraft {
            project_id,
            chapter_id,
            stage: MangaPipelineArtifactStage::Image,
            page_number: Some(1),
            input_fingerprint: "input".into(),
            payload: json!({"image":1}),
            stale: false,
        },
        fixed_time(1),
    )
    .await
    .unwrap();
    let preview = manga_pipeline_artifact_deletion_plan(
        &db,
        project_id,
        chapter_id,
        MangaPipelineArtifactStage::Image,
        Some(1),
    )
    .await
    .unwrap();
    resume_workflow(&db, owned).await;
    assert!(
        delete_manga_pipeline_artifact(
            &db,
            project_id,
            chapter_id,
            MangaPipelineArtifactStage::Image,
            Some(1),
            &preview.fingerprint,
            &[]
        )
        .await
        .unwrap_err()
        .to_string()
        .contains("正在运行")
    );
    assert_eq!(checkpoint_keys(&db, owned).await.len(), 1);
    crate::workflow::mark_workflow_status(&db, owned, "failed", true, Some("fixture failure"))
        .await
        .unwrap();
    delete_manga_pipeline_artifact(
        &db,
        project_id,
        chapter_id,
        MangaPipelineArtifactStage::Image,
        Some(1),
        &preview.fingerprint,
        &[],
    )
    .await
    .unwrap();
    for id in [owned, attached] {
        assert!(checkpoint_keys(&db, id).await.is_empty());
        assert!(
            !workflow_run::Entity::find_by_id(id)
                .one(&db)
                .await
                .unwrap()
                .unwrap()
                .can_resume
        );
    }
    assert_eq!(checkpoint_keys(&db, other_page).await.len(), 1);
    assert!(
        workflow_run::Entity::find_by_id(other_page)
            .one(&db)
            .await
            .unwrap()
            .unwrap()
            .can_resume
    );
}

#[tokio::test]
async fn deleting_a_stage_removes_only_its_checkpoints_then_exhausted_runs_cannot_resume() {
    let db = setup_db().await;
    let (project_id, chapter_id) = project_and_chapter(&db, "阶段删除检查点隔离").await;
    let (other_project, _) = project_and_chapter(&db, "其他项目检查点").await;
    let run_id = failed_workflow_with_checkpoints(&db, project_id, chapter_id).await;
    let other_run = failed_workflow_with_checkpoints(&db, other_project, chapter_id).await;
    let other_before = checkpoint_keys(&db, other_run).await;
    for stage in [
        MangaPipelineArtifactStage::Preprocessing,
        MangaPipelineArtifactStage::Storyboard,
        MangaPipelineArtifactStage::ReferenceDiscovery,
        MangaPipelineArtifactStage::DrawingPrompt,
    ] {
        upsert_manga_pipeline_artifact(
            &db,
            MangaPipelineArtifactDraft {
                project_id,
                chapter_id,
                stage,
                page_number: stage.page_scoped().then_some(1),
                input_fingerprint: stage.key().into(),
                payload: json!({"fixture":stage.key()}),
                stale: false,
            },
            fixed_time(1),
        )
        .await
        .unwrap();
    }
    for (stage, page) in [
        (MangaPipelineArtifactStage::Storyboard, Some(1)),
        (MangaPipelineArtifactStage::Preprocessing, None),
    ] {
        let plan = manga_pipeline_artifact_deletion_plan(&db, project_id, chapter_id, stage, page)
            .await
            .unwrap();
        delete_manga_pipeline_artifact(
            &db,
            project_id,
            chapter_id,
            stage,
            page,
            &plan.fingerprint,
            &[],
        )
        .await
        .unwrap();
        if page.is_some() {
            let mut expected = vec![
                MangaModelCall::Preprocessing(MangaPreprocessingStep::Facts),
                MangaModelCall::Outline,
                MangaModelCall::PageContent(1),
                MangaModelCall::PageContent(10),
                MangaModelCall::PageAdaptation(1),
                MangaModelCall::Storyboard(10),
            ]
            .into_iter()
            .map(|call| call.call_key(chapter_id))
            .collect::<Vec<_>>();
            expected.sort();
            assert_eq!(checkpoint_keys(&db, run_id).await, expected);
            assert!(
                workflow_run::Entity::find_by_id(run_id)
                    .one(&db)
                    .await
                    .unwrap()
                    .unwrap()
                    .can_resume
            );
        }
    }
    assert!(checkpoint_keys(&db, run_id).await.is_empty());
    assert!(
        !workflow_run::Entity::find_by_id(run_id)
            .one(&db)
            .await
            .unwrap()
            .unwrap()
            .can_resume
    );
    assert_eq!(checkpoint_keys(&db, other_run).await, other_before);
}

#[tokio::test]
async fn stage_deletion_cannot_skip_a_page_projection_or_erase_checkpoints_on_rejection() {
    let db = setup_db().await;
    let (project_id, chapter_id) = project_and_chapter(&db, "删除范围缺少页面投影").await;
    let run_id = failed_workflow_with_checkpoints(&db, project_id, chapter_id).await;
    let before = checkpoint_keys(&db, run_id).await;
    let now = fixed_time(1);
    let prompt = chapter_manga_prompt::ActiveModel {
        project_id: Set(project_id),
        chapter_id: Set(chapter_id),
        page_number: Set(1),
        prompt_type: Set(MANGA_PAGE_PROMPT_TYPE.into()),
        image_prompt: Set("保留已保存提示词".into()),
        layout: Set(Some(page_layout(1))),
        status: Set("pending".into()),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    upsert_manga_pipeline_artifact(
        &db,
        MangaPipelineArtifactDraft {
            project_id,
            chapter_id,
            stage: MangaPipelineArtifactStage::DrawingPrompt,
            page_number: Some(1),
            input_fingerprint: "input".into(),
            payload: json!({"fixture":true}),
            stale: false,
        },
        now,
    )
    .await
    .unwrap();
    let plan = manga_pipeline_artifact_deletion_plan(
        &db,
        project_id,
        chapter_id,
        MangaPipelineArtifactStage::DrawingPrompt,
        Some(1),
    )
    .await
    .unwrap();
    assert!(
        delete_manga_pipeline_artifact(
            &db,
            project_id,
            chapter_id,
            MangaPipelineArtifactStage::DrawingPrompt,
            Some(1),
            &plan.fingerprint,
            &[]
        )
        .await
        .is_err()
    );
    assert_eq!(checkpoint_keys(&db, run_id).await, before);
    assert!(
        manga_pipeline_artifact(
            &db,
            project_id,
            chapter_id,
            MangaPipelineArtifactStage::DrawingPrompt,
            Some(1)
        )
        .await
        .unwrap()
        .is_some()
    );
    assert_eq!(
        chapter_manga_prompt::Entity::find_by_id(prompt.id)
            .one(&db)
            .await
            .unwrap()
            .unwrap()
            .image_prompt,
        "保留已保存提示词"
    );
}

#[tokio::test]
async fn stage_deletion_rejects_changed_contents_and_running_workflows_without_mutation() {
    let db = setup_db().await;
    let (project_id, chapter_id) = project_and_chapter(&db, "删除预览版本与运行锁").await;
    let run_id = failed_workflow_with_checkpoints(&db, project_id, chapter_id).await;
    let keys = checkpoint_keys(&db, run_id).await;
    let draft = MangaPipelineArtifactDraft {
        project_id,
        chapter_id,
        stage: MangaPipelineArtifactStage::DrawingPrompt,
        page_number: Some(1),
        input_fingerprint: "input".into(),
        payload: json!({"version":1}),
        stale: false,
    };
    let original = upsert_manga_pipeline_artifact(&db, draft.clone(), fixed_time(1))
        .await
        .unwrap();
    let preview = manga_pipeline_artifact_deletion_plan(
        &db,
        project_id,
        chapter_id,
        draft.stage,
        draft.page_number,
    )
    .await
    .unwrap();
    let updated = upsert_manga_pipeline_artifact(
        &db,
        MangaPipelineArtifactDraft {
            payload: json!({"version":2}),
            ..draft.clone()
        },
        fixed_time(2),
    )
    .await
    .unwrap();
    assert_eq!(original.id, updated.id);
    let error = delete_manga_pipeline_artifact(
        &db,
        project_id,
        chapter_id,
        draft.stage,
        draft.page_number,
        &preview.fingerprint,
        &[],
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("范围已变化"), "{error}");

    let preview = manga_pipeline_artifact_deletion_plan(
        &db,
        project_id,
        chapter_id,
        draft.stage,
        draft.page_number,
    )
    .await
    .unwrap();
    resume_workflow(&db, run_id).await;
    let error = delete_manga_pipeline_artifact(
        &db,
        project_id,
        chapter_id,
        draft.stage,
        draft.page_number,
        &preview.fingerprint,
        &[],
    )
    .await
    .unwrap_err();
    assert!(error.to_string().contains("正在运行"), "{error}");
    assert_eq!(checkpoint_keys(&db, run_id).await, keys);
    assert_eq!(
        manga_pipeline_artifact(&db, project_id, chapter_id, draft.stage, draft.page_number)
            .await
            .unwrap()
            .unwrap(),
        updated
    );
}

#[tokio::test]
async fn stage_deletion_clears_half_published_outputs_and_their_summary_fields() {
    let db = setup_db().await;
    let (project_id, chapter_id) = project_and_chapter(&db, "半发布阶段的投影清理").await;
    let previous = page_layout(1);
    let mut drafts = [prompt_draft(
        chapter_id,
        project_id,
        1,
        "已发布但没有产物行的提示词",
    )];
    upgrade_to_pipeline_layouts(&mut drafts);
    let mut layout = drafts[0].layout.take().unwrap();
    layout["pipeline"]["storyboard"] = previous["page_plan"].clone();
    layout["pipeline"]["drawing_prompt"] = previous["drawing_prompt"].clone();
    layout["pipeline"]["stage_fingerprints"]["storyboard"] = json!("storyboard-input");
    layout["pipeline"]["stage_fingerprints"]["drawing_prompt"] = json!("drawing-input");
    layout["reference_bindings"] =
        json!([{"character_name":"主角","bound_asset":null,"user_added_assets":[]}]);
    let prompt = chapter_manga_prompt::ActiveModel {
        project_id: Set(project_id),
        chapter_id: Set(chapter_id),
        page_number: Set(1),
        prompt_type: Set(MANGA_PAGE_PROMPT_TYPE.into()),
        image_prompt: Set("已发布但没有产物行的提示词".into()),
        dialogue_text: Set(Some("旧对白".into())),
        characters_in_frame: Set(Some(json!(["主角"]))),
        layout: Set(Some(layout.clone())),
        status: Set("pending".into()),
        created_at: Set(fixed_time(1)),
        updated_at: Set(fixed_time(1)),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let stage = MangaPipelineArtifactStage::Storyboard;
    upsert_manga_pipeline_artifact(
        &db,
        MangaPipelineArtifactDraft {
            project_id,
            chapter_id,
            stage,
            page_number: Some(1),
            input_fingerprint: "input".into(),
            payload: previous["page_plan"].clone(),
            stale: false,
        },
        fixed_time(1),
    )
    .await
    .unwrap();
    let preview =
        manga_pipeline_artifact_deletion_plan(&db, project_id, chapter_id, stage, Some(1))
            .await
            .unwrap();
    assert_eq!(preview.artifacts.len(), 1);
    let mut cleared: a4n_domain::manga::MangaPageGenerationLayoutV4 =
        serde_json::from_value(layout.clone()).unwrap();
    cleared.clear_deleted_stage(stage);
    cleared.validate(1).unwrap();
    delete_manga_pipeline_artifact(
        &db,
        project_id,
        chapter_id,
        stage,
        Some(1),
        &preview.fingerprint,
        &[MangaPipelineArtifactProjectionReset {
            page_prompt_id: prompt.id,
            page_number: 1,
            expected_layout: layout,
            expected_generated_image_id: None,
            layout: serde_json::to_value(&cleared).unwrap(),
            image_prompt: String::new(),
            clear_generated_image: true,
        }],
    )
    .await
    .unwrap();
    let row = chapter_manga_prompt::Entity::find_by_id(prompt.id)
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.layout, Some(serde_json::to_value(&cleared).unwrap()));
    assert!(row.image_prompt.is_empty());
    assert!(row.dialogue_text.is_none());
    assert!(row.characters_in_frame.is_none());
    assert!(cleared.pipeline.page_content.is_some());
    assert!(cleared.pipeline.storyboard.is_none());
    assert!(cleared.pipeline.drawing_prompt.is_none());
    assert!(cleared.reference_bindings.is_empty());
    assert_eq!(cleared.pipeline.stage_fingerprints.len(), 1);
}
