use std::collections::{BTreeMap, BTreeSet, HashMap};

use a4n_api_contract::export::ExportFileData;
use a4n_archive_core::{ArchiveEntry, write_zip};
use a4n_domain::{
    character::CharacterPortraitRecord,
    galgame::{
        GalgameFacingV2, GalgameLineDocumentSchemaVersion, GalgameLinePresentationDocumentV3,
        GalgameStageLayerV2, StoryEdge, StoryGraph, StoryNode,
    },
};
use a4n_error::AppError;
use a4n_storage::canonicalize_data_file_path;
use base64::Engine as _;

use crate::OrchestratorContext;

use super::{
    GalgameChoice, GalgameLine, GalgameLineType, GalgameMediaRole, GalgameMediaTargetKind,
    GalgameScene, GalgameSceneDetail, GalgameSceneEntryKind, GalgameScriptDetail,
    webgal_export_rules::{
        media_file_extension, media_file_extension_for_webgal_role, sanitize_export_file_stem,
        webgal_choice_text, webgal_string_literal, webgal_transition_args,
    },
};
use crate::media::galgame_media_binding_required_media_kind;
use std::path::Path;

#[derive(Debug, Clone)]
struct WebgalExportAsset {
    script_name: String,
    zip_path: String,
    data_path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct WebgalFigureKey {
    profile_id: i64,
    expression: Option<String>,
    pose: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WebgalFigureState {
    script_name: String,
    position: String,
    facing: GalgameFacingV2,
    layer: GalgameStageLayerV2,
}

#[derive(Default)]
struct WebgalExportAssets {
    scene_backgrounds: BTreeMap<i64, WebgalExportAsset>,
    line_cgs: BTreeMap<i64, WebgalExportAsset>,
    line_bgms: BTreeMap<i64, WebgalExportAsset>,
    line_sfx: BTreeMap<i64, WebgalExportAsset>,
    figures: BTreeMap<WebgalFigureKey, WebgalExportAsset>,
}

impl OrchestratorContext {
    pub async fn run_export_galgame_webgal_validated(
        &self,
        project_id: i64,
    ) -> Result<ExportFileData, AppError> {
        let detail = self.get_script_detail(project_id).await?;
        let graph = detail
            .story_graph
            .as_ref()
            .ok_or_else(|| AppError::InvalidState {
                current: "missing_story_graph".into(),
                operation: "导出 WebGAL 前先生成 Galgame 节点图".into(),
            })?;
        if detail.story_graph_status.as_deref() == Some("stale") {
            return Err(AppError::InvalidState {
                current: "story_graph_stale".into(),
                operation: "从来源已变化的节点图导出 WebGAL".into(),
            });
        }
        let export_detail = webgal_detail_from_story_graph(&detail, graph)?;
        let portraits = self
            .galgame_webgal_portraits(detail.root_project_id)
            .await?;
        let bytes = build_webgal_package(self.data_dir(), &export_detail, &portraits)?;
        let file_stem = sanitize_export_file_stem(&detail.title).map_err(invalid_webgal)?;
        Ok(ExportFileData {
            file_name: format!("{file_stem}-webgal.zip"),
            mime_type: "application/zip".to_string(),
            data_base64: base64::engine::general_purpose::STANDARD.encode(bytes),
        })
    }
}

fn webgal_detail_from_story_graph(
    source: &GalgameScriptDetail,
    graph: &StoryGraph,
) -> Result<GalgameScriptDetail, AppError> {
    graph.validate().map_err(invalid_webgal)?;
    if !graph.loop_groups.is_empty()
        || !graph.state_variables.is_empty()
        || graph
            .nodes
            .iter()
            .any(|node| !node.state_operations.is_empty())
        || graph
            .edges
            .iter()
            .any(|edge| edge.guard.is_some() || !edge.state_operations.is_empty())
        || graph
            .branch_groups
            .iter()
            .flat_map(|group| &group.options)
            .any(|option| option.condition.is_some() || !option.state_operations.is_empty())
    {
        return Err(invalid_webgal(
            "当前 WebGAL 导出目标不能无损表示节点图中的循环、状态变量或条件操作",
        ));
    }
    if graph
        .edges
        .iter()
        .any(|edge| edge.kind == a4n_domain::galgame::StoryEdgeKind::Interrupt)
    {
        return Err(invalid_webgal("当前 WebGAL 导出目标不能无损表示中断边"));
    }

    let ordered_nodes = ordered_story_nodes(graph)?;
    let mut used_scene_ids = BTreeSet::new();
    let reserved_scene_ids = ordered_nodes
        .iter()
        .filter_map(|node| graph_numeric_id(&node.id, "scene:"))
        .collect::<BTreeSet<_>>();
    let mut next_scene_id = 1_i64;
    let mut scene_id_by_node = BTreeMap::new();
    for node in &ordered_nodes {
        let scene_id = allocate_graph_id(
            &node.id,
            "scene:",
            &reserved_scene_ids,
            &mut used_scene_ids,
            &mut next_scene_id,
        );
        scene_id_by_node.insert(node.id.clone(), scene_id);
    }

    let mut used_line_ids = BTreeSet::new();
    let reserved_line_ids = ordered_nodes
        .iter()
        .flat_map(|node| &node.lines)
        .filter_map(|line| graph_numeric_id(&line.id, "line:"))
        .collect::<BTreeSet<_>>();
    let mut next_line_id = 1_i64;
    let mut used_choice_ids = BTreeSet::new();
    let reserved_choice_ids = graph
        .edges
        .iter()
        .filter_map(|edge| graph_numeric_id(&edge.id, "choice:"))
        .collect::<BTreeSet<_>>();
    let mut next_choice_id = 1_i64;
    let mut scenes = Vec::with_capacity(ordered_nodes.len());
    let mut graph_media_targets = Vec::new();
    for node in &ordered_nodes {
        let scene_id = scene_id_by_node[&node.id];
        let mut graph_lines = node.lines.iter().collect::<Vec<_>>();
        graph_lines.sort_by_key(|line| line.display_order);
        let outgoing = sorted_outgoing_edges(graph, node);
        let branch_groups = graph
            .branch_groups
            .iter()
            .filter(|group| group.fork_node_id == node.id)
            .collect::<Vec<_>>();
        if branch_groups.len() > 1 {
            return Err(invalid_webgal(format!(
                "节点 {} 包含多个独立分支组，WebGAL 无法无损表达",
                node.id
            )));
        }

        let choice_line_story_id = (outgoing.len() > 1)
            .then(|| {
                graph_lines
                    .iter()
                    .rev()
                    .find(|line| line.line_type == "choice")
                    .map(|line| line.id.clone())
            })
            .flatten();
        let mut lines = Vec::with_capacity(
            graph_lines.len() + usize::from(outgoing.len() > 1 && choice_line_story_id.is_none()),
        );
        let mut line_id_by_story_id = HashMap::new();
        for story_line in &graph_lines {
            let line_id = allocate_graph_id(
                &story_line.id,
                "line:",
                &reserved_line_ids,
                &mut used_line_ids,
                &mut next_line_id,
            );
            line_id_by_story_id.insert(story_line.id.as_str(), line_id);
            let mut line_type = GalgameLineType::parse(&story_line.line_type).ok_or_else(|| {
                invalid_webgal(format!(
                    "节点 {} 的台词类型无法导出: {}",
                    node.id, story_line.line_type
                ))
            })?;
            if line_type == GalgameLineType::Choice
                && choice_line_story_id.as_deref() != Some(story_line.id.as_str())
            {
                line_type = GalgameLineType::Narration;
            }
            let presentation_json = serde_json::from_value::<GalgameLinePresentationDocumentV3>(
                story_line.presentation.clone(),
            )
            .map_err(|error| {
                invalid_webgal(format!(
                    "节点 {} 台词 {} 的演出信息无效: {error}",
                    node.id, story_line.id
                ))
            })?;
            lines.push(GalgameLine {
                id: line_id,
                scene_id,
                line_index: i32::try_from(lines.len() + 1)
                    .map_err(|_| invalid_webgal("节点台词数量超出 WebGAL 范围"))?,
                line_type,
                speaker_profile_id: story_line.speaker_profile_id,
                text: story_line.text.clone(),
                expression: None,
                pose: None,
                position: None,
                presentation_json,
                text_anchor_id: None,
            });
        }
        if let Some(scene_index) = graph_scene_index(&node.id)
            && let Some(source_scene) = source
                .scenes
                .iter()
                .find(|scene| scene.scene.scene_index == scene_index)
        {
            graph_media_targets.push((
                GalgameMediaTargetKind::GalgameScene,
                source_scene.scene.id,
                scene_id,
            ));
            for story_line in &graph_lines {
                let Some((line_scene_index, line_index)) = graph_line_identity(&story_line.id)
                else {
                    continue;
                };
                if line_scene_index != scene_index {
                    continue;
                }
                let Some(source_line) = source_scene
                    .lines
                    .iter()
                    .find(|line| line.line_index == line_index)
                else {
                    continue;
                };
                let Some(export_line_id) = line_id_by_story_id.get(story_line.id.as_str()) else {
                    continue;
                };
                graph_media_targets.push((
                    GalgameMediaTargetKind::GalgameLine,
                    source_line.id,
                    *export_line_id,
                ));
            }
        }

        let choices = if outgoing.len() > 1 {
            let choice_line_id = if let Some(story_line_id) = choice_line_story_id.as_deref() {
                line_id_by_story_id[story_line_id]
            } else {
                let line_id = allocate_graph_id(
                    &format!("webgal-choice:{}", node.id),
                    "line:",
                    &reserved_line_ids,
                    &mut used_line_ids,
                    &mut next_line_id,
                );
                lines.push(GalgameLine {
                    id: line_id,
                    scene_id,
                    line_index: i32::try_from(lines.len() + 1)
                        .map_err(|_| invalid_webgal("节点台词数量超出 WebGAL 范围"))?,
                    line_type: GalgameLineType::Choice,
                    speaker_profile_id: None,
                    text: String::new(),
                    expression: None,
                    pose: None,
                    position: None,
                    presentation_json: empty_webgal_presentation(),
                    text_anchor_id: None,
                });
                line_id
            };
            outgoing
                .iter()
                .enumerate()
                .map(|(index, edge)| {
                    let target_scene_id =
                        *scene_id_by_node.get(&edge.to_node_id).ok_or_else(|| {
                            invalid_webgal(format!(
                                "边 {} 指向不存在的节点 {}",
                                edge.id, edge.to_node_id
                            ))
                        })?;
                    let label = branch_groups
                        .first()
                        .and_then(|group| {
                            group
                                .options
                                .iter()
                                .find(|option| option.edge_id == edge.id)
                        })
                        .map(|option| option.label.as_str())
                        .unwrap_or_else(|| {
                            graph
                                .nodes
                                .iter()
                                .find(|target| target.id == edge.to_node_id)
                                .map(|target| target.title.as_str())
                                .unwrap_or("Continue")
                        });
                    Ok(GalgameChoice {
                        id: allocate_graph_id(
                            &edge.id,
                            "choice:",
                            &reserved_choice_ids,
                            &mut used_choice_ids,
                            &mut next_choice_id,
                        ),
                        line_id: choice_line_id,
                        option_index: i32::try_from(index + 1)
                            .map_err(|_| invalid_webgal("分支选项数量超出 WebGAL 范围"))?,
                        option_text: label.to_string(),
                        target_scene_id: Some(target_scene_id),
                    })
                })
                .collect::<Result<Vec<_>, AppError>>()?
        } else {
            Vec::new()
        };

        let next_scene_id = if outgoing.len() == 1 {
            Some(
                *scene_id_by_node
                    .get(&outgoing[0].to_node_id)
                    .ok_or_else(|| {
                        invalid_webgal(format!(
                            "边 {} 指向不存在的节点 {}",
                            outgoing[0].id, outgoing[0].to_node_id
                        ))
                    })?,
            )
        } else {
            None
        };
        let source_chapter_id = node
            .primary_source
            .as_ref()
            .or_else(|| node.source_refs.first())
            .map(|source| source.source_snapshot_chapter_id)
            .or_else(|| {
                source
                    .scenes
                    .first()
                    .map(|scene| scene.scene.source_snapshot_chapter_id)
            })
            .unwrap_or(1);
        scenes.push(GalgameSceneDetail {
            scene: GalgameScene {
                id: scene_id,
                source_snapshot_chapter_id: source_chapter_id,
                adaptation_plan_id: None,
                adaptation_unit_id: None,
                scene_index: i32::try_from(scenes.len() + 1)
                    .map_err(|_| invalid_webgal("节点数量超出 WebGAL 范围"))?,
                title: node.title.clone(),
                location: None,
                time_of_day: None,
                background_note: (!node.summary.is_empty()).then(|| node.summary.clone()),
                entry_kind: GalgameSceneEntryKind::Main,
                next_scene_id,
            },
            lines,
            choices,
        });
    }
    let mut export_detail = source.clone();
    export_detail.scenes = scenes;
    export_detail.media_bindings = source
        .media_bindings
        .iter()
        .filter_map(|binding| {
            let export_target =
                graph_media_targets
                    .iter()
                    .find_map(|(target_kind, source_id, export_id)| {
                        (binding.binding.target_kind == *target_kind
                            && binding.binding.target_id == *source_id)
                            .then_some(*export_id)
                    })?;
            let mut binding = binding.clone();
            binding.binding.target_id = export_target;
            Some(binding)
        })
        .collect();
    export_detail.story_graph = Some(graph.clone());
    Ok(export_detail)
}

fn ordered_story_nodes(graph: &StoryGraph) -> Result<Vec<&StoryNode>, AppError> {
    let mut ordered = Vec::with_capacity(graph.nodes.len());
    let mut visited = BTreeSet::new();
    let mut pending = vec![graph.entry_node_id.as_str()];
    while let Some(node_id) = pending.pop() {
        if !visited.insert(node_id.to_string()) {
            continue;
        }
        let node = graph
            .nodes
            .iter()
            .find(|node| node.id == node_id)
            .ok_or_else(|| invalid_webgal(format!("入口路径引用不存在的节点: {node_id}")))?;
        ordered.push(node);
        let mut outgoing = sorted_outgoing_edges(graph, node);
        outgoing.reverse();
        pending.extend(outgoing.into_iter().map(|edge| edge.to_node_id.as_str()));
    }
    if ordered.len() != graph.nodes.len() {
        return Err(invalid_webgal("节点图包含从入口不可达的节点"));
    }
    Ok(ordered)
}

fn sorted_outgoing_edges<'a>(graph: &'a StoryGraph, node: &StoryNode) -> Vec<&'a StoryEdge> {
    let mut edges = graph.outgoing_edges(&node.id);
    edges.sort_by_key(|edge| edge.display_order);
    edges
}

