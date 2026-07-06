//! Video canvas graph service.

use a4n_core::error::AppError;
use a4n_core::models::ai::{generated_image, generated_video};
use a4n_core::models::canvas::{canvas_edge, canvas_graph, canvas_node, canvas_text_asset};
use a4n_core::models::chapter::chapter_video_prompt;
use a4n_core::models::generation::{generation_spec, prompt_artifact};
use a4n_core::models::media::media_asset;
use a4n_core::models::project::novel_project;
use a4n_core::models::video_project::{assembly_timeline, video_segment};
use chrono::Utc;
use sea_orm::*;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use crate::project::{PROJECT_KIND_VIDEO, SOURCE_MODE_DERIVED, SOURCE_MODE_FREEFORM};

const GRAPH_KIND_VIDEO_STUDIO: &str = "video_studio";
const GRAPH_KIND_DERIVED_VIDEO: &str = "derived_video";
const GRAPH_KIND_TEMPLATE: &str = "template";

const NODE_STATE_READY: &str = "ready";
const CONTEXT_EDGE_ROLES: &[&str] = &[
    "context",
    "direct_content",
    "previous_context",
    "next_context",
];
const FIRST_FRAME_EDGE_ROLES: &[&str] = &["first_frame"];
const END_FRAME_EDGE_ROLES: &[&str] = &["end_frame"];
const REFERENCE_IMAGE_EDGE_ROLES: &[&str] = &[
    "character_reference",
    "object_reference",
    "scene_reference",
    "style_reference",
];
const SOURCE_VIDEO_EDGE_ROLES: &[&str] = &["source_video"];
const ASSEMBLY_CLIP_EDGE_ROLES: &[&str] = &["assembly_clip"];
const FINAL_VIDEO_EDGE_ROLES: &[&str] = &["final_video"];
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasGraphDocument {
    pub graph: canvas_graph::Model,
    pub nodes: Vec<canvas_node::Model>,
    pub edges: Vec<canvas_edge::Model>,
    pub text_assets: Vec<canvas_text_asset::Model>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasGraphInput {
    pub id: Option<i64>,
    pub name: Option<String>,
    pub graph_kind: Option<String>,
    pub viewport_json: Option<Value>,
    pub nodes: Vec<CanvasNodeInput>,
    pub edges: Vec<CanvasEdgeInput>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasNodeInput {
    pub client_id: String,
    pub id: Option<i64>,
    pub node_kind: String,
    pub ref_type: Option<String>,
    pub ref_id: Option<i64>,
    pub config_json: Option<Value>,
    pub position_json: Value,
    pub state: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CanvasEdgeInput {
    pub source_client_id: String,
    pub target_client_id: String,
    pub source_node_id: Option<i64>,
    pub target_node_id: Option<i64>,
    pub source_port: String,
    pub target_port: String,
    pub role: String,
    pub config_json: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpsertCanvasTextAssetInput {
    pub id: Option<i64>,
    pub title: String,
    pub text_kind: String,
    pub content: String,
}

#[derive(Clone)]
pub struct CanvasService {
    db: Arc<DatabaseConnection>,
}

impl CanvasService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn get_or_create_graph(
        &self,
        project_id: i64,
        graph_id: Option<i64>,
    ) -> Result<CanvasGraphDocument, AppError> {
        let project = self.require_canvas_project(project_id).await?;
        let graph = match graph_id {
            Some(id) => self.load_graph(project_id, id).await?,
            None => {
                if let Some(existing) = canvas_graph::Entity::find()
                    .filter(canvas_graph::Column::ProjectId.eq(project_id))
                    .filter(canvas_graph::Column::GraphKind.eq(default_graph_kind(&project)))
                    .order_by_asc(canvas_graph::Column::Id)
                    .one(self.db.as_ref())
                    .await
                    .map_err(AppError::Database)?
                {
                    existing
                } else {
                    self.create_default_graph(&project).await?
                }
            }
        };
        self.document_for_graph(graph).await
    }

    pub async fn save_graph(
        &self,
        project_id: i64,
        input: CanvasGraphInput,
    ) -> Result<CanvasGraphDocument, AppError> {
        let project = self.require_canvas_project(project_id).await?;
        validate_graph_input(&input)?;

        let tx = self.db.begin().await.map_err(AppError::Database)?;
        validate_graph_references(&tx, project_id, &input).await?;
        let now = Utc::now().naive_utc();
        let graph = match input.id {
            Some(graph_id) => {
                let existing = canvas_graph::Entity::find_by_id(graph_id)
                    .one(&tx)
                    .await
                    .map_err(AppError::Database)?
                    .ok_or_else(|| AppError::NotFound(format!("画布不存在: {graph_id}")))?;
                if existing.project_id != project_id {
                    return Err(AppError::Forbidden("画布不属于当前项目".to_string()));
                }
                let next_version = existing.version + 1;
                let mut active: canvas_graph::ActiveModel = existing.into();
                if let Some(name) = input
                    .name
                    .as_deref()
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                {
                    active.name = Set(name.to_string());
                }
                if let Some(kind) = input.graph_kind.as_deref() {
                    active.graph_kind = Set(normalize_graph_kind(kind)?.to_string());
                }
                active.viewport_json = Set(input.viewport_json.clone());
                active.version = Set(next_version);
                active.updated_at = Set(now);
                active.update(&tx).await.map_err(AppError::Database)?
            }
            None => canvas_graph::ActiveModel {
                project_id: Set(project_id),
                name: Set(input
                    .name
                    .as_deref()
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .unwrap_or("视频画布")
                    .to_string()),
                graph_kind: Set(input
                    .graph_kind
                    .as_deref()
                    .map(normalize_graph_kind)
                    .transpose()?
                    .unwrap_or(default_graph_kind(&project))
                    .to_string()),
                version: Set(1),
                viewport_json: Set(input.viewport_json.clone()),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(&tx)
            .await
            .map_err(AppError::Database)?,
        };

        canvas_edge::Entity::delete_many()
            .filter(canvas_edge::Column::GraphId.eq(graph.id))
            .exec(&tx)
            .await
            .map_err(AppError::Database)?;
        canvas_node::Entity::delete_many()
            .filter(canvas_node::Column::GraphId.eq(graph.id))
            .exec(&tx)
            .await
            .map_err(AppError::Database)?;

        let mut node_ids = BTreeMap::<String, i64>::new();
        for node in &input.nodes {
            let row = canvas_node::ActiveModel {
                graph_id: Set(graph.id),
                node_kind: Set(node.node_kind.trim().to_string()),
                ref_type: Set(node
                    .ref_type
                    .as_deref()
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .map(str::to_string)),
                ref_id: Set(node.ref_id),
                config_json: Set(node.config_json.clone().unwrap_or_else(|| json!({}))),
                position_json: Set(node.position_json.clone()),
                state: Set(node
                    .state
                    .as_deref()
                    .map(str::trim)
                    .filter(|v| !v.is_empty())
                    .unwrap_or(NODE_STATE_READY)
                    .to_string()),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(&tx)
            .await
            .map_err(AppError::Database)?;
            node_ids.insert(node.client_id.clone(), row.id);
        }

        for edge in &input.edges {
            let source_node_id = resolve_edge_node_id(
                edge.source_node_id,
                &edge.source_client_id,
                &node_ids,
                "source",
            )?;
            let target_node_id = resolve_edge_node_id(
                edge.target_node_id,
                &edge.target_client_id,
                &node_ids,
                "target",
            )?;
            canvas_edge::ActiveModel {
                graph_id: Set(graph.id),
                source_node_id: Set(source_node_id),
                source_port: Set(required_text(&edge.source_port, "source_port")?),
                target_node_id: Set(target_node_id),
                target_port: Set(required_text(&edge.target_port, "target_port")?),
                role: Set(required_text(&edge.role, "role")?),
                config_json: Set(edge.config_json.clone().unwrap_or_else(|| json!({}))),
                ..Default::default()
            }
            .insert(&tx)
            .await
            .map_err(AppError::Database)?;
        }

        tx.commit().await.map_err(AppError::Database)?;
        self.document_for_graph(graph).await
    }

    pub async fn upsert_text_asset(
        &self,
        project_id: i64,
        input: UpsertCanvasTextAssetInput,
    ) -> Result<canvas_text_asset::Model, AppError> {
        self.require_canvas_project(project_id).await?;
        let now = Utc::now().naive_utc();
        let title = required_text(&input.title, "title")?;
        let text_kind = normalize_text_kind(&input.text_kind)?;
        let content = input.content.trim().to_string();
        if content.is_empty() {
            return Err(AppError::InvalidArgument("content 不能为空".to_string()));
        }
        let content_hash = sha256_hex(content.as_bytes());
        match input.id {
            Some(id) => {
                let existing = canvas_text_asset::Entity::find_by_id(id)
                    .one(self.db.as_ref())
                    .await
                    .map_err(AppError::Database)?
                    .ok_or_else(|| AppError::NotFound(format!("画布文本资产不存在: {id}")))?;
                if existing.project_id != project_id {
                    return Err(AppError::Forbidden("文本资产不属于当前项目".to_string()));
                }
                let mut active: canvas_text_asset::ActiveModel = existing.into();
                active.title = Set(title);
                active.text_kind = Set(text_kind.to_string());
                active.content = Set(content);
                active.content_hash = Set(content_hash);
                active.updated_at = Set(now);
                active
                    .update(self.db.as_ref())
                    .await
                    .map_err(AppError::Database)
            }
            None => canvas_text_asset::ActiveModel {
                project_id: Set(project_id),
                title: Set(title),
                text_kind: Set(text_kind.to_string()),
                content: Set(content),
                content_hash: Set(content_hash),
                created_at: Set(now),
                updated_at: Set(now),
                ..Default::default()
            }
            .insert(self.db.as_ref())
            .await
            .map_err(AppError::Database),
        }
    }

    async fn require_canvas_project(
        &self,
        project_id: i64,
    ) -> Result<novel_project::Model, AppError> {
        let project = novel_project::Entity::find_by_id(project_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
        if project.project_kind != PROJECT_KIND_VIDEO {
            return Err(AppError::InvalidArgument(
                "画布工作台当前只支持视频项目".to_string(),
            ));
        }
        if project.source_mode != SOURCE_MODE_FREEFORM && project.source_mode != SOURCE_MODE_DERIVED
        {
            return Err(AppError::InvalidArgument(
                "视频画布只支持 derived 或 freeform 视频项目".to_string(),
            ));
        }
        Ok(project)
    }

    async fn create_default_graph(
        &self,
        project: &novel_project::Model,
    ) -> Result<canvas_graph::Model, AppError> {
        let now = Utc::now().naive_utc();
        canvas_graph::ActiveModel {
            project_id: Set(project.id),
            name: Set("视频画布".to_string()),
            graph_kind: Set(default_graph_kind(project).to_string()),
            version: Set(1),
            viewport_json: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(self.db.as_ref())
        .await
        .map_err(AppError::Database)
    }

    async fn load_graph(
        &self,
        project_id: i64,
        graph_id: i64,
    ) -> Result<canvas_graph::Model, AppError> {
        let graph = canvas_graph::Entity::find_by_id(graph_id)
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::NotFound(format!("画布不存在: {graph_id}")))?;
        if graph.project_id != project_id {
            return Err(AppError::Forbidden("画布不属于当前项目".to_string()));
        }
        Ok(graph)
    }

    async fn document_for_graph(
        &self,
        graph: canvas_graph::Model,
    ) -> Result<CanvasGraphDocument, AppError> {
        let nodes = canvas_node::Entity::find()
            .filter(canvas_node::Column::GraphId.eq(graph.id))
            .order_by_asc(canvas_node::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let edges = canvas_edge::Entity::find()
            .filter(canvas_edge::Column::GraphId.eq(graph.id))
            .order_by_asc(canvas_edge::Column::Id)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let text_assets = canvas_text_asset::Entity::find()
            .filter(canvas_text_asset::Column::ProjectId.eq(graph.project_id))
            .order_by_desc(canvas_text_asset::Column::UpdatedAt)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        Ok(CanvasGraphDocument {
            graph,
            nodes,
            edges,
            text_assets,
        })
    }
}

fn validate_graph_input(input: &CanvasGraphInput) -> Result<(), AppError> {
    validate_canvas_viewport_json(input.viewport_json.as_ref(), "canvas_graph.viewport_json")?;
    let mut client_ids = BTreeSet::new();
    let mut nodes_by_client_id = BTreeMap::<String, &CanvasNodeInput>::new();
    let mut edge_keys = BTreeSet::new();
    let mut assemble_clip_orders = BTreeMap::<String, Vec<i64>>::new();
    for node in &input.nodes {
        let client_id = required_text(&node.client_id, "node.client_id")?;
        if !client_ids.insert(client_id) {
            return Err(AppError::InvalidArgument(format!(
                "节点 client_id 重复: {}",
                node.client_id
            )));
        }
        required_text(&node.node_kind, "node.node_kind")?;
        if !node.position_json.is_object() {
            return Err(AppError::InvalidArgument(
                "node.position_json 必须是 JSON 对象".to_string(),
            ));
        }
        validate_canvas_node_ref_shape(node)?;
        nodes_by_client_id.insert(node.client_id.trim().to_string(), node);
    }
    for edge in &input.edges {
        let source_client_id = required_text(&edge.source_client_id, "edge.source_client_id")?;
        let target_client_id = required_text(&edge.target_client_id, "edge.target_client_id")?;
        if !client_ids.contains(source_client_id.as_str()) {
            return Err(AppError::InvalidArgument(format!(
                "连线 source_client_id 不存在: {}",
                edge.source_client_id
            )));
        }
        if !client_ids.contains(target_client_id.as_str()) {
            return Err(AppError::InvalidArgument(format!(
                "连线 target_client_id 不存在: {}",
                edge.target_client_id
            )));
        }
        if source_client_id == target_client_id {
            return Err(AppError::InvalidArgument(format!(
                "画布不允许节点自连: {source_client_id}"
            )));
        }
        let source_port = required_text(&edge.source_port, "edge.source_port")?;
        let target_port = required_text(&edge.target_port, "edge.target_port")?;
        let role = required_text(&edge.role, "edge.role")?;
        if !edge_keys.insert((
            source_client_id.clone(),
            target_client_id.clone(),
            source_port.clone(),
            target_port.clone(),
            role.clone(),
        )) {
            return Err(AppError::InvalidArgument("画布存在重复连线".to_string()));
        }
        let source = nodes_by_client_id
            .get(&source_client_id)
            .copied()
            .expect("source node was checked above");
        let target = nodes_by_client_id
            .get(&target_client_id)
            .copied()
            .expect("target node was checked above");
        validate_canvas_edge_contract(
            &source.node_kind,
            &source_port,
            &target.node_kind,
            &target_port,
            &role,
        )?;
        if target.node_kind == "AssembleVideoNode"
            && source_port == "video"
            && target_port == "clip"
            && role == "assembly_clip"
        {
            let order_index =
                canvas_edge_order_index(edge.config_json.as_ref(), "AssembleVideoNode")?;
            assemble_clip_orders
                .entry(target_client_id.clone())
                .or_default()
                .push(order_index);
        }
    }
    validate_contiguous_order_indexes(&assemble_clip_orders, "AssembleVideoNode")?;
    validate_graph_is_dag(input)?;
    Ok(())
}

pub fn validate_canvas_document_edge_contract(
    document: &CanvasGraphDocument,
) -> Result<(), AppError> {
    let nodes_by_id = document
        .nodes
        .iter()
        .map(|node| (node.id, node))
        .collect::<BTreeMap<_, _>>();
    let mut assemble_clip_orders = BTreeMap::<i64, Vec<i64>>::new();
    for edge in &document.edges {
        let source = nodes_by_id.get(&edge.source_node_id).ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "画布连线引用不存在的源节点: {}",
                edge.source_node_id
            ))
        })?;
        let target = nodes_by_id.get(&edge.target_node_id).ok_or_else(|| {
            AppError::InvalidArgument(format!(
                "画布连线引用不存在的目标节点: {}",
                edge.target_node_id
            ))
        })?;
        validate_canvas_edge_contract(
            &source.node_kind,
            &edge.source_port,
            &target.node_kind,
            &edge.target_port,
            &edge.role,
        )?;
        if target.node_kind == "AssembleVideoNode"
            && edge.source_port == "video"
            && edge.target_port == "clip"
            && edge.role == "assembly_clip"
        {
            let order_index =
                canvas_edge_order_index(Some(&edge.config_json), "AssembleVideoNode")?;
            assemble_clip_orders
                .entry(edge.target_node_id)
                .or_default()
                .push(order_index);
        }
    }
    validate_contiguous_order_indexes(&assemble_clip_orders, "AssembleVideoNode")?;
    Ok(())
}

pub fn validate_canvas_viewport_json(value: Option<&Value>, label: &str) -> Result<(), AppError> {
    if let Some(value) = value
        && !value.is_object()
    {
        return Err(AppError::InvalidArgument(format!(
            "{label} 必须是 JSON 对象"
        )));
    }
    Ok(())
}

pub fn validate_canvas_document_is_dag(document: &CanvasGraphDocument) -> Result<(), AppError> {
    let mut indegree = BTreeMap::<i64, usize>::new();
    let mut outgoing = BTreeMap::<i64, Vec<i64>>::new();
    for node in &document.nodes {
        indegree.insert(node.id, 0);
        outgoing.insert(node.id, Vec::new());
    }
    for edge in &document.edges {
        if let Some(value) = indegree.get_mut(&edge.target_node_id) {
            *value += 1;
        }
        outgoing
            .entry(edge.source_node_id)
            .or_default()
            .push(edge.target_node_id);
    }
    validate_dag_from_degrees(indegree, outgoing)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CanvasNodeRefKind {
    Inline,
    Config,
    CanvasTextAsset,
    PromptArtifact,
    MediaAsset,
    VideoSegment,
    AssemblyTimeline,
    GenerationSpec,
    GeneratedImage,
    GeneratedVideo,
    LegacyGeneratedImage,
    LegacyGeneratedVideo,
    LegacyChapterVideoPrompt,
}

async fn validate_graph_references<C>(
    db: &C,
    project_id: i64,
    input: &CanvasGraphInput,
) -> Result<(), AppError>
where
    C: ConnectionTrait,
{
    for node in &input.nodes {
        let Some(kind) = validate_canvas_node_ref_shape(node)? else {
            continue;
        };
        let Some(ref_id) = node.ref_id else {
            continue;
        };
        match kind {
            CanvasNodeRefKind::Inline | CanvasNodeRefKind::Config => {}
            CanvasNodeRefKind::CanvasTextAsset => {
                let row = canvas_text_asset::Entity::find_by_id(ref_id)
                    .one(db)
                    .await
                    .map_err(AppError::Database)?
                    .ok_or_else(|| AppError::NotFound(format!("画布文本资产不存在: {ref_id}")))?;
                ensure_ref_project(row.project_id, project_id, "画布文本资产", ref_id)?;
            }
            CanvasNodeRefKind::PromptArtifact => {
                let row = prompt_artifact::Entity::find_by_id(ref_id)
                    .one(db)
                    .await
                    .map_err(AppError::Database)?
                    .ok_or_else(|| AppError::NotFound(format!("提示词产物不存在: {ref_id}")))?;
                let spec = generation_spec::Entity::find_by_id(row.generation_spec_id)
                    .one(db)
                    .await
                    .map_err(AppError::Database)?
                    .ok_or_else(|| {
                        AppError::NotFound(format!(
                            "提示词产物缺少生成意图: prompt_artifact_id={ref_id}"
                        ))
                    })?;
                ensure_ref_project(spec.project_id, project_id, "提示词产物", ref_id)?;
            }
            CanvasNodeRefKind::MediaAsset => {
                let row = media_asset::Entity::find_by_id(ref_id)
                    .one(db)
                    .await
                    .map_err(AppError::Database)?
                    .ok_or_else(|| AppError::NotFound(format!("媒体资产不存在: {ref_id}")))?;
                ensure_ref_project(row.project_id, project_id, "媒体资产", ref_id)?;
            }
            CanvasNodeRefKind::VideoSegment => {
                let row = video_segment::Entity::find_by_id(ref_id)
                    .one(db)
                    .await
                    .map_err(AppError::Database)?
                    .ok_or_else(|| AppError::NotFound(format!("视频片段不存在: {ref_id}")))?;
                ensure_ref_project(row.project_id, project_id, "视频片段", ref_id)?;
            }
            CanvasNodeRefKind::AssemblyTimeline => {
                let row = assembly_timeline::Entity::find_by_id(ref_id)
                    .one(db)
                    .await
                    .map_err(AppError::Database)?
                    .ok_or_else(|| AppError::NotFound(format!("成片时间线不存在: {ref_id}")))?;
                ensure_ref_project(row.project_id, project_id, "成片时间线", ref_id)?;
            }
            CanvasNodeRefKind::GenerationSpec => {
                let row = generation_spec::Entity::find_by_id(ref_id)
                    .one(db)
                    .await
                    .map_err(AppError::Database)?
                    .ok_or_else(|| AppError::NotFound(format!("生成意图不存在: {ref_id}")))?;
                ensure_ref_project(row.project_id, project_id, "生成意图", ref_id)?;
            }
            CanvasNodeRefKind::GeneratedImage | CanvasNodeRefKind::LegacyGeneratedImage => {
                let row = generated_image::Entity::find_by_id(ref_id)
                    .one(db)
                    .await
                    .map_err(AppError::Database)?
                    .ok_or_else(|| AppError::NotFound(format!("生成图片不存在: {ref_id}")))?;
                ensure_ref_project(row.project_id, project_id, "生成图片", ref_id)?;
            }
            CanvasNodeRefKind::GeneratedVideo | CanvasNodeRefKind::LegacyGeneratedVideo => {
                let row = generated_video::Entity::find_by_id(ref_id)
                    .one(db)
                    .await
                    .map_err(AppError::Database)?
                    .ok_or_else(|| AppError::NotFound(format!("生成视频不存在: {ref_id}")))?;
                ensure_ref_project(row.project_id, project_id, "生成视频", ref_id)?;
            }
            CanvasNodeRefKind::LegacyChapterVideoPrompt => {
                let row = chapter_video_prompt::Entity::find_by_id(ref_id)
                    .one(db)
                    .await
                    .map_err(AppError::Database)?
                    .ok_or_else(|| AppError::NotFound(format!("章节视频提示词不存在: {ref_id}")))?;
                ensure_ref_project(row.project_id, project_id, "章节视频提示词", ref_id)?;
            }
        }
    }
    Ok(())
}

fn validate_canvas_edge_contract(
    source_kind: &str,
    source_port: &str,
    target_kind: &str,
    target_port: &str,
    role: &str,
) -> Result<(), AppError> {
    let source_kind = source_kind.trim();
    let source_port = source_port.trim();
    let target_kind = target_kind.trim();
    let target_port = target_port.trim();
    let role = role.trim();
    let Some(allowed_roles) =
        canvas_edge_allowed_roles(source_kind, source_port, target_kind, target_port)
    else {
        return Err(AppError::InvalidArgument(format!(
            "画布连线端口不匹配: {source_kind}.{source_port} -> {target_kind}.{target_port}"
        )));
    };
    if !allowed_roles.contains(&role) {
        return Err(AppError::InvalidArgument(format!(
            "画布连线 role 无效: {source_kind}.{source_port} -> {target_kind}.{target_port} 不允许 role={role}"
        )));
    }
    Ok(())
}

fn canvas_edge_order_index(config: Option<&Value>, label: &str) -> Result<i64, AppError> {
    let order_index = config
        .and_then(|config| config.get("order_index"))
        .and_then(Value::as_i64)
        .ok_or_else(|| {
            AppError::InvalidArgument(format!("{label} 片段连线必须设置 order_index"))
        })?;
    if order_index <= 0 {
        return Err(AppError::InvalidArgument(format!(
            "{label} 片段连线 order_index 必须是正整数"
        )));
    }
    Ok(order_index)
}

fn validate_contiguous_order_indexes<K: Ord + std::fmt::Display>(
    orders_by_target: &BTreeMap<K, Vec<i64>>,
    label: &str,
) -> Result<(), AppError> {
    for (target, orders) in orders_by_target {
        let mut seen = BTreeSet::new();
        for order in orders {
            if !seen.insert(*order) {
                return Err(AppError::InvalidArgument(format!(
                    "{label} 片段顺序重复: target={target}, order_index={order}"
                )));
            }
        }
        for expected in 1..=orders.len() as i64 {
            if !seen.contains(&expected) {
                return Err(AppError::InvalidArgument(format!(
                    "{label} 片段顺序必须从 1 连续递增: target={target}, 缺少 order_index={expected}"
                )));
            }
        }
    }
    Ok(())
}

fn canvas_edge_allowed_roles(
    source_kind: &str,
    source_port: &str,
    target_kind: &str,
    target_port: &str,
) -> Option<&'static [&'static str]> {
    if is_text_producer(source_kind) && source_port == "text" {
        return match (target_kind, target_port) {
            ("RenderTextNode", "input")
            | ("LlmTextNode", "input")
            | ("GenerateImageNode", "text")
            | ("GenerateVideoNode", "text")
            | ("TextProbeNode", "text")
            | ("TextOutputNode", "text") => Some(CONTEXT_EDGE_ROLES),
            _ => None,
        };
    }
    if is_image_producer(source_kind) && source_port == "image" {
        return match (target_kind, target_port) {
            ("GenerateVideoNode", "first_frame") => Some(FIRST_FRAME_EDGE_ROLES),
            ("GenerateVideoNode", "end_frame") => Some(END_FRAME_EDGE_ROLES),
            ("GenerateVideoNode", "reference_image") => Some(REFERENCE_IMAGE_EDGE_ROLES),
            ("ImageProbeNode", "image") | ("ImageOutputNode", "image") => Some(CONTEXT_EDGE_ROLES),
            _ => None,
        };
    }
    if is_video_producer(source_kind) && source_port == "video" {
        return match (target_kind, target_port) {
            ("ExtractFrameNode", "video") | ("TrimVideoNode", "video") => {
                Some(SOURCE_VIDEO_EDGE_ROLES)
            }
            ("AssembleVideoNode", "clip") => Some(ASSEMBLY_CLIP_EDGE_ROLES),
            ("VideoProbeNode", "video") => Some(CONTEXT_EDGE_ROLES),
            ("VideoOutputNode", "video") => Some(FINAL_VIDEO_EDGE_ROLES),
            _ => None,
        };
    }
    if is_audio_producer(source_kind) && source_port == "audio" {
        return match (target_kind, target_port) {
            ("AudioProbeNode", "audio") | ("AudioOutputNode", "audio") => Some(CONTEXT_EDGE_ROLES),
            _ => None,
        };
    }
    None
}

fn is_text_producer(kind: &str) -> bool {
    matches!(
        kind,
        "TextInputNode" | "RenderTextNode" | "LlmTextNode" | "TextProbeNode"
    )
}

fn is_image_producer(kind: &str) -> bool {
    matches!(
        kind,
        "ImageInputNode" | "GenerateImageNode" | "ExtractFrameNode" | "ImageProbeNode"
    )
}

fn is_video_producer(kind: &str) -> bool {
    matches!(
        kind,
        "VideoInputNode"
            | "GenerateVideoNode"
            | "TrimVideoNode"
            | "AssembleVideoNode"
            | "VideoProbeNode"
    )
}

fn is_audio_producer(kind: &str) -> bool {
    matches!(kind, "AudioInputNode" | "AudioProbeNode")
}

fn validate_canvas_node_ref_shape(
    node: &CanvasNodeInput,
) -> Result<Option<CanvasNodeRefKind>, AppError> {
    let Some(ref_type) = node
        .ref_type
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    else {
        if node.ref_id.is_some() {
            return Err(AppError::InvalidArgument(
                "node.ref_id 不能在 node.ref_type 为空时设置".to_string(),
            ));
        }
        return Ok(None);
    };
    let kind = parse_canvas_node_ref_kind(ref_type)?;
    match kind {
        CanvasNodeRefKind::Inline => {
            if node.ref_id.is_some() {
                return Err(AppError::InvalidArgument(
                    "inline 节点引用不能设置 ref_id".to_string(),
                ));
            }
        }
        CanvasNodeRefKind::Config => {
            if let Some(ref_id) = node.ref_id {
                ensure_positive_ref_id(ref_id, ref_type)?;
            }
        }
        _ => {
            let ref_id = node.ref_id.ok_or_else(|| {
                AppError::InvalidArgument(format!("{ref_type} 节点引用必须设置 ref_id"))
            })?;
            ensure_positive_ref_id(ref_id, ref_type)?;
        }
    }
    Ok(Some(kind))
}

fn parse_canvas_node_ref_kind(value: &str) -> Result<CanvasNodeRefKind, AppError> {
    match value.trim() {
        "inline" => Ok(CanvasNodeRefKind::Inline),
        "config" => Ok(CanvasNodeRefKind::Config),
        "canvas_text_asset" => Ok(CanvasNodeRefKind::CanvasTextAsset),
        "prompt_artifact" => Ok(CanvasNodeRefKind::PromptArtifact),
        "media_asset" => Ok(CanvasNodeRefKind::MediaAsset),
        "video_segment" => Ok(CanvasNodeRefKind::VideoSegment),
        "assembly_timeline" => Ok(CanvasNodeRefKind::AssemblyTimeline),
        "generation_spec" => Ok(CanvasNodeRefKind::GenerationSpec),
        "generated_image" => Ok(CanvasNodeRefKind::GeneratedImage),
        "generated_video" => Ok(CanvasNodeRefKind::GeneratedVideo),
        "legacy_generated_image" => Ok(CanvasNodeRefKind::LegacyGeneratedImage),
        "legacy_generated_video" => Ok(CanvasNodeRefKind::LegacyGeneratedVideo),
        "legacy_chapter_video_prompt" => Ok(CanvasNodeRefKind::LegacyChapterVideoPrompt),
        other => Err(AppError::InvalidArgument(format!(
            "未知画布节点引用类型: {other}"
        ))),
    }
}

fn ensure_positive_ref_id(ref_id: i64, ref_type: &str) -> Result<(), AppError> {
    if ref_id > 0 {
        Ok(())
    } else {
        Err(AppError::InvalidArgument(format!(
            "{ref_type} 节点引用 ref_id 必须是正整数"
        )))
    }
}

fn ensure_ref_project(
    record_project_id: i64,
    project_id: i64,
    label: &str,
    ref_id: i64,
) -> Result<(), AppError> {
    if record_project_id == project_id {
        Ok(())
    } else {
        Err(AppError::Forbidden(format!(
            "画布节点引用的{label}不属于当前项目: ref_id={ref_id}"
        )))
    }
}

fn validate_graph_is_dag(input: &CanvasGraphInput) -> Result<(), AppError> {
    let mut indegree = BTreeMap::<String, usize>::new();
    let mut outgoing = BTreeMap::<String, Vec<String>>::new();
    for node in &input.nodes {
        let client_id = node.client_id.trim().to_string();
        indegree.insert(client_id.clone(), 0);
        outgoing.insert(client_id, Vec::new());
    }
    for edge in &input.edges {
        let source = edge.source_client_id.trim().to_string();
        let target = edge.target_client_id.trim().to_string();
        if let Some(value) = indegree.get_mut(&target) {
            *value += 1;
        }
        outgoing.entry(source).or_default().push(target);
    }

    validate_dag_from_degrees(indegree, outgoing)
}

fn validate_dag_from_degrees<T>(
    mut indegree: BTreeMap<T, usize>,
    outgoing: BTreeMap<T, Vec<T>>,
) -> Result<(), AppError>
where
    T: Clone + Ord,
{
    let mut queue = indegree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(node, _)| node.clone())
        .collect::<Vec<_>>();
    let mut visited = 0usize;
    while let Some(node) = queue.pop() {
        visited += 1;
        for target in outgoing.get(&node).into_iter().flatten() {
            if let Some(degree) = indegree.get_mut(target) {
                *degree -= 1;
                if *degree == 0 {
                    queue.push(target.clone());
                }
            }
        }
    }
    if visited != indegree.len() {
        return Err(AppError::InvalidArgument(
            "画布暂只支持有向无环图，当前连线存在环路".to_string(),
        ));
    }
    Ok(())
}

fn normalize_graph_kind(value: &str) -> Result<&str, AppError> {
    match value.trim() {
        GRAPH_KIND_VIDEO_STUDIO => Ok(GRAPH_KIND_VIDEO_STUDIO),
        GRAPH_KIND_DERIVED_VIDEO => Ok(GRAPH_KIND_DERIVED_VIDEO),
        GRAPH_KIND_TEMPLATE => Ok(GRAPH_KIND_TEMPLATE),
        other => Err(AppError::InvalidArgument(format!("未知画布类型: {other}"))),
    }
}

fn default_graph_kind(project: &novel_project::Model) -> &'static str {
    if project.source_mode == SOURCE_MODE_DERIVED {
        GRAPH_KIND_DERIVED_VIDEO
    } else {
        GRAPH_KIND_VIDEO_STUDIO
    }
}

fn normalize_text_kind(value: &str) -> Result<&str, AppError> {
    match value.trim() {
        "script" => Ok("script"),
        "shot_note" => Ok("shot_note"),
        "dialogue" => Ok("dialogue"),
        "narration" => Ok("narration"),
        "character_note" => Ok("character_note"),
        "freeform" => Ok("freeform"),
        other => Err(AppError::InvalidArgument(format!(
            "未知画布文本类型: {other}"
        ))),
    }
}

fn required_text(value: &str, field: &str) -> Result<String, AppError> {
    let value = value.trim();
    if value.is_empty() {
        Err(AppError::InvalidArgument(format!("{field} 不能为空")))
    } else {
        Ok(value.to_string())
    }
}

fn resolve_edge_node_id(
    explicit_id: Option<i64>,
    client_id: &str,
    node_ids: &BTreeMap<String, i64>,
    label: &str,
) -> Result<i64, AppError> {
    let _ = explicit_id;
    node_ids
        .get(client_id.trim())
        .copied()
        .ok_or_else(|| AppError::InvalidArgument(format!("{label} 节点不存在: {client_id}")))
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(ref_type: Option<&str>, ref_id: Option<i64>) -> CanvasNodeInput {
        node_with_kind("ImageInputNode", "node-1", ref_type, ref_id, json!({}))
    }

    fn node_with_kind(
        node_kind: &str,
        client_id: &str,
        ref_type: Option<&str>,
        ref_id: Option<i64>,
        config_json: Value,
    ) -> CanvasNodeInput {
        CanvasNodeInput {
            client_id: client_id.to_string(),
            id: None,
            node_kind: node_kind.to_string(),
            ref_type: ref_type.map(str::to_string),
            ref_id,
            config_json: Some(config_json),
            position_json: json!({"x": 0, "y": 0}),
            state: None,
        }
    }

    fn graph_input(nodes: Vec<CanvasNodeInput>, edges: Vec<CanvasEdgeInput>) -> CanvasGraphInput {
        CanvasGraphInput {
            id: None,
            name: Some("测试画布".to_string()),
            graph_kind: Some(GRAPH_KIND_VIDEO_STUDIO.to_string()),
            viewport_json: None,
            nodes,
            edges,
        }
    }

    fn typed_edge(
        source: &str,
        target: &str,
        source_port: &str,
        target_port: &str,
        role: &str,
    ) -> CanvasEdgeInput {
        CanvasEdgeInput {
            source_client_id: source.to_string(),
            target_client_id: target.to_string(),
            source_node_id: None,
            target_node_id: None,
            source_port: source_port.to_string(),
            target_port: target_port.to_string(),
            role: role.to_string(),
            config_json: None,
        }
    }

    fn assemble_clip_edge(source: &str, target: &str, order_index: i64) -> CanvasEdgeInput {
        CanvasEdgeInput {
            config_json: Some(json!({
                "order_index": order_index
            })),
            ..typed_edge(source, target, "video", "clip", "assembly_clip")
        }
    }

    #[test]
    fn canvas_ref_shape_rejects_ref_id_without_ref_type() {
        let err = validate_canvas_node_ref_shape(&node(None, Some(1)))
            .expect_err("ref_id without ref_type must fail");

        assert!(err.to_string().contains("ref_type 为空"));
    }

    #[test]
    fn canvas_ref_shape_rejects_unknown_ref_type() {
        let err = validate_canvas_node_ref_shape(&node(Some("old_media"), Some(1)))
            .expect_err("unknown ref_type must fail");

        assert!(err.to_string().contains("未知画布节点引用类型"));
    }

    #[test]
    fn canvas_ref_shape_requires_id_for_project_owned_refs() {
        let err = validate_canvas_node_ref_shape(&node(Some("legacy_generated_image"), None))
            .expect_err("project-owned ref must have an id");

        assert!(err.to_string().contains("必须设置 ref_id"));
    }

    #[test]
    fn canvas_ref_shape_accepts_inline_without_id_and_config_with_positive_id() {
        let inline = validate_canvas_node_ref_shape(&node(Some("inline"), None))
            .expect("inline without ref_id should be valid");
        let config = validate_canvas_node_ref_shape(&node(Some("config"), Some(12)))
            .expect("config can carry a positive config id");

        assert_eq!(inline, Some(CanvasNodeRefKind::Inline));
        assert_eq!(config, Some(CanvasNodeRefKind::Config));
    }

    #[test]
    fn canvas_config_node_edges_are_not_part_of_four_type_contract() {
        let config = node_with_kind(
            "ConfigNode",
            "cfg",
            None,
            None,
            json!({
                "config_type": "image",
                "config_id": 7
            }),
        );
        let image = node_with_kind("GenerateImageNode", "image", None, None, json!({}));

        let err = validate_graph_input(&graph_input(
            vec![config, image],
            vec![typed_edge(
                "cfg",
                "image",
                "image_config",
                "config",
                "config",
            )],
        ))
        .expect_err("ConfigNode edges are no longer accepted");

        assert!(err.to_string().contains("端口不匹配"));
    }

    #[test]
    fn canvas_edge_contract_rejects_legacy_output_input_fallback() {
        let text = node_with_kind("TextInputNode", "text", Some("inline"), None, json!({}));
        let render = node_with_kind("RenderTextNode", "render", None, None, json!({}));

        let err = validate_graph_input(&graph_input(
            vec![text, render],
            vec![typed_edge("text", "render", "output", "input", "context")],
        ))
        .expect_err("generic output/input edge must not be accepted");

        assert!(err.to_string().contains("端口不匹配"));
    }

    #[test]
    fn canvas_edge_contract_accepts_documented_text_ports() {
        let text = node_with_kind("TextInputNode", "text", Some("inline"), None, json!({}));
        let render = node_with_kind("RenderTextNode", "render", None, None, json!({}));
        validate_graph_input(&graph_input(
            vec![text, render],
            vec![typed_edge("text", "render", "text", "input", "context")],
        ))
        .expect("documented text -> render text edge should be accepted");
    }

    #[test]
    fn canvas_edge_contract_accepts_text_producers_for_generation_nodes() {
        for source_kind in ["TextInputNode", "RenderTextNode", "LlmTextNode"] {
            for target_kind in ["GenerateImageNode", "GenerateVideoNode"] {
                let source = node_with_kind(source_kind, "source", Some("inline"), None, json!({}));
                let target = node_with_kind(target_kind, "target", None, None, json!({}));

                validate_graph_input(&graph_input(
                    vec![source, target],
                    vec![typed_edge("source", "target", "text", "text", "context")],
                ))
                .unwrap_or_else(|err| {
                    panic!("{source_kind}.text -> {target_kind}.text should be accepted: {err}")
                });
            }
        }
    }

    #[test]
    fn canvas_edge_contract_accepts_probe_passthrough_nodes() {
        let text = node_with_kind("TextInputNode", "text", Some("inline"), None, json!({}));
        let text_probe = node_with_kind("TextProbeNode", "text_probe", None, None, json!({}));
        let image = node_with_kind(
            "ImageInputNode",
            "image",
            Some("media_asset"),
            Some(1),
            json!({}),
        );
        let image_probe = node_with_kind("ImageProbeNode", "image_probe", None, None, json!({}));
        let video = node_with_kind(
            "VideoInputNode",
            "video",
            Some("media_asset"),
            Some(2),
            json!({}),
        );
        let video_probe = node_with_kind("VideoProbeNode", "video_probe", None, None, json!({}));
        let audio = node_with_kind(
            "AudioInputNode",
            "audio",
            Some("media_asset"),
            Some(3),
            json!({}),
        );
        let audio_probe = node_with_kind("AudioProbeNode", "audio_probe", None, None, json!({}));
        let image_gen = node_with_kind("GenerateImageNode", "image_gen", None, None, json!({}));
        let video_gen = node_with_kind("GenerateVideoNode", "video_gen", None, None, json!({}));
        let trim = node_with_kind("TrimVideoNode", "trim", None, None, json!({}));
        let audio_output = node_with_kind("AudioOutputNode", "audio_output", None, None, json!({}));

        validate_graph_input(&graph_input(
            vec![
                text,
                text_probe,
                image,
                image_probe,
                video,
                video_probe,
                audio,
                audio_probe,
                image_gen,
                video_gen,
                trim,
                audio_output,
            ],
            vec![
                typed_edge("text", "text_probe", "text", "text", "context"),
                typed_edge("text_probe", "image_gen", "text", "text", "context"),
                typed_edge("image", "image_probe", "image", "image", "context"),
                typed_edge(
                    "image_probe",
                    "video_gen",
                    "image",
                    "first_frame",
                    "first_frame",
                ),
                typed_edge("video", "video_probe", "video", "video", "context"),
                typed_edge("video_probe", "trim", "video", "video", "source_video"),
                typed_edge("audio", "audio_probe", "audio", "audio", "context"),
                typed_edge("audio_probe", "audio_output", "audio", "audio", "context"),
            ],
        ))
        .expect("probe nodes should pass through their own data type");
    }

    #[test]
    fn canvas_edge_contract_rejects_cross_type_probe_connections() {
        let text = node_with_kind("TextInputNode", "text", Some("inline"), None, json!({}));
        let image_probe = node_with_kind("ImageProbeNode", "image_probe", None, None, json!({}));

        let err = validate_graph_input(&graph_input(
            vec![text, image_probe],
            vec![typed_edge(
                "text",
                "image_probe",
                "text",
                "image",
                "context",
            )],
        ))
        .expect_err("text output must not feed image probe");

        assert!(err.to_string().contains("端口不匹配"));
    }

    #[test]
    fn canvas_edge_contract_accepts_text_for_llm_transform() {
        let text = node_with_kind("TextInputNode", "text", Some("inline"), None, json!({}));
        let llm = node_with_kind("LlmTextNode", "llm", None, None, json!({}));
        validate_graph_input(&graph_input(
            vec![text, llm],
            vec![typed_edge("text", "llm", "text", "input", "context")],
        ))
        .expect("TextInputNode should feed LlmTextNode input");
    }

    #[test]
    fn canvas_edge_contract_accepts_text_producers_for_render_text_input() {
        for source_kind in ["TextInputNode", "RenderTextNode", "LlmTextNode"] {
            let source = node_with_kind(source_kind, "source", Some("inline"), None, json!({}));
            let render = node_with_kind("RenderTextNode", "render", None, None, json!({}));

            validate_graph_input(&graph_input(
                vec![source, render],
                vec![typed_edge("source", "render", "text", "input", "context")],
            ))
            .unwrap_or_else(|err| {
                panic!("{source_kind}.text -> RenderTextNode.input should be accepted: {err}")
            });
        }
    }

    #[test]
    fn canvas_edge_contract_accepts_llm_transform_chaining() {
        let first = node_with_kind("LlmTextNode", "first", None, None, json!({}));
        let second = node_with_kind("LlmTextNode", "second", None, None, json!({}));

        validate_graph_input(&graph_input(
            vec![first, second],
            vec![typed_edge("first", "second", "text", "input", "context")],
        ))
        .expect("LlmTextNode output should feed another LlmTextNode");
    }

    #[test]
    fn canvas_edge_contract_uses_assembly_clip_role_for_assemble_node() {
        let video = node_with_kind(
            "VideoInputNode",
            "video",
            Some("legacy_generated_video"),
            Some(9),
            json!({}),
        );
        let assemble = node_with_kind("AssembleVideoNode", "assemble", None, None, json!({}));

        validate_graph_input(&graph_input(
            vec![video.clone(), assemble.clone()],
            vec![assemble_clip_edge("video", "assemble", 1)],
        ))
        .expect("VideoInputNode clip edges should use assembly_clip role");

        let err = validate_graph_input(&graph_input(
            vec![video, assemble],
            vec![typed_edge(
                "video",
                "assemble",
                "video",
                "clip",
                "source_video",
            )],
        ))
        .expect_err("AssembleVideoNode clip edges must not use source_video role");

        assert!(err.to_string().contains("role 无效"));
    }

    #[test]
    fn canvas_edge_contract_accepts_generator_outputs_as_downstream_sources() {
        let prompt = node_with_kind("TextInputNode", "prompt", Some("inline"), None, json!({}));
        let generate = node_with_kind("GenerateVideoNode", "generate", None, None, json!({}));
        let assemble = node_with_kind("AssembleVideoNode", "assemble", None, None, json!({}));
        let export = node_with_kind("VideoOutputNode", "export", None, None, json!({}));

        validate_graph_input(&graph_input(
            vec![prompt, generate, assemble, export],
            vec![
                typed_edge("prompt", "generate", "text", "text", "context"),
                assemble_clip_edge("generate", "assemble", 1),
                typed_edge("assemble", "export", "video", "video", "final_video"),
            ],
        ))
        .expect("generated video should flow into assemble and export sink");
    }

    #[test]
    fn canvas_edge_contract_rejects_sink_output_connections() {
        let export = node_with_kind("VideoOutputNode", "export", None, None, json!({}));
        let video = node_with_kind("GenerateVideoNode", "video", None, None, json!({}));

        let err = validate_graph_input(&graph_input(
            vec![export, video],
            vec![typed_edge("export", "video", "video", "text", "context")],
        ))
        .expect_err("sink nodes must not produce downstream edges");

        assert!(err.to_string().contains("端口不匹配"));
    }

    #[test]
    fn canvas_edge_contract_requires_explicit_assembly_clip_order() {
        let video = node_with_kind(
            "VideoInputNode",
            "video",
            Some("legacy_generated_video"),
            Some(9),
            json!({}),
        );
        let assemble = node_with_kind("AssembleVideoNode", "assemble", None, None, json!({}));

        let err = validate_graph_input(&graph_input(
            vec![video, assemble],
            vec![typed_edge(
                "video",
                "assemble",
                "video",
                "clip",
                "assembly_clip",
            )],
        ))
        .expect_err("AssembleVideoNode clip edges must carry order_index");

        assert!(err.to_string().contains("必须设置 order_index"));
    }

    #[test]
    fn canvas_edge_contract_rejects_duplicate_assembly_clip_order() {
        let first_video = node_with_kind(
            "VideoInputNode",
            "video-1",
            Some("legacy_generated_video"),
            Some(9),
            json!({}),
        );
        let second_video = node_with_kind(
            "VideoInputNode",
            "video-2",
            Some("legacy_generated_video"),
            Some(10),
            json!({}),
        );
        let assemble = node_with_kind("AssembleVideoNode", "assemble", None, None, json!({}));

        let err = validate_graph_input(&graph_input(
            vec![first_video, second_video, assemble],
            vec![
                assemble_clip_edge("video-1", "assemble", 1),
                assemble_clip_edge("video-2", "assemble", 1),
            ],
        ))
        .expect_err("AssembleVideoNode clip order_index must be unique per target");

        assert!(err.to_string().contains("片段顺序重复"));
    }

    #[test]
    fn canvas_edge_contract_rejects_invalid_role_for_port_pair() {
        let image = node_with_kind(
            "ImageInputNode",
            "image",
            Some("legacy_generated_image"),
            Some(9),
            json!({}),
        );
        let video = node_with_kind("GenerateVideoNode", "video", None, None, json!({}));

        let err = validate_graph_input(&graph_input(
            vec![image, video],
            vec![typed_edge(
                "image",
                "video",
                "image",
                "first_frame",
                "context",
            )],
        ))
        .expect_err("first_frame edge must require first_frame role");

        assert!(err.to_string().contains("role 无效"));
    }

    #[test]
    fn canvas_edge_contract_accepts_generate_video_end_frame_input() {
        let image = node_with_kind(
            "ImageInputNode",
            "image",
            Some("legacy_generated_image"),
            Some(9),
            json!({}),
        );
        let video = node_with_kind("GenerateVideoNode", "video", None, None, json!({}));

        validate_graph_input(&graph_input(
            vec![image, video],
            vec![typed_edge(
                "image",
                "video",
                "image",
                "end_frame",
                "end_frame",
            )],
        ))
        .expect("GenerateVideoNode end_frame should accept image input with end_frame role");
    }
}
