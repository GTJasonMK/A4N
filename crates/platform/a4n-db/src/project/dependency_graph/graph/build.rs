use a4n_error::AppError;
use serde_json::json;

use super::records::AssetGraphRecords;
use crate::project::dependency_graph::{
    AssetDependencyGraph,
    builder::{GraphBuilder, NodeSeedMetadata, node},
    nodes::{
        add_derived_project_nodes, add_generation_nodes, add_inspiration_nodes, add_manga_nodes,
        add_profile_nodes, add_story_nodes, add_video_nodes,
    },
    scope::AssetGraphScope,
};

pub(super) fn build_asset_dependency_graph(
    scope: &AssetGraphScope,
    records: &AssetGraphRecords,
) -> Result<AssetDependencyGraph, AppError> {
    let root_project_id = scope.root_project.id;
    let standalone_comic = scope.root_project.project_kind == "comic";
    let mut graph = GraphBuilder::default();
    graph.add_node(node(
        "project",
        if standalone_comic { "comic" } else { "text" },
        "project",
        &scope.root_project.title,
        1,
        0,
        NodeSeedMetadata {
            status: Some(scope.root_project.status.clone()),
            editable: false,
            needs_visualization: true,
            visualized: true,
            summary: Some(if standalone_comic {
                "独立漫画项目根节点，来源由项目文本素材和标题范围固定。".to_string()
            } else {
                "项目根节点，承载从灵感到正文、图像、漫画和视频的完整派生链路。"
                    .to_string()
            }),
            details: json!({
                "project_kind": scope.root_project.project_kind,
                "project_type": scope.root_project.project_type,
                "target_chapters": scope.root_project.target_chapters,
                "current_chapter": scope.root_project.current_chapter,
                "graph_root_project_id": root_project_id,
                "family_project_ids": scope.family_project_ids,
            }),
        },
    ));

    add_inspiration_nodes(&mut graph, &records.conversations);
    add_story_nodes(
        &mut graph,
        &records.blueprints,
        &records.blueprint_versions,
        &records.part_outlines,
        &records.chapter_outlines,
        &records.chapters,
    );
    add_profile_nodes(
        &mut graph,
        &records.character_profiles,
        &records.object_profiles,
        &records.scene_profiles,
        &records.portraits,
        &records.generated_images,
    );
    add_manga_nodes(
        &mut graph,
        &records.chapters,
        &records.manga_prompts,
        &records.comic_text_sources,
        &records.generated_images,
    );
    add_video_nodes(
        &mut graph,
        &records.chapters,
        &records.visual_plans,
        &records.shooting_scripts,
        &records.video_prompts,
        &records.generated_images,
        &records.generated_videos,
    );
    add_generation_nodes(
        &mut graph,
        &records.generation_specs,
        &records.prompt_artifacts,
        &records.media_assets,
        records.media_provenance_count,
    );
    add_derived_project_nodes(&mut graph, &scope.derived_projects);

    graph.finish(root_project_id)
}
