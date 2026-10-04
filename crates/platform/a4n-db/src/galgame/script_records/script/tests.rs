use super::persist_galgame_script_record;
use crate::galgame::{GalgameScriptRecordError, PersistGalgameScriptRecordInput};
use a4n_db_models::models::{
    chapter::chapter,
    galgame::{galgame_choice, galgame_line, galgame_scene},
    project::source_snapshot_chapter,
};
use a4n_domain::galgame::{
    GALGAME_STORY_GRAPH_SCHEMA_VERSION, StoryBranchGroup, StoryBranchOption, StoryEdge,
    StoryEdgeKind, StoryGraph, StoryNode, StoryNodeKind,
};
use a4n_domain::project::ProjectKind;
use chrono::Utc;
use sea_orm::{
    ActiveModelTrait, ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder, Set,
};
use serde_json::json;

fn graph_node(id: &str, kind: StoryNodeKind) -> StoryNode {
    StoryNode {
        id: id.into(),
        kind,
        title: id.into(),
        summary: String::new(),
        primary_source: None,
        source_refs: Vec::new(),
        lines: Vec::new(),
        state_before: Default::default(),
        state_after: Default::default(),
        state_operations: Vec::new(),
        media_intents: Vec::new(),
        layout_x: 0.0,
        layout_y: 0.0,
    }
}

fn graph_edge(
    id: &str,
    from: &str,
    to: &str,
    kind: StoryEdgeKind,
    group: Option<&str>,
) -> StoryEdge {
    StoryEdge {
        id: id.into(),
        from_node_id: from.into(),
        to_node_id: to.into(),
        from_port: "out".into(),
        to_port: "in".into(),
        kind,
        branch_group_id: group.map(str::to_string),
        guard: None,
        state_operations: Vec::new(),
        display_order: 0,
    }
}

fn minimal_graph(project_id: i64, source_snapshot_id: i64) -> StoryGraph {
    StoryGraph {
        schema_version: GALGAME_STORY_GRAPH_SCHEMA_VERSION.into(),
        id: format!("galgame:{project_id}"),
        project_id,
        source_snapshot_id,
        revision: 1,
        entry_node_id: "end".into(),
        terminal_node_ids: vec!["end".into()],
        nodes: vec![graph_node("end", StoryNodeKind::Terminal)],
        edges: Vec::new(),
        branch_groups: Vec::new(),
        loop_groups: Vec::new(),
        state_variables: Vec::new(),
    }
}

#[tokio::test]
async fn persist_script_rejects_empty_scenes_before_database_access() {
    let input = PersistGalgameScriptRecordInput {
        project_id: 1,
        root_project_id: 1,
        source_snapshot_id: 2,
        branch_plan: None,
        title: "测试 Galgame".to_string(),
        style_config_json: json!({}),
        status: "draft".to_string(),
        scenes: Vec::new(),
        expected_story_graph_revision: 0,
        story_graph: minimal_graph(1, 2),
    };

    assert!(matches!(
        persist_galgame_script_record(&DatabaseConnection::Disconnected, input).await,
        Err(GalgameScriptRecordError::InvalidArgument(message))
            if message == "Galgame persist_script 至少需要一个场"
    ));
}

