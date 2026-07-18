use std::collections::BTreeMap;

use a4n_core::models::scene::scene_profile;
use serde_json::json;

use super::materials::{MaterialNodeSpec, add_material_image_node};
use crate::project_dependency_graph::{GraphBuilder, NodeSeedMetadata, node};

pub(super) fn add_scene_profile_nodes(
    graph: &mut GraphBuilder,
    profiles: &[scene_profile::Model],
    image_counts: &BTreeMap<i64, u64>,
) {
    graph.add_node(node(
        "scene_profiles",
        "text",
        "scene_profile",
        "场景档案",
        profiles.len() as u64,
        7,
        NodeSeedMetadata {
            status: None,
            editable: true,
            needs_visualization: true,
            visualized: true,
            summary: Some(
                "从正文抽取的地点和空间设定，可派生场景素材图并参与视频/漫画参考图筛选。"
                    .to_string(),
            ),
            details: json!({}),
        },
    ));
    graph.add_edge(
        "chapter_analysis",
        "scene_profiles",
        "derives",
        profiles.len() as u64,
    );
    for profile in profiles {
        let node_id = format!("scene_profile:{}", profile.id);
        let image_count = image_counts.get(&profile.id).copied().unwrap_or(0);
        graph.add_node(
            node(
                &node_id,
                "text",
                "scene_profile_record",
                &profile.name,
                1,
                8,
                NodeSeedMetadata {
                    status: None,
                    editable: true,
                    needs_visualization: true,
                    visualized: true,
                    summary: Some(profile.visual_description.clone()),
                    details: json!({
                        "material_image_count": image_count,
                        "first_seen_chapter": profile.first_seen_chapter,
                        "last_seen_chapter": profile.last_seen_chapter,
                        "importance": profile.importance,
                    }),
                },
            )
            .profile_id(profile.id)
            .record_id(profile.id),
        );
        graph.add_edge("scene_profiles", &node_id, "contains", 1);
        add_material_image_node(
            graph,
            &node_id,
            profile.id,
            &profile.name,
            image_count,
            MaterialNodeSpec {
                node_prefix: "scene_material_images",
                asset_type: "scene_material_image",
                profile_label: "场景",
            },
        );
    }
}