fn allocate_graph_id(
    value: &str,
    prefix: &str,
    reserved: &BTreeSet<i64>,
    used: &mut BTreeSet<i64>,
    next: &mut i64,
) -> i64 {
    if let Some(id) = graph_numeric_id(value, prefix)
        && used.insert(id)
    {
        return id;
    }
    while reserved.contains(&*next) || !used.insert(*next) {
        *next = (*next).saturating_add(1);
    }
    let allocated = *next;
    *next = (*next).saturating_add(1);
    allocated
}

fn graph_numeric_id(value: &str, prefix: &str) -> Option<i64> {
    value
        .strip_prefix(prefix)
        .and_then(|suffix| suffix.parse::<i64>().ok())
        .filter(|id| *id > 0)
}

fn graph_scene_index(node_id: &str) -> Option<i32> {
    node_id
        .strip_prefix("scene:")
        .and_then(|suffix| suffix.parse::<i32>().ok())
        .filter(|index| *index > 0)
}

fn graph_line_identity(line_id: &str) -> Option<(i32, i32)> {
    let mut parts = line_id.strip_prefix("line:")?.split(':');
    let scene_index = parts.next()?.parse::<i32>().ok()?;
    let line_index = parts.next()?.parse::<i32>().ok()?;
    (parts.next().is_none() && scene_index > 0 && line_index > 0)
        .then_some((scene_index, line_index))
}

