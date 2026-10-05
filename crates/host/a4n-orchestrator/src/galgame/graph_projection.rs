use std::collections::{HashMap, HashSet};

use a4n_domain::galgame::{
    GALGAME_STORY_GRAPH_SCHEMA_VERSION, SCENE_ENTRY_BRANCH, SCENE_ENTRY_MAIN, StoryBranchGroup,
    StoryBranchOption, StoryEdge, StoryEdgeKind, StoryGraph, StoryNarrativeLine, StoryNode,
    StoryNodeKind, StorySourceRef,
};
use a4n_error::AppError;
use serde_json::{Value, json};

use super::{GalgamePersistScene, GalgameScriptPersistPlan};

pub(crate) fn story_graph_from_persist_plan(
    project_id: i64,
    root_project_id: i64,
    source_snapshot_id: i64,
    revision: i64,
    plan: &GalgameScriptPersistPlan,
) -> Result<StoryGraph, AppError> {
    for (field, value) in [
        ("project_id", project_id),
        ("root_project_id", root_project_id),
        ("source_snapshot_id", source_snapshot_id),
        ("revision", revision),
    ] {
        if value <= 0 {
            return Err(AppError::InvalidArgument(format!("{field} 必须是正整数")));
        }
    }
    if plan.scenes.is_empty() {
        return Err(AppError::InvalidArgument(
            "Galgame 节点图不能没有节点".into(),
        ));
    }
    let scene_by_index = plan
        .scenes
        .iter()
        .map(|scene| (scene.scene_index, scene))
        .collect::<HashMap<_, _>>();
    if scene_by_index.len() != plan.scenes.len() {
        return Err(AppError::InvalidArgument(
            "Galgame 生成计划中的节点索引重复".into(),
        ));
    }
    for scene in &plan.scenes {
        if let Some(target_index) = scene.next_scene_index
            && !scene_by_index.contains_key(&target_index)
        {
            return Err(AppError::InvalidArgument(format!(
                "节点 {} 指向不存在的节点 {}",
                scene.scene_index, target_index
            )));
        }
    }
    let has_choices = plan
        .scenes
        .iter()
        .filter(|scene| scene.lines.iter().any(|line| !line.choices.is_empty()))
        .map(|scene| scene.scene_index)
        .collect::<HashSet<_>>();
    let mut nodes = Vec::with_capacity(plan.scenes.len() * 2);
    let mut edges = Vec::new();
    let mut branch_groups = Vec::new();
    let mut terminal_node_ids = Vec::new();
    for scene in &plan.scenes {
        let source_node_id = scene_node_id(scene.scene_index);
        let choices = scene
            .lines
            .iter()
            .flat_map(|line| line.choices.iter().map(move |choice| (line, choice)))
            .collect::<Vec<_>>();
        let mut outgoing_count = 0usize;
        if !has_choices.contains(&scene.scene_index)
            && let Some(target_index) = scene.next_scene_index
        {
            let target = scene_by_index.get(&target_index).ok_or_else(|| {
                AppError::InvalidArgument(format!(
                    "节点 {} 指向不存在的节点 {}",
                    scene.scene_index, target_index
                ))
            })?;
            if scene.entry_kind != SCENE_ENTRY_BRANCH && target_index <= scene.scene_index {
                return Err(AppError::InvalidArgument(format!(
                    "Galgame 主线存在未声明循环边: {} -> {}",
                    scene.scene_index, target_index
                )));
            }
            edges.push(StoryEdge {
                id: format!("sequence:{}", scene.scene_index),
                from_node_id: source_node_id.clone(),
                to_node_id: scene_node_id(target_index),
                from_port: "next".into(),
                to_port: "in".into(),
                kind: transition_kind(scene, target),
                branch_group_id: None,
                guard: None,
                state_operations: Vec::new(),
                display_order: 0,
            });
            outgoing_count += 1;
        }

        if !choices.is_empty() {
            let group_id = format!("branch:scene:{}", scene.scene_index);
            let mut options = Vec::with_capacity(choices.len());
            for (display_order, (line, choice)) in choices.iter().enumerate() {
                let target = scene_by_index
                    .get(&choice.target_scene_index)
                    .ok_or_else(|| {
                        AppError::InvalidArgument(format!(
                            "Galgame 选择节点 {} 指向不存在的节点 {}",
                            scene.scene_index, choice.target_scene_index
                        ))
                    })?;
                let edge_id = format!(
                    "choice:{}:{}:{}",
                    scene.scene_index, line.line_index, choice.option_index
                );
                edges.push(StoryEdge {
                    id: edge_id.clone(),
                    from_node_id: source_node_id.clone(),
                    to_node_id: scene_node_id(choice.target_scene_index),
                    from_port: format!("choice:{}:{}", line.line_index, choice.option_index),
                    to_port: "in".into(),
                    kind: StoryEdgeKind::Choice,
                    branch_group_id: Some(group_id.clone()),
                    guard: None,
                    state_operations: Vec::new(),
                    display_order: i32::try_from(display_order).map_err(|_| {
                        AppError::InvalidArgument("Galgame 分支数量超出范围".into())
                    })?,
                });
                options.push(StoryBranchOption {
                    id: edge_id.clone(),
                    label: choice.option_text.clone(),
                    edge_id,
                    condition: None,
                    state_operations: Vec::new(),
                });
                outgoing_count += 1;
                if target.scene_index != choice.target_scene_index {
                    return Err(AppError::InvalidArgument("Galgame 节点索引不一致".into()));
                }
            }
            branch_groups.push(StoryBranchGroup {
                id: group_id,
                fork_node_id: source_node_id.clone(),
                options,
                join_node_id: None,
            });
        }

        let kind = if !choices.is_empty() {
            StoryNodeKind::Choice
        } else if outgoing_count == 0 {
            terminal_node_ids.push(source_node_id.clone());
            StoryNodeKind::Terminal
        } else {
            StoryNodeKind::Narrative
        };
        let source_refs = source_refs(scene, root_project_id, source_snapshot_id);
        let primary_source = source_refs.first().cloned().or_else(|| {
            Some(StorySourceRef {
                root_project_id,
                source_snapshot_id,
                source_snapshot_chapter_id: scene.source_snapshot_chapter_id,
                source_chapter_id: None,
                source_span_ids: Vec::new(),
            })
        });
        let lines = scene
            .lines
            .iter()
            .map(|line| StoryNarrativeLine {
                id: line_node_id(scene.scene_index, line.line_index),
                display_order: line.line_index.saturating_sub(1),
                line_type: line.line_type.clone(),
                speaker_profile_id: line.speaker_profile_id,
                text: line.text.clone(),
                presentation: line.presentation_json.clone(),
            })
            .collect();
        nodes.push(StoryNode {
            id: source_node_id,
            kind,
            title: scene.title.clone(),
            summary: scene.background_note.clone().unwrap_or_default(),
            primary_source,
            source_refs,
            lines,
            state_before: Default::default(),
            state_after: Default::default(),
            state_operations: Vec::new(),
            media_intents: media_intents(scene),
            layout_x: f64::from(scene.scene_index) * 260.0,
            layout_y: if scene.entry_kind == SCENE_ENTRY_BRANCH {
                240.0
            } else {
                0.0
            },
        });
    }

    let incoming_counts = edges
        .iter()
        .fold(HashMap::<String, usize>::new(), |mut counts, edge| {
            *counts.entry(edge.to_node_id.clone()).or_default() += 1;
            counts
        });
    let join_targets = plan
        .scenes
        .iter()
        .filter(|scene| {
            incoming_counts
                .get(&scene_node_id(scene.scene_index))
                .copied()
                .unwrap_or_default()
                > 1
        })
        .map(|scene| scene.scene_index)
        .collect::<HashSet<_>>();
    for target_index in &join_targets {
        let target = scene_by_index.get(target_index).ok_or_else(|| {
            AppError::InvalidArgument(format!("汇合目标节点不存在: {target_index}"))
        })?;
        let join_id = join_node_id(*target_index);
        let target_id = scene_node_id(*target_index);
        for edge in &mut edges {
            if edge.to_node_id == target_id {
                edge.to_node_id = join_id.clone();
            }
        }
        edges.push(StoryEdge {
            id: format!("{join_id}:continue"),
            from_node_id: join_id.clone(),
            to_node_id: target_id,
            from_port: "out".into(),
            to_port: "in".into(),
            kind: transition_kind(target, target),
            branch_group_id: None,
            guard: None,
            state_operations: Vec::new(),
            display_order: 0,
        });
        nodes.push(StoryNode {
            id: join_id,
            kind: StoryNodeKind::Join,
            title: format!("汇合：{}", target.title),
            summary: String::new(),
            primary_source: None,
            source_refs: Vec::new(),
            lines: Vec::new(),
            state_before: Default::default(),
            state_after: Default::default(),
            state_operations: Vec::new(),
            media_intents: Vec::new(),
            layout_x: f64::from(target.scene_index) * 260.0 - 130.0,
            layout_y: 360.0,
        });
    }

    for branch in &mut branch_groups {
        let Some(join_target) = options_join_target(branch, &edges, &join_targets, &scene_by_index)
        else {
            continue;
        };
        branch.join_node_id = Some(join_node_id(join_target));
    }

    let entry_scene = plan
        .scenes
        .iter()
        .filter(|scene| scene.entry_kind == SCENE_ENTRY_MAIN)
        .min_by_key(|scene| scene.scene_index)
        .or_else(|| plan.scenes.iter().min_by_key(|scene| scene.scene_index))
        .ok_or_else(|| AppError::InvalidArgument("Galgame 生成计划缺少入口节点".into()))?;
    let graph = StoryGraph {
        schema_version: GALGAME_STORY_GRAPH_SCHEMA_VERSION.into(),
        id: format!("galgame:{project_id}"),
        project_id,
        source_snapshot_id,
        revision,
        entry_node_id: scene_node_id(entry_scene.scene_index),
        terminal_node_ids,
        nodes,
        edges,
        branch_groups,
        loop_groups: Vec::new(),
        state_variables: Vec::new(),
    };
    graph.validate().map_err(AppError::InvalidArgument)?;
    Ok(graph)
}

