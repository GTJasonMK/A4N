use std::collections::{BTreeMap, BTreeSet};

use a4n_core::{
    models::chapter::{chapter, chapter_video_prompt},
    prompt_language::{
        PromptOutputLanguage, prompt_type_uses_prompt_output_language,
        with_prompt_output_language_metadata,
    },
};
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use serde_json::json;

use super::{asset_dependency_graph, asset_dependency_graph_node_detail};
use crate::project::{
    CreateDerivedProjectParams, CreateProjectRecordInput, DerivedChapterScope, PROJECT_KIND_VIDEO,
    create_derived_project_record, create_project_record,
};

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

async fn insert_chapter(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_number: i32,
) -> chapter::Model {
    let now = Utc::now().naive_utc();
    chapter::ActiveModel {
        project_id: Set(project_id),
        outline_id: Set(None),
        chapter_number: Set(chapter_number),
        title: Set(Some(format!("第 {chapter_number} 章"))),
        content: Set("他推开门，走进雨夜。".to_string()),
        word_count: Set(10),
        status: Set(chapter::STATUS_IMPORTED.to_string()),
        summary: Set(Some("雨夜开端".to_string())),
        style_tags: Set(None),
        analysis_data: Set(Some(json!({"beats": ["开端"]}))),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap()
}

async fn insert_video_prompt(
    db: &DatabaseConnection,
    project_id: i64,
    chapter_id: i64,
    prompt_type: &str,
    workflow_mode: &str,
) {
    let now = Utc::now().naive_utc();
    chapter_video_prompt::ActiveModel {
        project_id: Set(project_id),
        chapter_id: Set(chapter_id),
        style: Set(None),
        prompt_type: Set(prompt_type.to_string()),
        workflow_mode: Set(workflow_mode.to_string()),
        shot_number: Set(1),
        title: Set(Some("片段规划".to_string())),
        scene_summary: Set(Some("雨夜开端片段规划".to_string())),
        visual_prompt: Set("{}".to_string()),
        motion_prompt: Set(None),
        negative_prompt: Set(None),
        duration_secs: Set(5),
        aspect_ratio: Set(Some("16:9".to_string())),
        reference_image_ids_json: Set(None),
        status: Set("completed".to_string()),
        generated_video_id: Set(None),
        metadata_json: Set(prompt_type_uses_prompt_output_language(prompt_type)
            .then(|| with_prompt_output_language_metadata(json!({}), PromptOutputLanguage::ZhCn))),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

#[tokio::test]
async fn derived_project_graph_matches_root_family_graph() {
    let db = setup_db().await;
    let root = create_project_record(
        &db,
        CreateProjectRecordInput {
            user_id: 1,
            title: "根文本".to_string(),
            description: "测试简介".to_string(),
            project_type: "novel_long".to_string(),
        },
    )
    .await
    .unwrap();
    let chapter = insert_chapter(&db, root.id, 1).await;
    let derived = create_derived_project_record(
        &db,
        1,
        CreateDerivedProjectParams {
            root_project_id: root.id,
            derived_kind: PROJECT_KIND_VIDEO.to_string(),
            title: "视频派生".to_string(),
            description: String::new(),
            chapter_scope: DerivedChapterScope {
                start_chapter: 1,
                end_chapter: 1,
            },
        },
    )
    .await
    .unwrap();
    insert_video_prompt(
        &db,
        derived.id,
        chapter.id,
        "frame_segment_plan",
        "boundary_frames",
    )
    .await;
    insert_video_prompt(&db, derived.id, chapter.id, "video_prompt", "chat_video").await;

    let root_graph = asset_dependency_graph(&db, root.id).await.unwrap();
    let derived_graph = asset_dependency_graph(&db, derived.id).await.unwrap();

    let root_nodes = root_graph
        .nodes
        .iter()
        .map(|node| {
            (
                node.id.clone(),
                (node.asset_type.clone(), node.label.clone(), node.count),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let derived_nodes = derived_graph
        .nodes
        .iter()
        .map(|node| {
            (
                node.id.clone(),
                (node.asset_type.clone(), node.label.clone(), node.count),
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(derived_nodes, root_nodes);

    let root_edges = root_graph
        .edges
        .iter()
        .map(|edge| {
            (
                edge.from.clone(),
                edge.to.clone(),
                edge.relation.clone(),
                edge.count,
            )
        })
        .collect::<BTreeSet<_>>();
    let derived_edges = derived_graph
        .edges
        .iter()
        .map(|edge| {
            (
                edge.from.clone(),
                edge.to.clone(),
                edge.relation.clone(),
                edge.count,
            )
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(derived_edges, root_edges);

    let node = |id: &str| {
        derived_graph
            .nodes
            .iter()
            .find(|node| node.id == id)
            .unwrap_or_else(|| panic!("missing graph node {id}"))
    };
    assert_eq!(node("chapters").count, 1);
    assert_eq!(node("video_segment_plans").count, 1);
    assert_eq!(node("chapter_video_prompts").count, 1);
    assert_eq!(node("derived_projects").count, 1);
    assert!(
        derived_graph
            .nodes
            .iter()
            .all(|node| node.id == "project" || node.count > 0)
    );
    assert!(
        derived_graph
            .warnings
            .iter()
            .all(|warning| !warning.contains("依赖边不符合拓扑层级"))
    );
    assert!(derived_edges.contains(&(
        "chapters".to_string(),
        "video_segment_plans".to_string(),
        "derives".to_string(),
        1,
    )));
    assert!(derived_edges.contains(&(
        "video_segment_plans".to_string(),
        "chapter_video_prompts".to_string(),
        "derives".to_string(),
        1,
    )));
    let detail = asset_dependency_graph_node_detail(&db, derived.id, "video_segment_plans")
        .await
        .unwrap();
    assert_eq!(detail.records.len(), 1);
    assert!(
        detail
            .records
            .iter()
            .all(|record| record.asset_kind == "video_segment_plan")
    );
    assert!(detail.records.iter().all(|record| {
        record
            .source_refs
            .iter()
            .any(|source| source.id == "chapters")
    }));
}