fn empty_webgal_presentation() -> GalgameLinePresentationDocumentV3 {
    GalgameLinePresentationDocumentV3 {
        schema_version: GalgameLineDocumentSchemaVersion::V3,
        origin: a4n_domain::media_authoring::AdaptationOrigin::Added,
        beat_ref: None,
        visible_cast: Vec::new(),
        enter: Vec::new(),
        exit: Vec::new(),
        focus_character_profile_id: None,
        hold: false,
        player_knowledge: Vec::new(),
        cg: None,
        cg_note: None,
        cg_importance: None,
        transition: None,
        branch: None,
        branch_id: None,
        returns_to_scene_index: None,
        after_beat_ref: None,
    }
}

fn build_webgal_package(
    data_dir: &Path,
    detail: &GalgameScriptDetail,
    portraits: &[CharacterPortraitRecord],
) -> Result<Vec<u8>, AppError> {
    let scene_file_by_id = detail
        .scenes
        .iter()
        .enumerate()
        .map(|(index, scene)| {
            (
                scene.scene.id,
                format!("scene_{:03}_{}.txt", index + 1, scene.scene.id),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let speaker_name_by_id = detail
        .speaker_profiles
        .iter()
        .map(|profile| (profile.id, profile.name.clone()))
        .collect::<BTreeMap<_, _>>();
    let choices_by_line = detail
        .scenes
        .iter()
        .flat_map(|scene| scene.choices.iter())
        .fold(BTreeMap::<i64, Vec<_>>::new(), |mut choices, choice| {
            choices.entry(choice.line_id).or_default().push(choice);
            choices
        });
    let figure_profile_ids = detail
        .scenes
        .iter()
        .flat_map(|scene| &scene.lines)
        .flat_map(|line| &line.presentation_json.visible_cast)
        .map(|member| member.character_profile_id)
        .collect::<BTreeSet<_>>();
    validate_webgal_choice_contract(detail, &scene_file_by_id)?;
    let assets = webgal_export_assets(detail, portraits)?;
    let first_scene = scene_file_by_id
        .get(&detail.scenes[0].scene.id)
        .ok_or_else(|| invalid_webgal("Galgame 首场导出文件名缺失"))?;

    let mut entries = vec![ArchiveEntry {
        path: "game/scene/start.txt".to_string(),
        bytes: format!("changeScene:{first_scene};\n").into_bytes(),
        label: "WebGAL 启动场景".to_string(),
    }];
    for scene in &detail.scenes {
        let file_name = scene_file_by_id.get(&scene.scene.id).ok_or_else(|| {
            invalid_webgal(format!("Galgame 场导出文件名缺失: {}", scene.scene.id))
        })?;
        let script = render_webgal_scene(
            scene,
            &scene_file_by_id,
            &speaker_name_by_id,
            &choices_by_line,
            &assets,
            &figure_profile_ids,
        )?;
        entries.push(ArchiveEntry {
            path: format!("game/scene/{file_name}"),
            bytes: script.into_bytes(),
            label: format!("WebGAL 场景 {}", scene.scene.id),
        });
    }

    let mut written_assets = BTreeSet::new();
    for asset in assets
        .scene_backgrounds
        .values()
        .chain(assets.line_cgs.values())
        .chain(assets.line_bgms.values())
        .chain(assets.line_sfx.values())
        .chain(assets.figures.values())
    {
        if written_assets.insert(asset.zip_path.clone()) {
            entries.push(ArchiveEntry {
                path: asset.zip_path.clone(),
                bytes: read_webgal_data_file(data_dir, &asset.data_path)?,
                label: format!("WebGAL 媒体 {}", asset.zip_path),
            });
        }
    }

    let config = webgal_config_text(detail)?;
    write_zip(
        std::iter::once(ArchiveEntry {
            path: "game/config.txt".to_string(),
            bytes: config.into_bytes(),
            label: "WebGAL 配置".to_string(),
        })
        .chain(entries),
        "WebGAL zip 包",
    )
    .map_err(|error| invalid_webgal(error.to_string()))
}

fn render_webgal_scene(
    scene: &GalgameSceneDetail,
    scene_file_by_id: &BTreeMap<i64, String>,
    speaker_name_by_id: &BTreeMap<i64, String>,
    choices_by_line: &BTreeMap<i64, Vec<&GalgameChoice>>,
    assets: &WebgalExportAssets,
    figure_profile_ids: &BTreeSet<i64>,
) -> Result<String, AppError> {
    let mut output = String::new();
    let mut current_figure_by_profile = BTreeMap::<i64, WebgalFigureState>::new();
    for profile_id in figure_profile_ids {
        output.push_str(&format!(
            "changeFigure:none -id=a4n_profile_{profile_id} -next;\n"
        ));
    }
    if let Some(background) = assets.scene_backgrounds.get(&scene.scene.id) {
        output.push_str(&format!("changeBg:{} -next;\n", background.script_name));
    }
    for line in &scene.lines {
        if let Some(bgm) = assets.line_bgms.get(&line.id) {
            output.push_str(&format!("bgm:{};\n", bgm.script_name));
        }
        if let Some(sfx) = assets.line_sfx.get(&line.id) {
            output.push_str(&format!("playEffect:{} -next;\n", sfx.script_name));
        }
        if let Some(cg) = assets.line_cgs.get(&line.id) {
            let transition_args = webgal_transition_args(line.presentation_json.transition)
                .map_err(invalid_webgal)?;
            let name_var = format!("__a4n_cg_name_{}", line.id);
            output.push_str(&format!(
                "setVar:{name_var}=({});\n",
                webgal_string_literal(&format!("{} #{} CG", scene.scene.title, line.line_index))
            ));
            output.push_str(&format!(
                "changeBg:{}{} -next;\n",
                cg.script_name, transition_args
            ));
            output.push_str(&format!(
                "unlockCg:{} -name={{{name_var}}};\n",
                cg.script_name
            ));
        }
        let visible_figures = line
            .presentation_json
            .visible_cast
            .iter()
            .filter_map(|member| {
                webgal_cast_figure(member, assets)
                    .map(|figure| (member.character_profile_id, (member, figure)))
            })
            .collect::<BTreeMap<_, _>>();
        for profile_id in current_figure_by_profile.keys() {
            if visible_figures.contains_key(profile_id) {
                continue;
            }
            if line.presentation_json.exit.contains(profile_id) {
                output.push_str(&format!(
                    "setTransition: -target=a4n_profile_{profile_id} -exit=exit;\n"
                ));
            }
            output.push_str(&format!(
                "changeFigure:none -id=a4n_profile_{profile_id} -next;\n"
            ));
        }
        current_figure_by_profile.retain(|profile_id, _| visible_figures.contains_key(profile_id));
        for (profile_id, (member, figure)) in visible_figures {
            let state = WebgalFigureState {
                script_name: figure.script_name.clone(),
                position: member.position.to_string(),
                facing: member.facing,
                layer: member.layer,
            };
            if current_figure_by_profile.get(&profile_id) == Some(&state) {
                continue;
            }
            let layer = match member.layer {
                GalgameStageLayerV2::Back => 1,
                GalgameStageLayerV2::Middle => 2,
                GalgameStageLayerV2::Front => 3,
            };
            let facing_scale = if member.facing == GalgameFacingV2::Left {
                -1
            } else {
                1
            };
            let enter = if line.presentation_json.enter.contains(&profile_id) {
                match member.position.as_str() {
                    "left" => " -enter=enter-from-left",
                    "right" => " -enter=enter-from-right",
                    _ => " -enter=enter",
                }
            } else {
                ""
            };
            output.push_str(&format!(
                "changeFigure:{} -id=a4n_profile_{profile_id} -{} -zIndex={layer} -transform={{\"scale\":{{\"x\":{facing_scale},\"y\":1}}}}{} -next;\n",
                figure.script_name,
                state.position,
                enter,
            ));
            current_figure_by_profile.insert(profile_id, state);
        }
        output.push_str(&format!(
            "setVar:__a4n_line_text=({});\n",
            webgal_string_literal(&line.text)
        ));
        match line.line_type {
            GalgameLineType::Dialogue | GalgameLineType::Monologue => {
                let speaker_profile_id = line.speaker_profile_id.ok_or_else(|| {
                    invalid_webgal(format!(
                        "Galgame {} 行 {} 缺少 speaker_profile_id，无法导出 WebGAL",
                        line.line_type, line.id
                    ))
                })?;
                let speaker = speaker_name_by_id.get(&speaker_profile_id).ok_or_else(|| {
                    invalid_webgal(format!(
                        "Galgame {} 行 {} 绑定的角色档案 {} 不存在，无法导出 WebGAL",
                        line.line_type, line.id, speaker_profile_id
                    ))
                })?;
                output.push_str(&format!(
                    "setVar:__a4n_line_speaker=({});\nsay:{{__a4n_line_text}} -speaker={{__a4n_line_speaker}};\n",
                    webgal_string_literal(speaker)
                ));
            }
            GalgameLineType::Narration | GalgameLineType::StageDirection => {
                output.push_str("say:{__a4n_line_text} -clear;\n");
            }
            GalgameLineType::Choice => {
                let choices = choices_by_line
                    .get(&line.id)
                    .filter(|choices| !choices.is_empty())
                    .ok_or_else(|| {
                        invalid_webgal(format!(
                            "Galgame choice 行 {} 缺少 galgame_choice 选项，无法导出 WebGAL",
                            line.id
                        ))
                    })?;
                let mut sorted_choices = choices.clone();
                sorted_choices.sort_by_key(|choice| choice.option_index);
                let options = sorted_choices
                    .iter()
                    .map(|choice| {
                        let target_scene_id = choice.target_scene_id.ok_or_else(|| {
                            invalid_webgal(format!(
                                "Galgame choice {} 缺少 target_scene_id，无法导出 WebGAL",
                                choice.id
                            ))
                        })?;
                        let target = scene_file_by_id.get(&target_scene_id).ok_or_else(|| {
                            invalid_webgal(format!(
                                "Galgame choice {} 指向不存在的场 {}，无法导出 WebGAL",
                                choice.id, target_scene_id
                            ))
                        })?;
                        let text_var = format!("__a4n_choice_{}_{}", line.id, choice.option_index);
                        let choice_text =
                            webgal_choice_text(&choice.option_text).map_err(invalid_webgal)?;
                        output.push_str(&format!(
                            "setVar:{text_var}=({});\n",
                            webgal_string_literal(&choice_text)
                        ));
                        Ok(format!("{{{text_var}}}:{target}"))
                    })
                    .collect::<Result<Vec<_>, AppError>>()?
                    .join("|");
                output.push_str(&format!(
                    "say:{{__a4n_line_text}} -clear -next;\nchoose:{options};\n"
                ));
            }
        }
    }
    if let Some(next_scene_id) = scene.scene.next_scene_id {
        let next_file = scene_file_by_id.get(&next_scene_id).ok_or_else(|| {
            invalid_webgal(format!(
                "Galgame 场 {} next_scene_id 指向不存在的场 {next_scene_id}，无法导出 WebGAL",
                scene.scene.id
            ))
        })?;
        output.push_str(&format!("changeScene:{next_file};\n"));
    }
    Ok(output)
}

fn webgal_cast_figure<'a>(
    member: &a4n_domain::galgame::GalgameVisibleCastMemberV2,
    assets: &'a WebgalExportAssets,
) -> Option<&'a WebgalExportAsset> {
    let key = WebgalFigureKey {
        profile_id: member.character_profile_id,
        expression: Some(member.expression.clone()),
        pose: Some(member.pose.clone()),
    };
    assets.figures.get(&key)
}

fn webgal_export_assets(
    detail: &GalgameScriptDetail,
    portraits: &[CharacterPortraitRecord],
) -> Result<WebgalExportAssets, AppError> {
    let mut assets = WebgalExportAssets::default();
    for item in &detail.media_bindings {
        let binding = &item.binding;
        let asset = &item.asset;
        let expected_media_kind = galgame_media_binding_required_media_kind(
            binding.target_kind.as_str(),
            binding.role.as_str(),
        )
        .ok_or_else(|| {
            invalid_webgal(format!(
                "Galgame 媒体绑定目标/角色无效，无法导出 WebGAL: binding_id={}, target_kind={}, role={}",
                binding.id, binding.target_kind, binding.role
            ))
        })?;
        if asset.media_kind != expected_media_kind {
            return Err(invalid_webgal(format!(
                "Galgame 媒体绑定资产类型不匹配，无法导出 WebGAL: binding_id={}, media_asset_id={}, role={}, expected_media_kind={}, actual_media_kind={}",
                binding.id, asset.id, binding.role, expected_media_kind, asset.media_kind
            )));
        }
        let file_path = asset
            .file_path
            .as_deref()
            .ok_or_else(|| invalid_webgal(format!("媒体资产 {} 缺少文件路径", asset.id)))?;
        let extension = media_file_extension_for_webgal_role(file_path, binding.role)
            .map_err(invalid_webgal)?;
        let exported = |directory: &str, prefix: &str| WebgalExportAsset {
            script_name: format!("{prefix}_{}_{}.{}", binding.target_id, asset.id, extension),
            zip_path: format!(
                "game/{directory}/{prefix}_{}_{}.{}",
                binding.target_id, asset.id, extension
            ),
            data_path: file_path.to_string(),
        };
        match (binding.target_kind, binding.role) {
            (GalgameMediaTargetKind::GalgameScene, GalgameMediaRole::Background) => {
                assets
                    .scene_backgrounds
                    .insert(binding.target_id, exported("background", "background"));
            }
            (GalgameMediaTargetKind::GalgameLine, GalgameMediaRole::Cg) => {
                assets
                    .line_cgs
                    .insert(binding.target_id, exported("background", "cg"));
            }
            (GalgameMediaTargetKind::GalgameLine, GalgameMediaRole::Bgm) => {
                assets
                    .line_bgms
                    .insert(binding.target_id, exported("bgm", "bgm"));
            }
            (GalgameMediaTargetKind::GalgameLine, GalgameMediaRole::Sfx) => {
                assets
                    .line_sfx
                    .insert(binding.target_id, exported("vocal", "sfx"));
            }
            _ => {
                return Err(invalid_webgal(format!(
                    "Galgame 媒体绑定目标/角色无效，无法导出 WebGAL: binding_id={}, target_kind={}, role={}",
                    binding.id, binding.target_kind, binding.role
                )));
            }
        }
    }
    for portrait in portraits {
        let extension = media_file_extension(&portrait.image_path).map_err(invalid_webgal)?;
        let key = if portrait.is_primary && portrait.expression.is_none() && portrait.pose.is_none()
        {
            WebgalFigureKey {
                profile_id: portrait.profile_id,
                expression: None,
                pose: None,
            }
        } else {
            WebgalFigureKey {
                profile_id: portrait.profile_id,
                expression: portrait.expression.clone(),
                pose: portrait.pose.clone(),
            }
        };
        assets.figures.insert(
            key,
            WebgalExportAsset {
                script_name: format!(
                    "figure_{}_{}.{}",
                    portrait.profile_id, portrait.id, extension
                ),
                zip_path: format!(
                    "game/figure/figure_{}_{}.{}",
                    portrait.profile_id, portrait.id, extension
                ),
                data_path: portrait.image_path.clone(),
            },
        );
    }
    Ok(assets)
}

fn validate_webgal_choice_contract(
    detail: &GalgameScriptDetail,
    scene_file_by_id: &BTreeMap<i64, String>,
) -> Result<(), AppError> {
    for scene in &detail.scenes {
        let line_type_by_id = scene
            .lines
            .iter()
            .map(|line| (line.id, line.line_type))
            .collect::<BTreeMap<_, _>>();
        let mut choices_by_line = BTreeMap::<i64, Vec<&GalgameChoice>>::new();
        for choice in &scene.choices {
            match line_type_by_id.get(&choice.line_id).copied() {
                Some(GalgameLineType::Choice) => {}
                Some(line_type) => {
                    return Err(invalid_webgal(format!(
                        "Galgame choice {} 挂在非 choice 行下，无法导出 WebGAL: line_id={}, line_type={line_type}",
                        choice.id, choice.line_id
                    )));
                }
                None => {
                    return Err(invalid_webgal(format!(
                        "Galgame choice {} 引用不存在的行，无法导出 WebGAL: line_id={}",
                        choice.id, choice.line_id
                    )));
                }
            }
            let target_scene_id = choice.target_scene_id.ok_or_else(|| {
                invalid_webgal(format!(
                    "Galgame choice {} 缺少 target_scene_id，无法导出 WebGAL",
                    choice.id
                ))
            })?;
            if !scene_file_by_id.contains_key(&target_scene_id) {
                return Err(invalid_webgal(format!(
                    "Galgame choice {} 指向不存在的场 {target_scene_id}，无法导出 WebGAL",
                    choice.id
                )));
            }
            choices_by_line
                .entry(choice.line_id)
                .or_default()
                .push(choice);
        }

        for line in scene
            .lines
            .iter()
            .filter(|line| line.line_type == GalgameLineType::Choice)
        {
            let choices = choices_by_line.get(&line.id).ok_or_else(|| {
                invalid_webgal(format!(
                    "Galgame choice 行 {} 缺少选项，无法导出 WebGAL",
                    line.id
                ))
            })?;
            if choices.len() < 2 {
                return Err(invalid_webgal(format!(
                    "Galgame choice 行 {} 至少需要两个选项，无法导出 WebGAL: choice_count={}",
                    line.id,
                    choices.len()
                )));
            }
            let mut option_indexes = BTreeSet::new();
            for choice in choices {
                if choice.option_text.trim().is_empty() || choice.option_index <= 0 {
                    return Err(invalid_webgal(format!(
                        "Galgame choice {} 选项文本或序号无效，无法导出 WebGAL",
                        choice.id
                    )));
                }
                if !option_indexes.insert(choice.option_index) {
                    return Err(invalid_webgal(format!(
                        "Galgame choice {} option_index 重复，无法导出 WebGAL: line_id={}, option_index={}",
                        choice.id, choice.line_id, choice.option_index
                    )));
                }
            }
            let choice_count = i32::try_from(choices.len()).map_err(|_| {
                invalid_webgal(format!(
                    "Galgame choice 行 {} 的选项数量超过 i32 范围",
                    line.id
                ))
            })?;
            let expected_indexes = (1..=choice_count).collect::<BTreeSet<_>>();
            if option_indexes != expected_indexes {
                return Err(invalid_webgal(format!(
                    "Galgame choice 行 {} option_index 必须从 1 连续编号，无法导出 WebGAL",
                    line.id
                )));
            }
        }
    }
    Ok(())
}

fn webgal_config_text(detail: &GalgameScriptDetail) -> Result<String, AppError> {
    let title = &detail.title;
    if title.trim() != title
        || title
            .chars()
            .any(|ch| matches!(ch, ';' | '\r' | '\n' | '\t'))
        || title.contains(" -")
        || title.split('|').any(|part| part.trim() != part)
    {
        return Err(invalid_webgal(
            "WebGAL 配置无法无损表示游戏标题中的分号、控制空白、参数分隔符或带边界空格的竖线",
        ));
    }
    Ok(format!(
        "Game_name:{title};\nGame_key:a4n_galgame_{};\n",
        detail.id
    ))
}

fn read_webgal_data_file(data_dir: &Path, relative_path: &str) -> Result<Vec<u8>, AppError> {
    let raw = Path::new(relative_path);
    if raw.is_absolute() {
        return Err(invalid_webgal(
            "WebGAL 媒体文件必须是应用数据目录内的相对路径",
        ));
    }
    let canonical_file = canonicalize_data_file_path(data_dir, relative_path, "WebGAL 媒体文件")
        .map_err(invalid_webgal)?;
    std::fs::read(&canonical_file).map_err(|error| {
        invalid_webgal(format!(
            "读取 WebGAL 媒体文件失败: {}: {error}",
            canonical_file.display()
        ))
    })
}

fn invalid_webgal(message: impl Into<String>) -> AppError {
    AppError::InvalidState {
        current: "webgal_export".to_string(),
        operation: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::galgame::{GalgameLine, GalgameScene};
    use crate::galgame::{GalgameMediaAsset, GalgameMediaBinding, GalgameMediaBindingSummary};
    use a4n_domain::galgame::{
        GALGAME_STORY_GRAPH_SCHEMA_VERSION, GalgameFacingV2, GalgameLinePresentationDocumentV3,
        GalgameStageLayerV2, GalgameStagePositionV2, GalgameVisibleCastMemberV2, StoryGraph,
        StoryNarrativeLine, StoryNode, StoryNodeKind,
    };
    use a4n_test_support::process::SidecarProcessTestDir;
    use serde_json::Value;

    fn scene(lines: Vec<GalgameLine>) -> GalgameSceneDetail {
        GalgameSceneDetail {
            scene: GalgameScene {
                adaptation_plan_id: None,
                adaptation_unit_id: None,
                id: 10,
                source_snapshot_chapter_id: 20,
                scene_index: 1,
                title: "测试场".to_string(),
                location: None,
                time_of_day: None,
                background_note: None,
                entry_kind: GalgameSceneEntryKind::Main,
                next_scene_id: None,
            },
            lines,
            choices: Vec::new(),
        }
    }

    fn line(id: i64, line_type: GalgameLineType, presentation_json: Value) -> GalgameLine {
        GalgameLine {
            id,
            scene_id: 10,
            line_index: 1,
            line_type,
            speaker_profile_id: None,
            text: "旁白".to_string(),
            expression: None,
            pose: None,
            position: None,
            presentation_json: serde_json::from_value(presentation_json).unwrap(),
            text_anchor_id: None,
        }
    }

    fn presentation(
        visible_cast: Vec<GalgameVisibleCastMemberV2>,
        enter: Vec<i64>,
        exit: Vec<i64>,
    ) -> Value {
        serde_json::to_value(GalgameLinePresentationDocumentV3 {
            schema_version: a4n_domain::galgame::GalgameLineDocumentSchemaVersion::V3,
            origin: a4n_domain::media_authoring::AdaptationOrigin::Historical,
            beat_ref: None,
            visible_cast,
            enter,
            exit,
            focus_character_profile_id: None,
            hold: false,
            player_knowledge: Vec::new(),
            cg: None,
            cg_note: None,
            cg_importance: None,
            transition: None,
            branch: None,
            branch_id: None,
            returns_to_scene_index: None,
            after_beat_ref: None,
        })
        .unwrap()
    }

    fn graph_with_scene_line() -> StoryGraph {
        StoryGraph {
            schema_version: GALGAME_STORY_GRAPH_SCHEMA_VERSION.into(),
            id: "galgame:1".into(),
            project_id: 1,
            source_snapshot_id: 2,
            revision: 1,
            entry_node_id: "scene:1".into(),
            terminal_node_ids: vec!["scene:1".into()],
            nodes: vec![StoryNode {
                id: "scene:1".into(),
                kind: StoryNodeKind::Terminal,
                title: "测试场".into(),
                summary: String::new(),
                primary_source: None,
                source_refs: Vec::new(),
                lines: vec![StoryNarrativeLine {
                    id: "line:1:1".into(),
                    display_order: 0,
                    line_type: "narration".into(),
                    speaker_profile_id: None,
                    text: "旁白".into(),
                    presentation: presentation(Vec::new(), Vec::new(), Vec::new()),
                }],
                state_before: Default::default(),
                state_after: Default::default(),
                state_operations: Vec::new(),
                media_intents: Vec::new(),
                layout_x: 0.0,
                layout_y: 0.0,
            }],
            edges: Vec::new(),
            branch_groups: Vec::new(),
            loop_groups: Vec::new(),
            state_variables: Vec::new(),
        }
    }

    fn media_binding(
        id: i64,
        target_kind: GalgameMediaTargetKind,
        target_id: i64,
        role: GalgameMediaRole,
    ) -> GalgameMediaBindingSummary {
        GalgameMediaBindingSummary {
            binding: GalgameMediaBinding {
                id,
                media_asset_id: id + 100,
                target_kind,
                target_id,
                role,
            },
            asset: GalgameMediaAsset {
                id: id + 100,
                media_kind: "image".into(),
                file_path: Some(format!("media/{id}.png")),
                thumbnail_path: None,
            },
        }
    }

    #[test]
    fn reads_only_relative_files_inside_the_application_data_directory() {
        let temp = SidecarProcessTestDir::new("webgal-export");
        let data_dir = temp.path().join("data");
        std::fs::create_dir_all(&data_dir).unwrap();
        std::fs::write(data_dir.join("media.png"), b"media").unwrap();
        std::fs::write(temp.path().join("outside.png"), b"outside").unwrap();

        assert_eq!(
            read_webgal_data_file(&data_dir, "media.png").unwrap(),
            b"media"
        );
        let absolute = read_webgal_data_file(&data_dir, "/tmp/outside.png").unwrap_err();
        assert!(matches!(
            absolute,
            AppError::InvalidState { operation, .. }
                if operation == "WebGAL 媒体文件必须是应用数据目录内的相对路径"
        ));
        assert!(read_webgal_data_file(&data_dir, "../outside.png").is_err());
    }

    #[test]
    fn renders_all_visible_cast_members_on_narration_lines() {
        let visible_cast = vec![
            GalgameVisibleCastMemberV2 {
                character_profile_id: 47,
                expression: "serious".to_string(),
                pose: "watch".to_string(),
                position: GalgameStagePositionV2::Left,
                facing: GalgameFacingV2::Left,
                layer: GalgameStageLayerV2::Back,
            },
            GalgameVisibleCastMemberV2 {
                character_profile_id: 48,
                expression: "neutral".to_string(),
                pose: "toolbox".to_string(),
                position: GalgameStagePositionV2::Right,
                facing: GalgameFacingV2::Right,
                layer: GalgameStageLayerV2::Front,
            },
        ];
        let mut assets = WebgalExportAssets::default();
        for (profile_id, expression, pose, file) in [
            (47, "serious", "watch", "lin.webp"),
            (48, "neutral", "toolbox", "zhou.webp"),
        ] {
            assets.figures.insert(
                WebgalFigureKey {
                    profile_id,
                    expression: Some(expression.to_string()),
                    pose: Some(pose.to_string()),
                },
                WebgalExportAsset {
                    script_name: file.to_string(),
                    zip_path: format!("game/figure/{file}"),
                    data_path: format!("portraits/{file}"),
                },
            );
        }

        let script = render_webgal_scene(
            &scene(vec![
                line(
                    100,
                    GalgameLineType::Narration,
                    presentation(visible_cast, vec![47, 48], Vec::new()),
                ),
                line(
                    101,
                    GalgameLineType::Narration,
                    presentation(
                        vec![GalgameVisibleCastMemberV2 {
                            character_profile_id: 48,
                            expression: "neutral".to_string(),
                            pose: "toolbox".to_string(),
                            position: GalgameStagePositionV2::Right,
                            facing: GalgameFacingV2::Right,
                            layer: GalgameStageLayerV2::Front,
                        }],
                        Vec::new(),
                        vec![47],
                    ),
                ),
            ]),
            &BTreeMap::new(),
            &BTreeMap::new(),
            &BTreeMap::new(),
            &assets,
            &BTreeSet::from([47, 48]),
        )
        .unwrap();

        assert!(script.contains("changeFigure:lin.webp -id=a4n_profile_47 -left"));
        assert!(script.contains("changeFigure:zhou.webp -id=a4n_profile_48 -right"));
        assert!(script.contains("-zIndex=1"));
        assert!(script.contains("-zIndex=3"));
        assert!(script.contains("-transform={\"scale\":{\"x\":-1,\"y\":1}}"));
        assert!(script.contains(
            "setTransition: -target=a4n_profile_47 -exit=exit;\nchangeFigure:none -id=a4n_profile_47 -next;"
        ));
    }

    #[test]
    fn injects_choice_text_after_webgal_command_parsing() {
        let option_text = "继续调查 - 暂缓验证:甲|乙";
        let choices = [
            GalgameChoice {
                id: 201,
                line_id: 100,
                option_index: 1,
                option_text: option_text.to_string(),
                target_scene_id: Some(20),
            },
            GalgameChoice {
                id: 202,
                line_id: 100,
                option_index: 2,
                option_text: "暂时离开".to_string(),
                target_scene_id: Some(30),
            },
        ];
        let choices_by_line = BTreeMap::from([(100, choices.iter().collect())]);
        let script = render_webgal_scene(
            &scene(vec![line(
                100,
                GalgameLineType::Choice,
                presentation(Vec::new(), Vec::new(), Vec::new()),
            )]),
            &BTreeMap::from([
                (20, "scene_002_20.txt".to_string()),
                (30, "scene_003_30.txt".to_string()),
            ]),
            &BTreeMap::new(),
            &choices_by_line,
            &WebgalExportAssets::default(),
            &BTreeSet::new(),
        )
        .unwrap();
        let escaped = webgal_choice_text(option_text).unwrap();
        let literal = webgal_string_literal(&escaped);

        assert!(script.contains(&format!("setVar:__a4n_choice_100_1=({literal});")));
        assert!(script.contains(
            "choose:{__a4n_choice_100_1}:scene_002_20.txt|{__a4n_choice_100_2}:scene_003_30.txt;"
        ));
    }

    #[test]
    fn maps_existing_scene_and_line_media_bindings_to_graph_export_ids() {
        let source = GalgameScriptDetail {
            id: 1,
            root_project_id: 2,
            title: "测试剧本".into(),
            speaker_profiles: Vec::new(),
            text_anchors: Vec::new(),
            media_bindings: vec![
                media_binding(
                    11,
                    GalgameMediaTargetKind::GalgameScene,
                    10,
                    GalgameMediaRole::Background,
                ),
                media_binding(
                    12,
                    GalgameMediaTargetKind::GalgameLine,
                    100,
                    GalgameMediaRole::Cg,
                ),
                media_binding(
                    13,
                    GalgameMediaTargetKind::GalgameScene,
                    99,
                    GalgameMediaRole::Background,
                ),
            ],
            scenes: vec![scene(vec![line(
                100,
                GalgameLineType::Narration,
                presentation(Vec::new(), Vec::new(), Vec::new()),
            )])],
            story_graph: None,
            story_graph_status: None,
        };

        let export = webgal_detail_from_story_graph(&source, &graph_with_scene_line()).unwrap();

        assert_eq!(export.media_bindings.len(), 2);
        let background = export
            .media_bindings
            .iter()
            .find(|binding| binding.binding.id == 11)
            .unwrap();
        assert_eq!(
            background.binding.target_kind,
            GalgameMediaTargetKind::GalgameScene
        );
        assert_eq!(background.binding.target_id, export.scenes[0].scene.id);
        let cg = export
            .media_bindings
            .iter()
            .find(|binding| binding.binding.id == 12)
            .unwrap();
        assert_eq!(cg.binding.target_kind, GalgameMediaTargetKind::GalgameLine);
        assert_eq!(cg.binding.target_id, export.scenes[0].lines[0].id);
    }

    #[test]
    fn rejects_choice_targets_missing_from_export_scene_map() {
        let line_id = 100;
        let mut source_scene = scene(vec![line(
            line_id,
            GalgameLineType::Choice,
            presentation(Vec::new(), Vec::new(), Vec::new()),
        )]);
        source_scene.lines[0].scene_id = source_scene.scene.id;
        source_scene.choices = (0..2)
            .map(|option| GalgameChoice {
                id: 200 + option,
                line_id,
                option_index: option as i32 + 1,
                option_text: format!("选项 {}", option + 1),
                target_scene_id: Some(999),
            })
            .collect();
        let detail = GalgameScriptDetail {
            id: 1,
            root_project_id: 1,
            title: "选择目标错误回归".into(),
            speaker_profiles: Vec::new(),
            text_anchors: Vec::new(),
            media_bindings: Vec::new(),
            scenes: vec![source_scene],
            story_graph: None,
            story_graph_status: None,
        };

        let error = validate_webgal_choice_contract(
            &detail,
            &BTreeMap::from([(10, "scene_001_10.txt".to_string())]),
        )
        .expect_err("choice target must exist in the export scene map");

        assert!(error.to_string().contains("指向不存在的场 999"));
    }

    #[tokio::test]
    #[ignore = "manual performance probe"]
    async fn webgal_choice_validation_probe() {
        use std::time::{Duration, Instant};

        for scene_count in [1_000_i64, 5_000, 10_000] {
            let mut scenes = Vec::with_capacity(scene_count as usize);
            for index in 0..scene_count {
                let scene_id = index + 1;
                let line_id = scene_id * 10;
                let mut item = scene(vec![line(
                    line_id,
                    GalgameLineType::Choice,
                    presentation(Vec::new(), Vec::new(), Vec::new()),
                )]);
                item.scene.id = scene_id;
                item.scene.scene_index = scene_id as i32;
                item.lines[0].scene_id = scene_id;
                item.choices = (0..2)
                    .map(|option| GalgameChoice {
                        id: scene_id * 10 + option + 1,
                        line_id,
                        option_index: option as i32 + 1,
                        option_text: format!("选项 {}", option + 1),
                        target_scene_id: Some(
                            (scene_count * 3 / 4 + index % (scene_count / 4)).max(1),
                        ),
                    })
                    .collect();
                scenes.push(item);
            }
            let detail = GalgameScriptDetail {
                id: 1,
                root_project_id: 1,
                title: "导出性能夹具".into(),
                speaker_profiles: Vec::new(),
                text_anchors: Vec::new(),
                media_bindings: Vec::new(),
                scenes,
                story_graph: None,
                story_graph_status: None,
            };
            let scene_file_by_id = detail
                .scenes
                .iter()
                .enumerate()
                .map(|(index, scene)| {
                    (
                        scene.scene.id,
                        format!("scene_{:03}_{}.txt", index + 1, scene.scene.id),
                    )
                })
                .collect::<BTreeMap<_, _>>();
            let mut samples = Vec::with_capacity(5);
            let warmup_until = Instant::now() + Duration::from_millis(250);
            let mut iteration = 0;
            while samples.len() < 5 {
                let started = Instant::now();
                validate_webgal_choice_contract(&detail, &scene_file_by_id)
                    .expect("valid WebGAL choice contract");
                let elapsed = started.elapsed().as_secs_f64() * 1_000.0;
                if iteration >= 2 && started >= warmup_until {
                    samples.push(elapsed);
                }
                iteration += 1;
            }
            samples.sort_by(f64::total_cmp);
            println!(
                "webgal_choice_validation scenes={} choices={} p50_ms={:.3} samples={samples:?}",
                detail.scenes.len(),
                detail
                    .scenes
                    .iter()
                    .map(|scene| scene.choices.len())
                    .sum::<usize>(),
                samples[samples.len() / 2],
            );
        }
    }
}