fn scene_node_id(scene_index: i32) -> String {
    format!("scene:{scene_index}")
}

fn line_node_id(scene_index: i32, line_index: i32) -> String {
    format!("line:{scene_index}:{line_index}")
}

fn join_node_id(scene_index: i32) -> String {
    format!("join:scene:{scene_index}")
}

fn transition_kind(source: &GalgamePersistScene, target: &GalgamePersistScene) -> StoryEdgeKind {
    if source.source_snapshot_chapter_id != target.source_snapshot_chapter_id {
        StoryEdgeKind::ChapterTransition
    } else {
        StoryEdgeKind::Sequence
    }
}

fn source_refs(
    scene: &GalgamePersistScene,
    root_project_id: i64,
    source_snapshot_id: i64,
) -> Vec<StorySourceRef> {
    let mut seen = HashSet::new();
    scene
        .lines
        .iter()
        .filter_map(|line| line.text_anchor_id)
        .filter(|anchor_id| seen.insert(*anchor_id))
        .map(|anchor_id| StorySourceRef {
            root_project_id,
            source_snapshot_id,
            source_snapshot_chapter_id: scene.source_snapshot_chapter_id,
            source_chapter_id: None,
            source_span_ids: vec![format!("text-anchor:{anchor_id}")],
        })
        .collect()
}

