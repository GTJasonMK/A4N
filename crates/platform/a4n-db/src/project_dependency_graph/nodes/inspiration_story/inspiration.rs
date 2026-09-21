use serde_json::json;

use crate::project_dependency_graph::graph::ConversationGraphRow;
use crate::project_dependency_graph::{GraphBuilder, NodeSeedMetadata, node};

pub(in crate::project_dependency_graph) fn add_inspiration_nodes(
    graph: &mut GraphBuilder,
    conversations: &[ConversationGraphRow],
) {
    let message_count = conversations
        .iter()
        .map(|row| row.message_count.max(0) as u64)
        .sum::<u64>();
    graph.add_node(node(
        "inspiration",
        "text",
        "inspiration_conversation",
        "灵感对话",
        conversations.len() as u64,
        1,
        NodeSeedMetadata {
            status: None,
            editable: false,
            needs_visualization: true,
            visualized: true,
            summary: Some(format!(
                "{} 个对话，{} 条消息，是蓝图生成的上游来源。",
                conversations.len(),
                message_count
            )),
            details: json!({ "message_count": message_count }),
        },
    ));
    graph.add_edge(
        "project",
        "inspiration",
        "contains",
        conversations.len() as u64,
    );
}
