use super::*;
use crate::project::{
    CreateDerivedProjectParams, DerivedChapterScope, PROJECT_KIND_VIDEO, PROJECT_TYPE_NOVEL_LONG,
};
use a4n_core::event::bridge::BroadcastEventBus;
use chrono::Utc;
use sea_orm::{ActiveModelTrait, DatabaseConnection, Set};
use std::sync::Arc;

async fn setup_db() -> DatabaseConnection {
    a4n_migration::test_support::connect_test_database().await
}

fn service(db: Arc<DatabaseConnection>) -> ProjectService {
    ProjectService::new(db, Arc::new(BroadcastEventBus::new(16)))
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
        title: Set(Some("镜头分析".to_string())),
        scene_summary: Set(Some("雨夜开端镜头分析".to_string())),
        visual_prompt: Set("{}".to_string()),
        motion_prompt: Set(None),
        negative_prompt: Set(None),
        duration_secs: Set(5),
        aspect_ratio: Set(Some("16:9".to_string())),
        reference_image_ids_json: Set(None),
        status: Set("completed".to_string()),
        generated_video_id: Set(None),
        metadata_json: Set(None),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    }
    .insert(db)
    .await
    .unwrap();
}

fn video_prompt_model(
    id: i64,
    prompt_type: &str,
    workflow_mode: &str,
) -> chapter_video_prompt::Model {
    let now = Utc::now().naive_utc();
    chapter_video_prompt::Model {
        id,
        project_id: 2,
        chapter_id: 1,
        style: None,
        prompt_type: prompt_type.to_string(),
        workflow_mode: workflow_mode.to_string(),
        shot_number: 1,
        title: Some(prompt_type.to_string()),
        scene_summary: Some("雨夜开端".to_string()),
        visual_prompt: "{}".to_string(),
        motion_prompt: None,
        negative_prompt: None,
        duration_secs: 5,
        aspect_ratio: Some("16:9".to_string()),
        reference_image_ids_json: None,
        status: "completed".to_string(),
        generated_video_id: None,
        metadata_json: None,
        created_at: now,
        updated_at: now,
    }
}

fn generated_frame_image(id: i64, prompt_id: i64) -> generated_image::Model {
    let now = Utc::now().naive_utc();
    generated_image::Model {
        id,
        project_id: 2,
        image_type: "video_first_frame".to_string(),
        file_path: format!("video/frame-{id}.png"),
        prompt: "frame".to_string(),
        model: "test".to_string(),
        width: 1024,
        height: 576,
        file_size: Some(100),
        ai_score: None,
        is_favorite: false,
        chapter_id: Some(1),
        panel_id: None,
        metadata_json: Some(json!({"prompt_id": prompt_id})),
        created_at: now,
    }
}

#[test]
fn video_graph_edges_follow_primary_chain_without_cross_level_shortcuts() {
    let now = Utc::now().naive_utc();
    let chapters = vec![chapter::Model {
        id: 1,
        project_id: 1,
        outline_id: None,
        chapter_number: 1,
        title: Some("第一章".to_string()),
        content: "他推开门，走进雨夜。".to_string(),
        word_count: 10,
        status: chapter::STATUS_IMPORTED.to_string(),
        summary: Some("雨夜开端".to_string()),
        style_tags: None,
        analysis_data: None,
        created_at: now,
        updated_at: now,
    }];
    let prompts = vec![
        video_prompt_model(1, "frame_shot_analysis", "boundary_frames"),
        video_prompt_model(2, "frame_segment_plan", "boundary_frames"),
        video_prompt_model(3, "frame_prompt", "boundary_frames"),
        video_prompt_model(4, "video_prompt", "boundary_frames"),
    ];
    let images = vec![generated_frame_image(10, 3)];
    let videos = Vec::new();
    let mut graph = GraphBuilder::default();
    graph.add_node(node(
        "chapters",
        "text",
        "chapter_content",
        "章节正文",
        1,
        5,
        NodeSeedMetadata {
            status: None,
            editable: true,
            needs_visualization: true,
            visualized: true,
            summary: None,
            details: json!({}),
        },
    ));

    add_video_nodes(&mut graph, &chapters, &prompts, &images, &videos);
    let graph = graph.finish(1).unwrap();
    let edges = graph
        .edges
        .iter()
        .map(|edge| (edge.from.as_str(), edge.to.as_str(), edge.relation.as_str()))
        .collect::<BTreeSet<_>>();

    assert!(edges.contains(&("video_shot_analysis", "video_segment_plans", "derives")));
    assert!(edges.contains(&("video_segment_plans", "video_frame_prompts", "derives")));
    assert!(edges.contains(&("video_frame_prompts", "video_frame_images", "generates")));
    assert!(edges.contains(&("video_frame_images", "chapter_video_prompts", "derives")));
    assert!(
        !edges.iter().any(|(from, to, _)| {
            *from == "video_shot_analysis" && *to == "chapter_video_prompts"
        })
    );
    assert!(
        !edges.iter().any(|(from, to, _)| {
            *from == "video_segment_plans" && *to == "chapter_video_prompts"
        })
    );
    assert!(
        !edges.iter().any(|(from, to, _)| {
            *from == "video_frame_prompts" && *to == "chapter_video_prompts"
        })
    );
}

#[tokio::test]
async fn derived_project_graph_matches_root_family_graph() {
    let db = Arc::new(setup_db().await);
    let service = service(db.clone());
    let root = service
        .create(1, "根文本", "测试简介", PROJECT_TYPE_NOVEL_LONG)
        .await
        .unwrap();
    let chapter = insert_chapter(db.as_ref(), root.id, 1).await;
    let derived = service
        .create_derived_project(
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
                creation_options_json: json!({}),
            },
        )
        .await
        .unwrap();
    insert_video_prompt(
        db.as_ref(),
        derived.id,
        chapter.id,
        "frame_shot_analysis",
        "boundary_frames",
    )
    .await;
    insert_video_prompt(
        db.as_ref(),
        derived.id,
        chapter.id,
        "chat_video_shot_analysis",
        "chat_video",
    )
    .await;
    insert_video_prompt(
        db.as_ref(),
        derived.id,
        chapter.id,
        "chat_video_segment_plan",
        "chat_video",
    )
    .await;
    insert_video_prompt(
        db.as_ref(),
        derived.id,
        chapter.id,
        "video_prompt",
        "chat_video",
    )
    .await;

    let root_graph = service.asset_dependency_graph(root.id).await.unwrap();
    let derived_graph = service.asset_dependency_graph(derived.id).await.unwrap();

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
    assert_eq!(node("video_shot_analysis").count, 2);
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
        "video_shot_analysis".to_string(),
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
    assert!(!derived_edges.iter().any(|(from, to, _, _)| {
        from == "video_shot_analysis" && to == "chapter_video_prompts"
    }));

    let detail = service
        .asset_dependency_graph_node_detail(derived.id, "video_shot_analysis")
        .await
        .unwrap();
    assert_eq!(detail.records.len(), 2);
    assert!(
        detail
            .records
            .iter()
            .all(|record| record.asset_kind == "video_shot_analysis")
    );
    assert!(detail.records.iter().all(|record| {
        record
            .source_refs
            .iter()
            .any(|source| source.id == "chapters")
    }));
}
