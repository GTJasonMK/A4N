use super::*;

pub(in crate::project_dependency_graph) fn add_generation_nodes(
    graph: &mut GraphBuilder,
    generation_specs: &[generation_spec::Model],
    prompt_artifacts: &[prompt_artifact::Model],
    media_assets: &[media_asset::Model],
    media_provenance: &[media_provenance::Model],
) {
    if generation_specs.is_empty() && prompt_artifacts.is_empty() && media_assets.is_empty() {
        return;
    }
    graph.add_node(node(
        "generation_specs",
        "text",
        "generation_spec",
        "统一生成规格",
        generation_specs.len() as u64,
        8,
        NodeSeedMetadata {
            status: None,
            editable: false,
            needs_visualization: false,
            visualized: false,
            summary: Some(
                "统一 generation/media 链路中的生成意图记录；这是系统内部依赖，不需要单独人工编辑。"
                    .to_string(),
            ),
            details: json!({
                "target_kinds": count_strings(generation_specs.iter().map(|row| row.target_kind.as_str())),
            }),
        },
    ));
    graph.add_edge(
        "chapters",
        "generation_specs",
        "references",
        generation_specs.len() as u64,
    );

    graph.add_node(node(
        "prompt_artifacts",
        "prompt",
        "prompt_artifact",
        "统一提示词产物",
        prompt_artifacts.len() as u64,
        9,
        NodeSeedMetadata {
            status: None,
            editable: false,
            needs_visualization: false,
            visualized: false,
            summary: Some(
                "统一 generation/media 链路中的渲染提示词快照；通常由具体工作区查看，不作为独立编辑入口。"
                    .to_string(),
            ),
            details: json!({
                "prompt_kinds": count_strings(prompt_artifacts.iter().map(|row| row.prompt_kind.as_str())),
            }),
        },
    ));
    graph.add_edge(
        "generation_specs",
        "prompt_artifacts",
        "renders",
        prompt_artifacts.len() as u64,
    );

    graph.add_node(node(
        "media_assets",
        "media",
        "media_asset",
        "统一媒体资产",
        media_assets.len() as u64,
        10,
        NodeSeedMetadata {
            status: None,
            editable: false,
            needs_visualization: false,
            visualized: false,
            summary: Some("统一 media_asset 表中的图片/视频资产。".to_string()),
            details: json!({
                "media_kinds": count_strings(media_assets.iter().map(|row| row.media_kind.as_str())),
                "provenance_count": media_provenance.len(),
            }),
        },
    ));
    graph.add_edge(
        "prompt_artifacts",
        "media_assets",
        "generates",
        media_assets.len() as u64,
    );
}

pub(in crate::project_dependency_graph) fn add_project_derivation_nodes(
    graph: &mut GraphBuilder,
    derived_projects: &[novel_project::Model],
    project_derivations: &[project_derivation::Model],
) {
    if derived_projects.is_empty() {
        return;
    }
    graph.add_node(node(
        "derived_projects",
        "text",
        "derived_project",
        "派生项目",
        derived_projects.len() as u64,
        13,
        NodeSeedMetadata {
            status: None,
            editable: false,
            needs_visualization: true,
            visualized: true,
            summary: Some("由当前文字项目派生出的漫画、视频或其他项目。".to_string()),
            details: json!({
                "derived_kinds": count_strings(derived_projects.iter().map(|row| row.project_kind.as_str())),
                "project_ids": derived_projects.iter().map(|row| row.id).collect::<Vec<_>>(),
                "derivation_count": project_derivations.len(),
            }),
        },
    ));
    graph.add_edge(
        "chapters",
        "derived_projects",
        "snapshots",
        derived_projects.len() as u64,
    );
}
