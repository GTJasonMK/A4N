use super::ProjectService;

use a4n_core::error::AppError;
use a4n_core::models::{
    ai::{generated_image, generated_video},
    asset::object_profile,
    chapter::{chapter, chapter_manga_prompt, chapter_outline, chapter_video_prompt, part_outline},
    character::{character_portrait, protagonist_profile},
    generation::{generation_run, generation_run_target, generation_spec, prompt_artifact},
    media::{media_asset, media_provenance},
    project::{blueprint_version, novel_blueprint, novel_project, project_derivation},
    scene::scene_profile,
    user::conversation,
};
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

mod builder;
mod details;
mod scope;

use self::builder::{GraphBuilder, NodeSeedMetadata, node};
use self::details::*;
use self::scope::load_asset_graph_scope;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetDependencyGraph {
    pub project_id: i64,
    pub generated_at: String,
    pub nodes: Vec<AssetGraphNode>,
    pub edges: Vec<AssetGraphEdge>,
    pub layers: Vec<AssetGraphLayer>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetGraphNode {
    pub id: String,
    pub kind: String,
    pub asset_type: String,
    pub label: String,
    pub count: u64,
    pub status: Option<String>,
    pub layer: i32,
    pub chapter_id: Option<i64>,
    pub profile_id: Option<i64>,
    pub record_id: Option<i64>,
    pub editable: bool,
    pub mutable: bool,
    pub direct_downstream_count: u64,
    pub downstream_count: u64,
    pub mutation_policy: String,
    pub needs_visualization: bool,
    pub visualized: bool,
    pub summary: Option<String>,
    pub details: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetGraphEdge {
    pub from: String,
    pub to: String,
    pub relation: String,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetGraphLayer {
    pub index: i32,
    pub label: String,
    pub node_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetGraphNodeDetail {
    pub node_id: String,
    pub title: String,
    pub kind: String,
    pub asset_type: String,
    pub summary: Option<String>,
    pub records: Vec<AssetLineageRecord>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetLineageRecord {
    pub id: String,
    pub label: String,
    pub asset_kind: String,
    pub asset_id: Option<i64>,
    pub status: Option<String>,
    pub origin: String,
    pub source_refs: Vec<AssetLineageRef>,
    pub prompt_sources: Vec<PromptSource>,
    pub media_sources: Vec<AssetLineageRef>,
    pub request_snapshot: Option<Value>,
    pub input_status: String,
    pub generation_runs: Vec<GenerationRunSnapshot>,
    pub generation_inputs: Vec<GenerationInputSource>,
    pub result: Value,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AssetLineageRef {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub asset_type: String,
    pub record_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptSource {
    pub id: String,
    pub kind: String,
    pub text: String,
    pub source_record_id: Option<i64>,
    pub edited_by_user: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationRunSnapshot {
    pub id: Option<i64>,
    pub workflow: String,
    pub step_key: String,
    pub title: String,
    pub model_kind: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub status: String,
    pub created_at: Option<String>,
    pub completed_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerationInputSource {
    pub id: String,
    pub run_id: Option<i64>,
    pub group: String,
    pub kind: String,
    pub label: String,
    pub content_type: String,
    pub text: Option<String>,
    pub value: Option<Value>,
    pub source_record_id: Option<i64>,
}

const VIDEO_SHOT_ANALYSIS_PROMPT_TYPES: &[&str] = &[
    "frame_shot_analysis",
    "first_frame_shot_analysis",
    "chat_video_shot_analysis",
];
const VIDEO_SEGMENT_PLAN_PROMPT_TYPES: &[&str] = &[
    "frame_segment_plan",
    "first_frame_segment_plan",
    "chat_video_segment_plan",
];
const VIDEO_FRAME_PROMPT_TYPES: &[&str] = &[
    "frame_prompt",
    "first_frame_prompt",
    "chat_video_frame_prompt",
];
const CHAPTER_VIDEO_PROMPT_TYPES: &[&str] = &["video_prompt"];

impl ProjectService {
    pub async fn asset_dependency_graph(
        &self,
        project_id: i64,
    ) -> Result<AssetDependencyGraph, AppError> {
        let project = novel_project::Entity::find_by_id(project_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
        let scope = load_asset_graph_scope(self.db.as_ref(), project).await?;
        let root_project_id = scope.root_project.id;
        let family_project_ids = scope.family_project_ids.clone();

        let conversations = conversation::Entity::find()
            .filter(conversation::Column::ProjectId.eq(root_project_id))
            .order_by_asc(conversation::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let blueprints = novel_blueprint::Entity::find()
            .filter(novel_blueprint::Column::ProjectId.eq(root_project_id))
            .order_by_asc(novel_blueprint::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let blueprint_versions = blueprint_version::Entity::find()
            .filter(blueprint_version::Column::ProjectId.eq(root_project_id))
            .order_by_asc(blueprint_version::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let part_outlines = part_outline::Entity::find()
            .filter(part_outline::Column::ProjectId.eq(root_project_id))
            .order_by_asc(part_outline::Column::PartNumber)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let chapter_outlines = chapter_outline::Entity::find()
            .filter(chapter_outline::Column::ProjectId.eq(root_project_id))
            .order_by_asc(chapter_outline::Column::ChapterNumber)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let chapters = chapter::Entity::find()
            .filter(chapter::Column::ProjectId.eq(root_project_id))
            .filter(chapter::Column::Status.ne(chapter::STATUS_DELETED))
            .order_by_asc(chapter::Column::ChapterNumber)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let character_profiles = protagonist_profile::Entity::find()
            .filter(protagonist_profile::Column::ProjectId.eq(root_project_id))
            .order_by_asc(protagonist_profile::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let object_profiles = object_profile::Entity::find()
            .filter(object_profile::Column::ProjectId.eq(root_project_id))
            .order_by_asc(object_profile::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let scene_profiles = scene_profile::Entity::find()
            .filter(scene_profile::Column::ProjectId.eq(root_project_id))
            .order_by_asc(scene_profile::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let portraits = character_portrait::Entity::find()
            .filter(character_portrait::Column::ProjectId.is_in(family_project_ids.clone()))
            .order_by_asc(character_portrait::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let manga_prompts = chapter_manga_prompt::Entity::find()
            .filter(chapter_manga_prompt::Column::ProjectId.is_in(family_project_ids.clone()))
            .order_by_asc(chapter_manga_prompt::Column::ChapterId)
            .order_by_asc(chapter_manga_prompt::Column::PageNumber)
            .order_by_asc(chapter_manga_prompt::Column::PanelNumber)
            .order_by_asc(chapter_manga_prompt::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let video_prompts = chapter_video_prompt::Entity::find()
            .filter(chapter_video_prompt::Column::ProjectId.is_in(family_project_ids.clone()))
            .order_by_asc(chapter_video_prompt::Column::ChapterId)
            .order_by_asc(chapter_video_prompt::Column::WorkflowMode)
            .order_by_asc(chapter_video_prompt::Column::PromptType)
            .order_by_asc(chapter_video_prompt::Column::ShotNumber)
            .order_by_asc(chapter_video_prompt::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let generated_images = generated_image::Entity::find()
            .filter(generated_image::Column::ProjectId.is_in(family_project_ids.clone()))
            .order_by_asc(generated_image::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let generated_videos = generated_video::Entity::find()
            .filter(generated_video::Column::ProjectId.is_in(family_project_ids.clone()))
            .order_by_asc(generated_video::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let generation_specs = generation_spec::Entity::find()
            .filter(generation_spec::Column::ProjectId.is_in(family_project_ids.clone()))
            .order_by_asc(generation_spec::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let prompt_artifacts = if generation_specs.is_empty() {
            Vec::new()
        } else {
            prompt_artifact::Entity::find()
                .filter(
                    prompt_artifact::Column::GenerationSpecId
                        .is_in(generation_specs.iter().map(|row| row.id)),
                )
                .order_by_asc(prompt_artifact::Column::Id)
                .all(self.db.as_ref())
                .await
                .map_err(AppError::Database)?
        };
        let media_assets = media_asset::Entity::find()
            .filter(media_asset::Column::ProjectId.is_in(family_project_ids.clone()))
            .order_by_asc(media_asset::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let media_provenance = if media_assets.is_empty() {
            Vec::new()
        } else {
            media_provenance::Entity::find()
                .filter(
                    media_provenance::Column::MediaAssetId
                        .is_in(media_assets.iter().map(|row| row.id)),
                )
                .order_by_asc(media_provenance::Column::MediaAssetId)
                .all(self.db.as_ref())
                .await
                .map_err(AppError::Database)?
        };

        let mut graph = GraphBuilder::default();
        graph.add_node(node(
            "project",
            "text",
            "project",
            &scope.root_project.title,
            1,
            0,
            NodeSeedMetadata {
                status: Some(scope.root_project.status.clone()),
                editable: false,
                needs_visualization: true,
                visualized: true,
                summary: Some(
                    "项目根节点，承载从灵感到正文、图像、漫画和视频的完整派生链路。".to_string(),
                ),
                details: json!({
                    "project_kind": scope.root_project.project_kind,
                    "project_type": scope.root_project.project_type,
                    "target_chapters": scope.root_project.target_chapters,
                    "current_chapter": scope.root_project.current_chapter,
                    "graph_root_project_id": root_project_id,
                    "family_project_ids": family_project_ids,
                }),
            },
        ));

        add_inspiration_nodes(&mut graph, &conversations);
        add_story_nodes(
            &mut graph,
            &blueprints,
            &blueprint_versions,
            &part_outlines,
            &chapter_outlines,
            &chapters,
        );
        add_profile_nodes(
            &mut graph,
            &character_profiles,
            &object_profiles,
            &scene_profiles,
            &portraits,
            &generated_images,
        );
        add_manga_nodes(&mut graph, &chapters, &manga_prompts, &generated_images);
        add_video_nodes(
            &mut graph,
            &chapters,
            &video_prompts,
            &generated_images,
            &generated_videos,
        );
        add_generation_nodes(
            &mut graph,
            &generation_specs,
            &prompt_artifacts,
            &media_assets,
            &media_provenance,
        );
        add_project_derivation_nodes(
            &mut graph,
            &scope.derived_projects,
            &scope.project_derivations,
        );

        graph.finish(root_project_id)
    }

    pub async fn asset_dependency_graph_node_detail(
        &self,
        project_id: i64,
        node_id: &str,
    ) -> Result<AssetGraphNodeDetail, AppError> {
        let requested_project = novel_project::Entity::find_by_id(project_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
        let scope = load_asset_graph_scope(self.db.as_ref(), requested_project).await?;
        let root_project_id = scope.root_project.id;
        let family_project_ids = scope.family_project_ids.clone();
        let graph = self.asset_dependency_graph(project_id).await?;
        let node = graph
            .nodes
            .iter()
            .find(|node| node.id == node_id)
            .cloned()
            .ok_or_else(|| AppError::NotFound(format!("依赖图节点不存在: {node_id}")))?;
        let structural_source_refs = structural_source_refs_for_node(&graph, &node.id);
        let mut warnings = Vec::new();
        let mut records = match node.asset_type.as_str() {
            "novel_blueprint" => {
                blueprint_detail_records(self.db.as_ref(), root_project_id).await?
            }
            "blueprint_version" => {
                blueprint_version_detail_records(self.db.as_ref(), root_project_id).await?
            }
            "part_outline" => {
                part_outline_detail_records(self.db.as_ref(), root_project_id).await?
            }
            "chapter_outline" => {
                chapter_outline_detail_records(self.db.as_ref(), root_project_id).await?
            }
            "chapter_content" => {
                chapter_content_detail_records(self.db.as_ref(), root_project_id).await?
            }
            "chapter_analysis" => {
                chapter_analysis_detail_records(self.db.as_ref(), root_project_id).await?
            }
            "character_profile_record" => {
                character_profile_detail_records(self.db.as_ref(), root_project_id, node.record_id)
                    .await?
            }
            "object_profile_record" => {
                object_profile_detail_records(self.db.as_ref(), root_project_id, node.record_id)
                    .await?
            }
            "scene_profile_record" => {
                scene_profile_detail_records(self.db.as_ref(), root_project_id, node.record_id)
                    .await?
            }
            "video_shot_analysis" => {
                video_prompt_detail_records(
                    self.db.as_ref(),
                    &family_project_ids,
                    VIDEO_SHOT_ANALYSIS_PROMPT_TYPES,
                )
                .await?
            }
            "video_segment_plan" => {
                video_prompt_detail_records(
                    self.db.as_ref(),
                    &family_project_ids,
                    VIDEO_SEGMENT_PLAN_PROMPT_TYPES,
                )
                .await?
            }
            "character_portrait" => {
                character_portrait_detail_records(
                    self.db.as_ref(),
                    &family_project_ids,
                    node.profile_id,
                )
                .await?
            }
            "object_material_image" => {
                profile_material_image_detail_records(
                    self.db.as_ref(),
                    &family_project_ids,
                    node.profile_id,
                    crate::profile_material::PROFILE_KIND_OBJECT,
                )
                .await?
            }
            "scene_material_image" => {
                profile_material_image_detail_records(
                    self.db.as_ref(),
                    &family_project_ids,
                    node.profile_id,
                    crate::profile_material::PROFILE_KIND_SCENE,
                )
                .await?
            }
            "chapter_manga_prompt" => {
                manga_prompt_detail_records(self.db.as_ref(), &family_project_ids).await?
            }
            "manga_page_image" => {
                manga_image_detail_records(self.db.as_ref(), &family_project_ids, &mut warnings)
                    .await?
            }
            "video_frame_prompt" => {
                video_prompt_detail_records(
                    self.db.as_ref(),
                    &family_project_ids,
                    VIDEO_FRAME_PROMPT_TYPES,
                )
                .await?
            }
            "video_frame_image" => {
                video_frame_image_detail_records(
                    self.db.as_ref(),
                    &family_project_ids,
                    &mut warnings,
                )
                .await?
            }
            "chapter_video_prompt" => {
                video_prompt_detail_records(
                    self.db.as_ref(),
                    &family_project_ids,
                    CHAPTER_VIDEO_PROMPT_TYPES,
                )
                .await?
            }
            "generated_video" => {
                generated_video_detail_records(self.db.as_ref(), &family_project_ids, &mut warnings)
                    .await?
            }
            "generation_spec" => {
                generation_spec_detail_records(self.db.as_ref(), &family_project_ids).await?
            }
            "prompt_artifact" => {
                prompt_artifact_detail_records(self.db.as_ref(), &family_project_ids).await?
            }
            "media_asset" => {
                media_asset_detail_records(self.db.as_ref(), &family_project_ids, &mut warnings)
                    .await?
            }
            _ => generic_node_detail_records(&node, structural_source_refs.clone()),
        };
        for record in &mut records {
            if record.source_refs.is_empty() {
                record.source_refs = structural_source_refs.clone();
            }
        }
        enrich_lineage_records_with_generation_inputs(self.db.as_ref(), &mut records).await?;

        Ok(AssetGraphNodeDetail {
            node_id: node.id,
            title: node.label,
            kind: node.kind,
            asset_type: node.asset_type,
            summary: node.summary,
            records,
            warnings,
        })
    }
}

fn add_inspiration_nodes(graph: &mut GraphBuilder, conversations: &[conversation::Model]) {
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

fn add_story_nodes(
    graph: &mut GraphBuilder,
    blueprints: &[novel_blueprint::Model],
    blueprint_versions: &[blueprint_version::Model],
    part_outlines: &[part_outline::Model],
    chapter_outlines: &[chapter_outline::Model],
    chapters: &[chapter::Model],
) {
    graph.add_node(node(
        "blueprint",
        "text",
        "novel_blueprint",
        "故事蓝图",
        blueprints.len() as u64,
        2,
        NodeSeedMetadata {
            status: blueprints.first().map(|row| row.current_version.clone()),
            editable: true,
            needs_visualization: true,
            visualized: true,
            summary: blueprints
                .first()
                .and_then(|row| row.one_sentence_summary.clone()),
            details: json!({ "version_count": blueprint_versions.len() }),
        },
    ));
    graph.add_edge(
        "inspiration",
        "blueprint",
        "derives",
        blueprints.len() as u64,
    );

    if !blueprint_versions.is_empty() {
        graph.add_node(node(
            "blueprint_versions",
            "text",
            "blueprint_version",
            "蓝图版本",
            blueprint_versions.len() as u64,
            3,
            NodeSeedMetadata {
                status: blueprint_versions.last().map(|row| row.version.clone()),
                editable: false,
                needs_visualization: true,
                visualized: true,
                summary: Some("蓝图快照历史，用于对比和恢复。".to_string()),
                details: json!({}),
            },
        ));
        graph.add_edge(
            "blueprint",
            "blueprint_versions",
            "snapshots",
            blueprint_versions.len() as u64,
        );
    }

    graph.add_node(node(
        "part_outlines",
        "text",
        "part_outline",
        "分部大纲",
        part_outlines.len() as u64,
        3,
        NodeSeedMetadata {
            status: None,
            editable: true,
            needs_visualization: true,
            visualized: true,
            summary: outline_range_summary(
                part_outlines.first().map(|row| row.start_chapter),
                part_outlines.last().map(|row| row.end_chapter),
            ),
            details: json!({}),
        },
    ));
    graph.add_edge(
        "blueprint",
        "part_outlines",
        "derives",
        part_outlines.len() as u64,
    );

    graph.add_node(node(
        "chapter_outlines",
        "text",
        "chapter_outline",
        "章节大纲",
        chapter_outlines.len() as u64,
        4,
        NodeSeedMetadata {
            status: None,
            editable: true,
            needs_visualization: true,
            visualized: true,
            summary: outline_range_summary(
                chapter_outlines.first().map(|row| row.chapter_number),
                chapter_outlines.last().map(|row| row.chapter_number),
            ),
            details: json!({}),
        },
    ));
    if !part_outlines.is_empty() {
        graph.add_edge(
            "part_outlines",
            "chapter_outlines",
            "derives",
            chapter_outlines.len() as u64,
        );
    } else {
        graph.add_edge(
            "blueprint",
            "chapter_outlines",
            "derives",
            chapter_outlines.len() as u64,
        );
    }

    let word_count = chapters
        .iter()
        .map(|row| row.word_count.max(0) as u64)
        .sum::<u64>();
    graph.add_node(node(
        "chapters",
        "text",
        "chapter_content",
        "章节正文",
        chapters.len() as u64,
        5,
        NodeSeedMetadata {
            status: None,
            editable: true,
            needs_visualization: true,
            visualized: true,
            summary: Some(format!(
                "{} 章正文，总字数约 {}。",
                chapters.len(),
                word_count
            )),
            details: json!({ "word_count": word_count }),
        },
    ));
    graph.add_edge(
        "chapter_outlines",
        "chapters",
        "derives",
        chapters.len() as u64,
    );

    let analysis_count = chapters
        .iter()
        .filter(|row| {
            row.analysis_data
                .as_ref()
                .is_some_and(|value| !value.is_null())
        })
        .count();
    graph.add_node(node(
        "chapter_analysis",
        "text",
        "chapter_analysis",
        "章节分析",
        analysis_count as u64,
        6,
        NodeSeedMetadata {
            status: None,
            editable: true,
            needs_visualization: true,
            visualized: true,
            summary: Some(
                "章节正文的结构化分析，是角色、场景、物体、漫画和视频链路的重要上游。".to_string(),
            ),
            details: json!({}),
        },
    ));
    graph.add_edge(
        "chapters",
        "chapter_analysis",
        "analyzes",
        analysis_count as u64,
    );
}

fn add_profile_nodes(
    graph: &mut GraphBuilder,
    character_profiles: &[protagonist_profile::Model],
    object_profiles: &[object_profile::Model],
    scene_profiles: &[scene_profile::Model],
    portraits: &[character_portrait::Model],
    generated_images: &[generated_image::Model],
) {
    let portrait_counts = portrait_counts_by_profile(portraits);
    let object_image_counts = material_image_counts_by_profile(
        generated_images,
        crate::profile_material::PROFILE_KIND_OBJECT,
    );
    let scene_image_counts = material_image_counts_by_profile(
        generated_images,
        crate::profile_material::PROFILE_KIND_SCENE,
    );

    graph.add_node(node(
        "character_profiles",
        "text",
        "character_profile",
        "人物档案",
        character_profiles.len() as u64,
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
        character_profiles.len() as u64,
    );
    for profile in character_profiles {
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

    graph.add_node(node(
        "object_profiles",
        "text",
        "object_profile",
        "物体档案",
        object_profiles.len() as u64,
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
        object_profiles.len() as u64,
    );
    for profile in object_profiles {
        let node_id = format!("object_profile:{}", profile.id);
        let image_count = object_image_counts.get(&profile.id).copied().unwrap_or(0);
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
        if image_count > 0 {
            let images_node = format!("object_material_images:{}", profile.id);
            graph.add_node(
                node(
                    &images_node,
                    "image",
                    "object_material_image",
                    &format!("{}素材图", profile.name),
                    image_count,
                    9,
                    NodeSeedMetadata {
                        status: None,
                        editable: false,
                        needs_visualization: true,
                        visualized: true,
                        summary: Some(format!(
                            "物体档案「{}」派生的 {} 张图片。",
                            profile.name, image_count
                        )),
                        details: json!({}),
                    },
                )
                .profile_id(profile.id),
            );
            graph.add_edge(&node_id, &images_node, "generates", image_count);
        }
    }

    graph.add_node(node(
        "scene_profiles",
        "text",
        "scene_profile",
        "场景档案",
        scene_profiles.len() as u64,
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
        scene_profiles.len() as u64,
    );
    for profile in scene_profiles {
        let node_id = format!("scene_profile:{}", profile.id);
        let image_count = scene_image_counts.get(&profile.id).copied().unwrap_or(0);
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
        if image_count > 0 {
            let images_node = format!("scene_material_images:{}", profile.id);
            graph.add_node(
                node(
                    &images_node,
                    "image",
                    "scene_material_image",
                    &format!("{}素材图", profile.name),
                    image_count,
                    9,
                    NodeSeedMetadata {
                        status: None,
                        editable: false,
                        needs_visualization: true,
                        visualized: true,
                        summary: Some(format!(
                            "场景档案「{}」派生的 {} 张图片。",
                            profile.name, image_count
                        )),
                        details: json!({}),
                    },
                )
                .profile_id(profile.id),
            );
            graph.add_edge(&node_id, &images_node, "generates", image_count);
        }
    }
}

fn portrait_counts_by_profile(portraits: &[character_portrait::Model]) -> BTreeMap<i64, u64> {
    let mut counts = BTreeMap::new();
    for portrait in portraits {
        *counts.entry(portrait.profile_id).or_insert(0) += 1;
    }
    counts
}

fn material_image_counts_by_profile(
    generated_images: &[generated_image::Model],
    profile_kind: &str,
) -> BTreeMap<i64, u64> {
    let mut counts = BTreeMap::new();
    for image in generated_images {
        let Some((kind, profile_id)) =
            crate::profile_material::generated_image_profile_material_identity(image)
        else {
            continue;
        };
        if kind == profile_kind {
            *counts.entry(profile_id).or_insert(0) += 1;
        }
    }
    counts
}

fn add_manga_nodes(
    graph: &mut GraphBuilder,
    chapters: &[chapter::Model],
    manga_prompts: &[chapter_manga_prompt::Model],
    generated_images: &[generated_image::Model],
) {
    let prompt_stats = manga_prompt_stats(manga_prompts);
    graph.add_node(node(
        "manga_prompts",
        "prompt",
        "chapter_manga_prompt",
        "漫画中间提示词",
        prompt_stats.visible_count,
        8,
        NodeSeedMetadata {
            status: None,
            editable: true,
            needs_visualization: true,
            visualized: true,
            summary: Some("漫画信息抽取、分页规划和分格绘图提示词。".to_string()),
            details: json!({
                "chapter_count": prompt_stats.chapter_count(),
                "prompt_types": prompt_stats.prompt_type_counts,
            }),
        },
    ));
    if !chapters.is_empty() {
        graph.add_edge(
            "chapters",
            "manga_prompts",
            "derives",
            prompt_stats.visible_count,
        );
    }
    if graph.node_count("character_profiles") > 0 {
        graph.add_edge(
            "character_profiles",
            "manga_prompts",
            "references",
            prompt_stats.visible_count,
        );
    }

    let image_type_manga_count = generated_images
        .iter()
        .filter(|row| row.image_type == "manga_page")
        .count() as u64;
    let manga_image_count = prompt_stats.linked_image_count.max(image_type_manga_count);
    graph.add_node(node(
        "manga_images",
        "image",
        "manga_page_image",
        "漫画图片",
        manga_image_count,
        9,
        NodeSeedMetadata {
            status: None,
            editable: false,
            needs_visualization: true,
            visualized: true,
            summary: Some("由漫画分格提示词生成或绑定的页面/分格图片。".to_string()),
            details: json!({
                "linked_prompt_count": prompt_stats.linked_image_count,
                "image_type_count": image_type_manga_count,
            }),
        },
    ));
    graph.add_edge(
        "manga_prompts",
        "manga_images",
        "generates",
        manga_image_count,
    );
}

struct MangaPromptStats {
    visible_count: u64,
    linked_image_count: u64,
    chapter_ids: BTreeSet<i64>,
    prompt_type_counts: BTreeMap<String, u64>,
}

impl MangaPromptStats {
    fn chapter_count(&self) -> usize {
        self.chapter_ids.len()
    }
}

fn manga_prompt_stats(manga_prompts: &[chapter_manga_prompt::Model]) -> MangaPromptStats {
    let mut stats = MangaPromptStats {
        visible_count: 0,
        linked_image_count: 0,
        chapter_ids: BTreeSet::new(),
        prompt_type_counts: BTreeMap::new(),
    };

    for row in manga_prompts {
        if row.prompt_type != "pipeline_status" {
            stats.visible_count += 1;
        }
        if row.generated_image_id.is_some() {
            stats.linked_image_count += 1;
        }
        stats.chapter_ids.insert(row.chapter_id);
        count_string_value(&mut stats.prompt_type_counts, &row.prompt_type);
    }

    stats
}

fn add_video_nodes(
    graph: &mut GraphBuilder,
    chapters: &[chapter::Model],
    video_prompts: &[chapter_video_prompt::Model],
    generated_images: &[generated_image::Model],
    generated_videos: &[generated_video::Model],
) {
    let prompt_stats = video_prompt_stats(video_prompts);
    graph.add_node(node(
        "video_shot_analysis",
        "text",
        "video_shot_analysis",
        "视频镜头分析",
        prompt_stats.shot_analysis_count,
        8,
        NodeSeedMetadata {
            status: None,
            editable: true,
            needs_visualization: true,
            visualized: true,
            summary: Some("由章节正文、章节分析和可用素材生成的镜头分析中间产物。".to_string()),
            details: json!({
                "workflow_modes": prompt_stats.workflow_mode_counts,
            }),
        },
    ));
    if !chapters.is_empty() {
        graph.add_edge(
            "chapters",
            "video_shot_analysis",
            "derives",
            prompt_stats.shot_analysis_count,
        );
    }
    if graph.node_count("character_profiles") > 0 {
        graph.add_edge(
            "character_profiles",
            "video_shot_analysis",
            "references",
            prompt_stats.shot_analysis_count,
        );
    }
    if graph.node_count("object_profiles") > 0 {
        graph.add_edge(
            "object_profiles",
            "video_shot_analysis",
            "references",
            prompt_stats.shot_analysis_count,
        );
    }
    if graph.node_count("scene_profiles") > 0 {
        graph.add_edge(
            "scene_profiles",
            "video_shot_analysis",
            "references",
            prompt_stats.shot_analysis_count,
        );
    }

    graph.add_node(node(
        "video_segment_plans",
        "text",
        "video_segment_plan",
        "视频分段规划",
        prompt_stats.segment_plan_count,
        9,
        NodeSeedMetadata {
            status: None,
            editable: true,
            needs_visualization: true,
            visualized: true,
            summary: Some(
                "镜头分析之后生成的片段规划，决定每个视频片段的意图、节奏和首尾帧数量。"
                    .to_string(),
            ),
            details: json!({}),
        },
    ));
    if let Some(source) = first_available_graph_source(&[
        ("video_shot_analysis", prompt_stats.shot_analysis_count),
        ("chapters", chapters.len() as u64),
    ]) {
        graph.add_edge(
            source,
            "video_segment_plans",
            "derives",
            prompt_stats.segment_plan_count,
        );
    }

    graph.add_node(node(
        "video_frame_prompts",
        "prompt",
        "video_frame_prompt",
        "视频首尾帧提示词",
        prompt_stats.frame_prompt_count,
        10,
        NodeSeedMetadata {
            status: None,
            editable: true,
            needs_visualization: true,
            visualized: true,
            summary: Some("由片段规划派生的边界关键帧/首帧图片提示词。".to_string()),
            details: json!({}),
        },
    ));
    if let Some(source) = first_available_graph_source(&[
        ("video_segment_plans", prompt_stats.segment_plan_count),
        ("video_shot_analysis", prompt_stats.shot_analysis_count),
        ("chapters", chapters.len() as u64),
    ]) {
        graph.add_edge(
            source,
            "video_frame_prompts",
            "derives",
            prompt_stats.frame_prompt_count,
        );
    }

    let frame_image_count = generated_images
        .iter()
        .filter(|image| video_frame_image_prompt_id(image.metadata_json.as_ref()).is_some())
        .count() as u64;
    graph.add_node(node(
        "video_frame_images",
        "image",
        "video_frame_image",
        "视频首尾帧图片",
        frame_image_count,
        11,
        NodeSeedMetadata {
            status: None,
            editable: false,
            needs_visualization: true,
            visualized: true,
            summary: Some("由首尾帧提示词生成的视频边界帧图片。".to_string()),
            details: json!({}),
        },
    ));
    if let Some(source) = first_available_graph_source(&[
        ("video_frame_prompts", prompt_stats.frame_prompt_count),
        ("video_segment_plans", prompt_stats.segment_plan_count),
        ("video_shot_analysis", prompt_stats.shot_analysis_count),
    ]) {
        graph.add_edge(source, "video_frame_images", "generates", frame_image_count);
    }

    graph.add_node(node(
        "chapter_video_prompts",
        "prompt",
        "chapter_video_prompt",
        "视频生成提示词",
        prompt_stats.chapter_video_prompt_count,
        12,
        NodeSeedMetadata {
            status: None,
            editable: true,
            needs_visualization: true,
            visualized: true,
            summary: Some(
                "由正文、镜头分析、分段规划、首尾帧提示词和参考素材派生的视频生成提示词。"
                    .to_string(),
            ),
            details: json!({}),
        },
    ));
    if let Some(source) = first_available_graph_source(&[
        ("video_frame_images", frame_image_count),
        ("video_frame_prompts", prompt_stats.frame_prompt_count),
        ("video_segment_plans", prompt_stats.segment_plan_count),
        ("video_shot_analysis", prompt_stats.shot_analysis_count),
        ("chapters", chapters.len() as u64),
    ]) {
        graph.add_edge(
            source,
            "chapter_video_prompts",
            "derives",
            prompt_stats.chapter_video_prompt_count,
        );
    }

    let generated_video_count = generated_videos.len() as u64;
    graph.add_node(node(
        "generated_videos",
        "video",
        "generated_video",
        "生成视频",
        generated_video_count,
        13,
        NodeSeedMetadata {
            status: None,
            editable: false,
            needs_visualization: true,
            visualized: true,
            summary: Some("由视频生成提示词、首尾帧和参考素材生成或导入的视频成品。".to_string()),
            details: json!({
                "linked_prompt_count": prompt_stats.linked_video_count,
                "statuses": count_strings(generated_videos.iter().map(|row| row.status.as_str())),
            }),
        },
    ));
    graph.add_edge(
        "chapter_video_prompts",
        "generated_videos",
        "generates",
        prompt_stats.linked_video_count.max(generated_video_count),
    );
}

struct VideoPromptStats {
    shot_analysis_count: u64,
    segment_plan_count: u64,
    frame_prompt_count: u64,
    chapter_video_prompt_count: u64,
    linked_video_count: u64,
    workflow_mode_counts: BTreeMap<String, u64>,
}

fn video_prompt_stats(video_prompts: &[chapter_video_prompt::Model]) -> VideoPromptStats {
    let mut stats = VideoPromptStats {
        shot_analysis_count: 0,
        segment_plan_count: 0,
        frame_prompt_count: 0,
        chapter_video_prompt_count: 0,
        linked_video_count: 0,
        workflow_mode_counts: BTreeMap::new(),
    };

    for row in video_prompts {
        if prompt_type_in(row, VIDEO_SHOT_ANALYSIS_PROMPT_TYPES) {
            stats.shot_analysis_count += 1;
        }
        if prompt_type_in(row, VIDEO_SEGMENT_PLAN_PROMPT_TYPES) {
            stats.segment_plan_count += 1;
        }
        if prompt_type_in(row, VIDEO_FRAME_PROMPT_TYPES) {
            stats.frame_prompt_count += 1;
        }
        if prompt_type_in(row, CHAPTER_VIDEO_PROMPT_TYPES) {
            stats.chapter_video_prompt_count += 1;
        }
        if row.generated_video_id.is_some() {
            stats.linked_video_count += 1;
        }
        count_string_value(&mut stats.workflow_mode_counts, &row.workflow_mode);
    }

    stats
}

fn add_generation_nodes(
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

fn add_project_derivation_nodes(
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

fn count_strings<'a>(values: impl Iterator<Item = &'a str>) -> BTreeMap<String, u64> {
    let mut counts = BTreeMap::new();
    for value in values {
        count_string_value(&mut counts, value);
    }
    counts
}

fn count_string_value(counts: &mut BTreeMap<String, u64>, value: &str) {
    let value = value.trim();
    if value.is_empty() {
        return;
    }
    *counts.entry(value.to_string()).or_insert(0) += 1;
}

fn prompt_type_in(row: &chapter_video_prompt::Model, prompt_types: &[&str]) -> bool {
    prompt_type_str_in(&row.prompt_type, prompt_types)
}

fn prompt_type_str_in(prompt_type: &str, prompt_types: &[&str]) -> bool {
    prompt_types.contains(&prompt_type)
}

fn first_available_graph_source<'a>(sources: &[(&'a str, u64)]) -> Option<&'a str> {
    sources
        .iter()
        .find_map(|(node_id, count)| (*count > 0).then_some(*node_id))
}

fn outline_range_summary(start: Option<i32>, end: Option<i32>) -> Option<String> {
    match (start, end) {
        (Some(start), Some(end)) => Some(format!("覆盖第 {start} 到第 {end} 章。")),
        _ => None,
    }
}

fn video_frame_image_prompt_id(metadata: Option<&Value>) -> Option<i64> {
    let metadata = metadata?;
    metadata
        .get("generation_context")
        .and_then(Value::as_object)
        .and_then(|context| context.get("prompt_id"))
        .and_then(Value::as_i64)
        .or_else(|| metadata.get("prompt_id").and_then(Value::as_i64))
        .filter(|id| *id > 0)
}

#[cfg(test)]
mod tests;
