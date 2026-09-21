use serde_json::json;

use crate::project_dependency_graph::{GraphBuilder, NodeSeedMetadata, node};

pub(super) struct MaterialNodeSpec<'a> {
    pub node_prefix: &'a str,
    pub asset_type: &'a str,
    pub profile_label: &'a str,
}

pub(super) fn add_material_image_node(
    graph: &mut GraphBuilder,
    profile_node_id: &str,
    profile_id: i64,
    profile_name: &str,
    image_count: u64,
    spec: MaterialNodeSpec<'_>,
) {
    if image_count == 0 {
        return;
    }
    let images_node = format!("{}:{profile_id}", spec.node_prefix);
    graph.add_node(
        node(
            &images_node,
            "image",
            spec.asset_type,
            &format!("{profile_name}素材图"),
            image_count,
            9,
            NodeSeedMetadata {
                status: None,
                editable: false,
                needs_visualization: true,
                visualized: true,
                summary: Some(format!(
                    "{}档案「{}」派生的 {} 张图片。",
                    spec.profile_label, profile_name, image_count
                )),
                details: json!({}),
            },
        )
        .profile_id(profile_id),
    );
    graph.add_edge(profile_node_id, &images_node, "generates", image_count);
}
