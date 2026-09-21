use std::collections::BTreeMap;

use serde_json::json;

use crate::project_dependency_graph::{
    GraphBuilder, NodeSeedMetadata, graph::CharacterProfileGraphRow, node,
};

pub(super) fn add_character_profile_nodes(
    graph: &mut GraphBuilder,
    profiles: &[CharacterProfileGraphRow],
    portrait_counts: &BTreeMap<i64, u64>,
) {
    graph.add_node(node(
        "character_profiles",
        "text",
        "character_profile",
        "人物档案",
        profiles.len() as u64,
        7,
        NodeSeedMetadata {
            status: None,
            editable: true,
            needs_visualization: true,
            visualized: true,
            summary: Some("从正文和章节分析同步的人物设定、状态和声音指纹。".to_string()),
            details: json!({}),
        },
    ));
    graph.add_edge(
        "chapter_analysis",
        "character_profiles",
        "derives",
        profiles.len() as u64,
    );
    for profile in profiles {
        let node_id = format!("character_profile:{}", profile.id);
        let portrait_count = portrait_counts.get(&profile.id).copied().unwrap_or(0);
        graph.add_node(
            node(
                &node_id,
                "text",
                "character_profile_record",
                &profile.name,
                1,
                8,
                NodeSeedMetadata {
                    status: None,
                    editable: true,
                    needs_visualization: true,
                    visualized: true,
                    summary: profile.appearance.clone(),
                    details: json!({
                        "portrait_count": portrait_count,
                        "first_seen_chapter": profile.first_seen_chapter(),
                        "last_synced_chapter": profile.last_synced_chapter,
                    }),
                },
            )
            .profile_id(profile.id)
            .record_id(profile.id),
        );
        graph.add_edge("character_profiles", &node_id, "contains", 1);
        if portrait_count > 0 {
            let portraits_node = format!("character_portraits:{}", profile.id);
            graph.add_node(
                node(
                    &portraits_node,
                    "image",
                    "character_portrait",
                    &format!("{}立绘", profile.name),
                    portrait_count,
                    9,
                    NodeSeedMetadata {
                        status: None,
                        editable: false,
                        needs_visualization: true,
                        visualized: true,
                        summary: Some(format!("{} 张角色立绘。", portrait_count)),
                        details: json!({}),
                    },
                )
                .profile_id(profile.id),
            );
            graph.add_edge(&node_id, &portraits_node, "generates", portrait_count);
        }
    }
}
