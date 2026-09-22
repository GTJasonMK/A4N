use a4n_domain::image::{IMAGE_TYPE_OBJECT_MATERIAL, IMAGE_TYPE_SCENE_MATERIAL};
use serde_json::json;

use super::{
    counts::{material_image_counts_by_profile, portrait_counts_by_profile},
    materials::{MaterialNodeSpec, add_material_image_node},
};
use crate::project::dependency_graph::graph::{GeneratedImageGraphRow, PortraitGraphRow};
use crate::project::dependency_graph::{GraphBuilder, NodeSeedMetadata, node};

fn portrait(profile_id: i64) -> PortraitGraphRow {
    PortraitGraphRow { profile_id }
}

fn material_image(
    id: i64,
    image_type: &str,
    profile_kind: &str,
    profile_id: i64,
) -> GeneratedImageGraphRow {
    let _ = id;
    GeneratedImageGraphRow {
        image_type: image_type.to_string(),
        metadata_json: Some(json!({
            "profile_kind": profile_kind,
            "profile_id": profile_id,
        })),
    }
}

#[test]
fn profile_asset_counts_group_portraits_and_filter_material_kind() {
    let portrait_counts = portrait_counts_by_profile(&[portrait(7), portrait(7), portrait(8)]);
    assert_eq!(portrait_counts.get(&7), Some(&2));
    assert_eq!(portrait_counts.get(&8), Some(&1));

    let images = vec![
        material_image(1, IMAGE_TYPE_OBJECT_MATERIAL, "object", 7),
        material_image(2, IMAGE_TYPE_OBJECT_MATERIAL, "object", 7),
        material_image(3, IMAGE_TYPE_SCENE_MATERIAL, "scene", 7),
        material_image(4, IMAGE_TYPE_OBJECT_MATERIAL, "scene", 9),
    ];
    let object_counts = material_image_counts_by_profile(&images, "object");
    let scene_counts = material_image_counts_by_profile(&images, "scene");
    assert_eq!(object_counts.get(&7), Some(&2));
    assert_eq!(scene_counts.get(&7), Some(&1));
    assert!(!object_counts.contains_key(&9));
}

#[test]
fn shared_material_node_skips_zero_and_projects_profile_edge() {
    let mut graph = GraphBuilder::default();
    graph.add_node(node(
        "object_profile:7",
        "text",
        "object_profile_record",
        "怀表",
        1,
        8,
        NodeSeedMetadata {
            status: None,
            editable: true,
            needs_visualization: true,
            visualized: true,
            summary: None,
            details: json!({}),
        },
    ));
    let spec = MaterialNodeSpec {
        node_prefix: "object_material_images",
        asset_type: "object_material_image",
        profile_label: "物体",
    };
    add_material_image_node(&mut graph, "object_profile:7", 7, "怀表", 0, spec);
    assert_eq!(graph.node_count("object_material_images:7"), 0);

    add_material_image_node(
        &mut graph,
        "object_profile:7",
        7,
        "怀表",
        2,
        MaterialNodeSpec {
            node_prefix: "object_material_images",
            asset_type: "object_material_image",
            profile_label: "物体",
        },
    );
    let graph = graph.finish(1).unwrap();
    let material = graph
        .nodes
        .iter()
        .find(|node| node.id == "object_material_images:7")
        .unwrap();
    assert_eq!(material.asset_type, "object_material_image");
    assert_eq!(material.profile_id, Some(7));
    assert_eq!(material.count, 2);
    assert_eq!(
        material.summary.as_deref(),
        Some("物体档案「怀表」派生的 2 张图片。")
    );
    assert!(graph.edges.iter().any(|edge| {
        edge.from == "object_profile:7"
            && edge.to == "object_material_images:7"
            && edge.relation == "generates"
            && edge.count == 2
    }));
}