#[tokio::test]
async fn persist_script_writes_scene_line_choice_graph_in_one_contract() {
    let db = a4n_migration::test_support::connect_test_database().await;
    let root = crate::project::create_project_record(
        &db,
        crate::project::CreateProjectRecordInput {
            user_id: 1,
            title: "Galgame 批量写入根项目".to_string(),
            description: String::new(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .unwrap();
    let now = Utc::now().naive_utc();
    let _chapter = chapter::ActiveModel {
        project_id: Set(root.id),
        chapter_number: Set(1),
        title: Set(Some("第一章".to_string())),
        content: Set("正文".to_string()),
        word_count: Set(2),
        status: Set(chapter::STATUS_COMPLETED.to_string()),
        summary: Set(None),
        style_tags: Set(None),
        analysis_data: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(&db)
    .await
    .unwrap();
    let derived = crate::project::create_derived_project_record(
        &db,
        1,
        crate::project::CreateDerivedProjectParams {
            root_project_id: root.id,
            derived_kind: ProjectKind::Galgame.as_str().to_string(),
            title: "Galgame 批量写入".to_string(),
            description: String::new(),
            chapter_scope: crate::project::DerivedChapterScope {
                start_chapter: 1,
                end_chapter: 1,
            },
        },
    )
    .await
    .unwrap();
    let snapshot_chapter = source_snapshot_chapter::Entity::find()
        .filter(source_snapshot_chapter::Column::SnapshotId.eq(derived.source_snapshot_id.unwrap()))
        .one(&db)
        .await
        .unwrap()
        .unwrap();
    let presentation = json!({
        "schema_version": "galgame_line_presentation_v3", "origin": "added", "beat_ref": null,
        "visible_cast": [],
        "enter": [],
        "exit": [],
        "focus_character_profile_id": null,
        "hold": false,
        "player_knowledge": [],
        "cg": null,
        "cg_note": null,
        "cg_importance": null,
        "transition": null,
        "branch": null,
        "branch_id": null,
        "returns_to_scene_index": null,
        "after_beat_ref": null
    });
    let plan_inputs = a4n_domain::media_authoring::AuthoringDocumentInputs {
        sources: vec![a4n_domain::media_authoring::AuthoringSourceRef {
            root_project_id: root.id,
            source_chapter_id: snapshot_chapter.source_chapter_id,
            source_snapshot_id: snapshot_chapter.snapshot_id,
            source_snapshot_chapter_id: snapshot_chapter.id,
            content_sha256: snapshot_chapter.content_hash.clone(),
        }],
        dependencies: Vec::new(),
        video_reference: None,
    };
    let units = (1..=2)
        .map(|id| {
            a4n_domain::media_authoring::legacy::historical_galgame_unit(
                a4n_domain::media_authoring::legacy::HistoricalGalgameScene {
                    id,
                    title: format!("场景 {id}"),
                    location: None,
                    participants: Vec::new(),
                    lines: Vec::new(),
                },
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let adaptation = crate::media_authoring::save_authoring_document(&db, crate::media_authoring::SaveAuthoringDocument {
        project_id: derived.id, source_snapshot_id: snapshot_chapter.snapshot_id, source_chapter_id: Some(snapshot_chapter.source_chapter_id),
        kind: a4n_domain::media_authoring::AuthoringDocumentKind::AdaptationPlan, record_id: None, expected_revision: None,
        inputs: plan_inputs.clone(), document: json!({"schema_version":"adaptation_plan_v1", "target":"galgame", "intention":"保存人工场景图", "decisions":[], "units":units}),
    }).await.unwrap();
    let adaptation = crate::media_authoring::adopt_authoring_document(
        &db,
        derived.id,
        adaptation.id,
        adaptation.revision,
    )
    .await
    .unwrap();
    let branch = crate::media_authoring::save_authoring_document(
        &db,
        crate::media_authoring::SaveAuthoringDocument {
            project_id: derived.id,
            source_snapshot_id: snapshot_chapter.snapshot_id,
            source_chapter_id: None,
            kind: a4n_domain::media_authoring::AuthoringDocumentKind::GalgameBranchPlan,
            record_id: None,
            expected_revision: None,
            inputs: a4n_domain::media_authoring::AuthoringDocumentInputs {
                dependencies: vec![adaptation.reference()],
                ..plan_inputs
            },
            document: json!({"choices":[]}),
        },
    )
    .await
    .unwrap();
    let branch = crate::media_authoring::adopt_authoring_document(
        &db,
        derived.id,
        branch.id,
        branch.revision,
    )
    .await
    .unwrap();
    let script = crate::galgame::replace_galgame_script_for_project_record(
        &db,
        derived.id,
        PersistGalgameScriptRecordInput {
            project_id: derived.id,
            root_project_id: root.id,
            source_snapshot_id: derived.source_snapshot_id.unwrap(),
            branch_plan: Some(branch.reference()),
            title: "批量合同脚本".to_string(),
            style_config_json: json!({}),
            status: "draft".to_string(),
            expected_story_graph_revision: 0,
            story_graph: StoryGraph {
                schema_version: GALGAME_STORY_GRAPH_SCHEMA_VERSION.into(),
                id: format!("galgame:{}", derived.id),
                project_id: derived.id,
                source_snapshot_id: derived.source_snapshot_id.unwrap(),
                revision: 1,
                entry_node_id: "scene:1".into(),
                terminal_node_ids: vec!["scene:2".into()],
                nodes: vec![
                    graph_node("scene:1", StoryNodeKind::Choice),
                    graph_node("join:scene:2", StoryNodeKind::Join),
                    graph_node("scene:2", StoryNodeKind::Terminal),
                ],
                edges: vec![
                    graph_edge(
                        "choice:1",
                        "scene:1",
                        "join:scene:2",
                        StoryEdgeKind::Choice,
                        Some("branch:scene:1"),
                    ),
                    graph_edge(
                        "choice:2",
                        "scene:1",
                        "join:scene:2",
                        StoryEdgeKind::Choice,
                        Some("branch:scene:1"),
                    ),
                    graph_edge(
                        "join:continue",
                        "join:scene:2",
                        "scene:2",
                        StoryEdgeKind::Sequence,
                        None,
                    ),
                ],
                branch_groups: vec![StoryBranchGroup {
                    id: "branch:scene:1".into(),
                    fork_node_id: "scene:1".into(),
                    options: vec![
                        StoryBranchOption {
                            id: "option:1".into(),
                            label: "前进".into(),
                            edge_id: "choice:1".into(),
                            condition: None,
                            state_operations: Vec::new(),
                        },
                        StoryBranchOption {
                            id: "option:2".into(),
                            label: "观察".into(),
                            edge_id: "choice:2".into(),
                            condition: None,
                            state_operations: Vec::new(),
                        },
                    ],
                    join_node_id: Some("join:scene:2".into()),
                }],
                loop_groups: Vec::new(),
                state_variables: Vec::new(),
            },
            scenes: vec![
                crate::galgame::PersistGalgameSceneRecordInput {
                    scene_index: 1,
                    source_snapshot_chapter_id: snapshot_chapter.id,
                    adaptation_plan_id: Some(adaptation.id),
                    adaptation_unit_id: Some("legacy-scene-1".into()),
                    title: "序幕".to_string(),
                    location: Some("大厅".to_string()),
                    time_of_day: None,
                    participants_json: json!([]),
                    background_note: None,
                    entry_kind: "main".to_string(),
                    next_scene_index: Some(2),
                    lines: vec![
                        crate::galgame::PersistGalgameLineRecordInput {
                            line_index: 1,
                            line_type: "narration".to_string(),
                            speaker_profile_id: None,
                            text: "灯亮了。".to_string(),
                            expression: None,
                            pose: None,
                            position: None,
                            presentation_json: presentation.clone(),
                            text_anchor_id: None,
                            choices: Vec::new(),
                        },
                        crate::galgame::PersistGalgameLineRecordInput {
                            line_index: 2,
                            line_type: "choice".to_string(),
                            speaker_profile_id: None,
                            text: "接下来呢？".to_string(),
                            expression: None,
                            pose: None,
                            position: None,
                            presentation_json: presentation.clone(),
                            text_anchor_id: None,
                            choices: vec![
                                crate::galgame::PersistGalgameChoiceRecordInput {
                                    option_index: 1,
                                    option_text: "前进".to_string(),
                                    target_scene_index: 2,
                                },
                                crate::galgame::PersistGalgameChoiceRecordInput {
                                    option_index: 2,
                                    option_text: "观察".to_string(),
                                    target_scene_index: 2,
                                },
                            ],
                        },
                    ],
                },
                crate::galgame::PersistGalgameSceneRecordInput {
                    scene_index: 2,
                    adaptation_plan_id: Some(adaptation.id),
                    adaptation_unit_id: Some("legacy-scene-2".into()),
                    source_snapshot_chapter_id: snapshot_chapter.id,
                    title: "回收".to_string(),
                    location: Some("走廊".to_string()),
                    time_of_day: None,
                    participants_json: json!([]),
                    background_note: None,
                    entry_kind: "main".to_string(),
                    next_scene_index: None,
                    lines: vec![crate::galgame::PersistGalgameLineRecordInput {
                        line_index: 1,
                        line_type: "narration".to_string(),
                        speaker_profile_id: None,
                        text: "故事继续。".to_string(),
                        expression: None,
                        pose: None,
                        position: None,
                        presentation_json: presentation,
                        text_anchor_id: None,
                        choices: Vec::new(),
                    }],
                },
            ],
        },
    )
    .await
    .unwrap();

    let scenes = galgame_scene::Entity::find()
        .filter(galgame_scene::Column::ScriptId.eq(script.id))
        .order_by_asc(galgame_scene::Column::SceneIndex)
        .all(&db)
        .await
        .unwrap();
    assert_eq!(scenes.len(), 2);
    assert_eq!(scenes[0].next_scene_id, Some(scenes[1].id));
    let saved_graph = crate::galgame::current_galgame_story_graph(&db, derived.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(saved_graph.revision, 1);
    assert_eq!(saved_graph.graph.entry_node_id, "scene:1");
    let lines = galgame_line::Entity::find()
        .filter(galgame_line::Column::SceneId.is_in(scenes.iter().map(|scene| scene.id)))
        .order_by_asc(galgame_line::Column::Id)
        .all(&db)
        .await
        .unwrap();
    assert_eq!(lines.len(), 3);
    let reordered = crate::galgame::reorder_galgame_line_records_transaction(
        &db,
        scenes[0].id,
        vec![lines[1].id, lines[0].id],
    )
    .await
    .unwrap();
    assert_eq!(
        reordered
            .iter()
            .map(|line| (line.id, line.line_index))
            .collect::<Vec<_>>(),
        vec![(lines[1].id, 1), (lines[0].id, 2)]
    );
    let choices = galgame_choice::Entity::find()
        .filter(galgame_choice::Column::LineId.eq(lines[1].id))
        .order_by_asc(galgame_choice::Column::OptionIndex)
        .all(&db)
        .await
        .unwrap();
    assert_eq!(choices.len(), 2);
    assert_eq!(choices[0].target_scene_id, Some(scenes[1].id));
    assert_eq!(choices[1].target_scene_id, Some(scenes[1].id));

    crate::galgame::delete_galgame_line_with_generation_media_graph_record(
        &db,
        derived.id,
        lines[0].id,
    )
    .await
    .unwrap();
    let remaining_lines = galgame_line::Entity::find()
        .filter(galgame_line::Column::SceneId.eq(scenes[0].id))
        .order_by_asc(galgame_line::Column::LineIndex)
        .all(&db)
        .await
        .unwrap();
    assert_eq!(
        remaining_lines
            .iter()
            .map(|line| (line.id, line.line_index))
            .collect::<Vec<_>>(),
        vec![(lines[1].id, 1)]
    );
}
