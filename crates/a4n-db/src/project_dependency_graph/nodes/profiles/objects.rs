use std::collections::BTreeMap;

use a4n_core::models::asset::object_profile;
use serde_json::json;

use super::materials::{MaterialNodeSpec, add_material_image_node};
use crate::project_dependency_graph::{GraphBuilder, NodeSeedMetadata, node};

pub(super) fn add_object_profile_nodes(
    graph: &mut GraphBuilder,
    profiles: &[object_profile::Model],
    image_counts: &BTreeMap<i64, u64>,
) {
    graph.add_node(node(
        "object_profiles",
        "text",
        "object_profile",
        "物体档案",
        profiles.len() as u64,
        7,
        NodeSeedMetadata {
            status: None,
            editable: true,
            needs_visualization: true,
            visualized: true,
            summary: Some(
                "从正文抽取的重要物件设定，可派生物体素材图并参与视频参考图筛选。".to_string(),
            ),
            details: json!({}),
        },
    ));
    graph.add_edge(
        "chapter_analysis",
        "object_profiles",
        "derives",
        profiles.len() as u64,
    );
    for profile in profiles {
        let node_id = format!("object_profile:{}", profile.id);
        let image_count = image_counts.get(&profile.id).copied().unwrap_or(0);
        graph.add_node(
            node(
                &node_id,
                "text",
                "object_profile_record",
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
        graph.add_edge("object_profiles", &node_id, "contains", 1);
        add_material_image_node(
            graph,
            &node_id,
            profile.id,
            &profile.name,
            image_count,
            MaterialNodeSpec {
                node_prefix: "object_material_images",
                asset_type: "object_material_image",
                profile_label: "物体",
            },
        );
    }
}