fn media_intents(scene: &GalgamePersistScene) -> Vec<Value> {
    let mut intents = vec![json!({
        "kind": "background",
        "prompt": scene.background_note,
        "scene_index": scene.scene_index,
    })];
    for line in &scene.lines {
        if line.presentation_json.get("cg").and_then(Value::as_bool) == Some(true) {
            intents.push(json!({
                "kind": "cg",
                "prompt": line.presentation_json.get("cg_note"),
                "scene_index": scene.scene_index,
                "line_index": line.line_index,
            }));
        }
        if let Some(visible_cast) = line
            .presentation_json
            .get("visible_cast")
            .and_then(Value::as_array)
        {
            for cast in visible_cast {
                intents.push(json!({
                    "kind": "portrait",
                    "profile_id": cast.get("character_profile_id"),
                    "expression": cast.get("expression"),
                    "pose": cast.get("pose"),
                    "line_index": line.line_index,
                }));
            }
        }
    }
    intents
}

fn options_join_target(
    group: &StoryBranchGroup,
    edges: &[StoryEdge],
    join_targets: &HashSet<i32>,
    scenes: &HashMap<i32, &GalgamePersistScene>,
) -> Option<i32> {
    let targets = group
        .options
        .iter()
        .filter_map(|option| edges.iter().find(|edge| edge.id == option.edge_id))
        .map(|edge| edge.to_node_id.as_str())
        .collect::<Vec<_>>();
    if targets.len() != group.options.len() {
        return None;
    }
    let direct_join = targets.iter().find_map(|target| {
        target
            .strip_prefix("join:scene:")
            .and_then(|value| value.parse::<i32>().ok())
            .filter(|target_index| join_targets.contains(target_index))
    });
    if let Some(join_target) = direct_join {
        return targets
            .iter()
            .all(|target| {
                target
                    .strip_prefix("join:scene:")
                    .and_then(|value| value.parse::<i32>().ok())
                    == Some(join_target)
                    || target
                        .strip_prefix("scene:")
                        .and_then(|value| value.parse::<i32>().ok())
                        .and_then(|scene_index| scenes.get(&scene_index))
                        .is_some_and(|scene| {
                            scene.entry_kind == SCENE_ENTRY_BRANCH
                                && scene.next_scene_index == Some(join_target)
                        })
            })
            .then_some(join_target);
    }
    let option_targets = group
        .options
        .iter()
        .filter_map(|option| edges.iter().find(|edge| edge.id == option.edge_id))
        .filter_map(|edge| {
            edge.to_node_id
                .strip_prefix("scene:")
                .and_then(|value| value.parse::<i32>().ok())
        })
        .collect::<Vec<_>>();
    let first_scene = scenes.get(option_targets.first()?)?;
    if first_scene.entry_kind != SCENE_ENTRY_BRANCH {
        return None;
    }
    let join_target = first_scene.next_scene_index?;
    (join_targets.contains(&join_target)
        && option_targets.iter().all(|scene_index| {
            scenes.get(scene_index).is_some_and(|scene| {
                scene.entry_kind == SCENE_ENTRY_BRANCH
                    && scene.next_scene_index == Some(join_target)
            })
        }))
    .then_some(join_target)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::galgame::{GalgamePersistChoice, GalgamePersistLine};

    fn scene(
        scene_index: i32,
        source_snapshot_chapter_id: i64,
        entry_kind: &str,
        next_scene_index: Option<i32>,
        choices: Vec<GalgamePersistChoice>,
    ) -> GalgamePersistScene {
        GalgamePersistScene {
            scene_index,
            source_snapshot_chapter_id,
            adaptation_source: None,
            title: format!("Scene {scene_index}"),
            location: None,
            time_of_day: None,
            participants_json: json!([]),
            background_note: None,
            entry_kind: entry_kind.into(),
            next_scene_index,
            lines: vec![GalgamePersistLine {
                line_index: 1,
                line_type: if choices.is_empty() {
                    "narration".into()
                } else {
                    "choice".into()
                },
                speaker_profile_id: None,
                text: format!("Line {scene_index}"),
                expression: None,
                pose: None,
                position: None,
                presentation_json: json!({}),
                text_anchor_id: None,
                choices,
            }],
        }
    }

    #[test]
    fn creates_explicit_join_for_cross_chapter_branch_convergence() {
        let plan = GalgameScriptPersistPlan {
            scenes: vec![
                scene(
                    1,
                    10,
                    SCENE_ENTRY_MAIN,
                    None,
                    vec![
                        GalgamePersistChoice {
                            option_index: 1,
                            option_text: "North".into(),
                            target_scene_index: 2,
                        },
                        GalgamePersistChoice {
                            option_index: 2,
                            option_text: "South".into(),
                            target_scene_index: 3,
                        },
                    ],
                ),
                scene(2, 11, SCENE_ENTRY_BRANCH, Some(4), Vec::new()),
                scene(3, 12, SCENE_ENTRY_BRANCH, Some(4), Vec::new()),
                scene(4, 20, SCENE_ENTRY_MAIN, None, Vec::new()),
            ],
        };

        let graph = story_graph_from_persist_plan(7, 2, 9, 1, &plan).unwrap();
        let join = graph
            .nodes
            .iter()
            .find(|node| node.kind == StoryNodeKind::Join)
            .unwrap();

        assert_eq!(join.id, "join:scene:4");
        assert_eq!(
            graph.branch_groups[0].join_node_id.as_deref(),
            Some("join:scene:4")
        );
        assert_eq!(
            graph
                .edges
                .iter()
                .filter(|edge| edge.to_node_id == join.id)
                .count(),
            2
        );
        let chapter_transitions = graph
            .edges
            .iter()
            .filter(|edge| edge.from_node_id == "scene:2" || edge.from_node_id == "scene:3")
            .map(|edge| edge.kind)
            .collect::<Vec<_>>();
        assert_eq!(chapter_transitions.len(), 2);
        assert!(
            chapter_transitions
                .iter()
                .all(|kind| *kind == StoryEdgeKind::ChapterTransition)
        );
    }

    #[test]
    fn rejects_duplicate_scene_indexes_before_projection() {
        let plan = GalgameScriptPersistPlan {
            scenes: vec![
                scene(1, 10, SCENE_ENTRY_MAIN, None, Vec::new()),
                scene(1, 10, SCENE_ENTRY_MAIN, None, Vec::new()),
            ],
        };

        let error = story_graph_from_persist_plan(7, 2, 9, 1, &plan).unwrap_err();
        assert!(error.to_string().contains("节点索引重复"));
    }
}
