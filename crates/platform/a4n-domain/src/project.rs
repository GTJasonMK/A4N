use chrono::NaiveDateTime;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

pub fn normalized_blueprint_identity(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

pub fn blueprint_character_identity_keys<'a>(
    names: impl IntoIterator<Item = &'a str>,
) -> Result<Vec<String>, String> {
    let mut seen = std::collections::HashSet::new();
    names
        .into_iter()
        .map(|name| {
            let key = normalized_blueprint_identity(name);
            if key.is_empty() {
                return Err("蓝图角色名称不能为空".to_string());
            }
            if !seen.insert(key.clone()) {
                return Err(format!("蓝图角色重复: {name}"));
            }
            Ok(key)
        })
        .collect()
}

pub fn blueprint_relationship_identity_keys<'a>(
    relationships: impl IntoIterator<Item = (&'a str, &'a str, &'a str)>,
) -> Result<Vec<String>, String> {
    let mut seen = std::collections::HashSet::new();
    relationships
        .into_iter()
        .map(|(kind, a, b)| {
            if !BLUEPRINT_RELATIONSHIP_TYPES.contains(&kind) {
                return Err(format!(
                    "蓝图关系 relationship_type 无效：{kind}；必须是以下值之一：{}",
                    BLUEPRINT_RELATIONSHIP_TYPES.join(", ")
                ));
            }
            let keys = [kind, a, b].map(normalized_blueprint_identity);
            if keys.iter().any(String::is_empty) {
                return Err("蓝图关系的角色和类型不能为空".to_string());
            }
            let key = keys.join("|");
            if !seen.insert(key.clone()) {
                return Err(format!("蓝图关系重复: {kind} / {a} / {b}"));
            }
            Ok(key)
        })
        .collect()
}

/// 一页制分页：page 从 1 起，计算 offset。
pub fn project_list_offset(page: u64, limit: u64) -> Result<u64, String> {
    page.checked_sub(1)
        .and_then(|page_index| page_index.checked_mul(limit))
        .ok_or_else(|| "项目分页参数过大".to_string())
}

/// 必填文本：trim 后非空。
pub fn required_text<'a>(value: &'a str, field: &str) -> Result<&'a str, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{field} 不能为空"))
    } else {
        Ok(value)
    }
}

/// 可选文本：None 或空白视为空串。
pub fn optional_text(value: Option<&str>) -> String {
    value.map(str::trim).unwrap_or_default().to_string()
}
use serde_json::{Value, json};

#[derive(Debug, thiserror::Error)]
pub enum ProjectMediaCopyError {
    #[error("媒体源路径无效：{0}")]
    InvalidSourcePath(String),
    #[error("媒体文件缺少扩展名: {path}")]
    MissingExtension { path: String },
    #[error("无法创建媒体复制目录 {}: {source}", path.display())]
    CreateDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error(
        "复制媒体文件失败: {} -> {}: {source}",
        source_path.display(),
        target_path.display()
    )]
    CopyFile {
        source_path: PathBuf,
        target_path: PathBuf,
        #[source]
        source: std::io::Error,
    },
}

pub trait ProjectMediaFileCopier {
    fn copy_media_file(
        &mut self,
        new_project_id: i64,
        source_media_asset_id: i64,
        field: &str,
        relative_path: Option<&str>,
    ) -> Result<Option<String>, ProjectMediaCopyError>;
}

pub const BLUEPRINT_MIN_TOTAL_CHAPTERS: i32 = 1;
pub const BLUEPRINT_MIN_GENERATION_CHAPTERS: i32 = 5;
pub const BLUEPRINT_MAX_TOTAL_CHAPTERS: i32 = 10_000;
pub const PART_OUTLINE_THRESHOLD: i32 = 50;
pub const CHAPTERS_PER_PART: i32 = 25;

pub const BLUEPRINT_GENERATION_KIND: &str = "blueprint_generation";
pub const BLUEPRINT_BASE_PROMPT: &str = "blueprint";
pub const BLUEPRINT_STAGE_SYSTEM_PROMPT: &str = "blueprint_stage_system";
pub const BLUEPRINT_STAGE_USER_PROMPT: &str = "blueprint_stage_user";
pub const BLUEPRINT_GENERATION_PROMPTS: [&str; 3] = [
    BLUEPRINT_BASE_PROMPT,
    BLUEPRINT_STAGE_SYSTEM_PROMPT,
    BLUEPRINT_STAGE_USER_PROMPT,
];
pub const BLUEPRINT_REFINEMENT_TASK_KIND: &str = "blueprint_refinement";
pub const BLUEPRINT_REFINEMENT_WORKFLOW_KIND: &str = "blueprint_refine";
pub const BLUEPRINT_REFINEMENT_MODE: &str = "refine";
pub const BLUEPRINT_REFINEMENT_PROMPT: &str = "blueprint_optimization";
pub const BLUEPRINT_REFINEMENT_USER_PROMPT: &str = "blueprint_optimization_user";
pub const BLUEPRINT_REFINEMENT_PROMPTS: [&str; 2] = [
    BLUEPRINT_REFINEMENT_PROMPT,
    BLUEPRINT_REFINEMENT_USER_PROMPT,
];
pub const BLUEPRINT_REFINEMENT_FIELDS: &[&str] = &[
    "style",
    "full_synopsis",
    "world_setting",
    "characters",
    "relationships",
];

/// 章节资料变更后需要由 Host 处理的派生数据清理结果。
///
/// 该投影只描述事实记录 ID 和变更标记；数据库摘要到此结构的
/// 转换仍由 Orchestrator 负责。
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct ChapterDerivedCleanupArtifacts {
    pub blueprint_facts_changed: bool,
    pub character_facts_changed: bool,
    pub stale_character_state_ids: Vec<i64>,
    pub stale_attribute_event_ids: Vec<i64>,
    pub changed_foreshadowing_ids: Vec<i64>,
    pub scene_records_changed: bool,
}

impl ChapterDerivedCleanupArtifacts {
    pub fn merge(&mut self, other: Self) {
        self.blueprint_facts_changed |= other.blueprint_facts_changed;
        self.character_facts_changed |= other.character_facts_changed;
        self.stale_character_state_ids
            .extend(other.stale_character_state_ids);
        self.stale_attribute_event_ids
            .extend(other.stale_attribute_event_ids);
        self.changed_foreshadowing_ids
            .extend(other.changed_foreshadowing_ids);
        self.scene_records_changed |= other.scene_records_changed;

        self.stale_character_state_ids.sort_unstable();
        self.stale_character_state_ids.dedup();
        self.stale_attribute_event_ids.sort_unstable();
        self.stale_attribute_event_ids.dedup();
        self.changed_foreshadowing_ids.sort_unstable();
        self.changed_foreshadowing_ids.dedup();
    }
}

pub const BLUEPRINT_RELATIONSHIP_TYPES: &[&str] = &[
    "family", "friend", "enemy", "lover", "rival", "mentor", "other",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlueprintRecord {
    pub id: i64,
    pub project_id: i64,
    pub title: String,
    pub current_version: String,
    pub total_chapters: Option<i32>,
    pub chapters_per_part: Option<i32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectSummaryRecord {
    pub id: i64,
    pub user_id: i64,
    pub title: String,
    pub description: String,
    pub project_type: String,
    pub project_kind: String,
    pub root_project_id: Option<i64>,
    pub source_snapshot_id: Option<i64>,
    pub source_sync_status: String,
    pub status: String,
    pub lifecycle_gate: String,
    pub target_chapters: i32,
    pub current_chapter: i32,
    pub total_chapters: Option<i32>,
    pub chapters_per_part: Option<i32>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl ProjectSummaryRecord {
    pub fn source_mode(&self) -> Result<ProjectSourceMode, ProjectSourceModeError> {
        ProjectSourceMode::from_identity(
            &self.project_kind,
            self.root_project_id,
            self.source_snapshot_id,
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectOptimizationContextRecord {
    pub id: i64,
    pub title: String,
    pub description: String,
    pub project_type: String,
    pub status: String,
    pub target_chapters: i32,
    pub current_chapter: i32,
    pub total_chapters: Option<i32>,
    pub chapters_per_part: Option<i32>,
    pub raw_blueprint: Option<String>,
    pub export_config: Option<String>,
}

pub const BLUEPRINT_RAW_JSON_STREAM_ID: &str = "raw_json";
pub const BLUEPRINT_GENERATE_PROGRESS_TOTAL: u32 = 12;

pub fn blueprint_refinement_call_key(project_id: i64) -> String {
    format!("{BLUEPRINT_REFINEMENT_WORKFLOW_KIND}:project:{project_id}")
}

pub fn blueprint_stage_call_key(stage_key: &str) -> String {
    format!("{BLUEPRINT_GENERATION_KIND}.{stage_key}")
}

pub fn blueprint_refinement_business_key(project_id: i64) -> String {
    format!("{BLUEPRINT_REFINEMENT_WORKFLOW_KIND}:{project_id}")
}

pub fn blueprint_generation_input_fingerprint(
    project_id: i64,
    title: &str,
    genre: &str,
    target_length: i32,
    variables: &Value,
) -> Value {
    json!({
        "version": 1,
        "task_kind": BLUEPRINT_GENERATION_KIND,
        "project_id": project_id,
        "title": title,
        "genre": genre,
        "target_length": target_length,
        "variables": variables,
    })
}

pub fn blueprint_refinement_input_fingerprint(
    project_id: i64,
    focus: Option<&str>,
    variables: &Value,
) -> Value {
    json!({
        "version": 1,
        "task_kind": BLUEPRINT_REFINEMENT_TASK_KIND,
        "project_id": project_id,
        "focus": normalized_blueprint_refinement_focus(focus),
        "variables": variables,
    })
}

pub fn normalized_blueprint_refinement_focus(focus: Option<&str>) -> Option<&str> {
    focus.map(str::trim).filter(|value| !value.is_empty())
}

pub fn validate_blueprint_chapter_count(value: i32, context: &str) -> Result<(), String> {
    if is_valid_blueprint_generation_chapters(value) {
        Ok(())
    } else {
        Err(format!(
            "{context} 必须在 {BLUEPRINT_MIN_GENERATION_CHAPTERS} 到 {BLUEPRINT_MAX_TOTAL_CHAPTERS} 之间"
        ))
    }
}

pub fn build_blueprint_generation_payload(
    novel_title: &str,
    genre: &str,
    target_chapters: i32,
    variables: &std::collections::HashMap<String, Value>,
) -> Result<Value, String> {
    validate_blueprint_chapter_count(target_chapters, "requested_chapter_count")?;

    let mut payload = serde_json::Map::new();
    payload.insert(
        "novel_title".to_string(),
        Value::String(novel_title.to_string()),
    );
    payload.insert("genre".to_string(), Value::String(genre.to_string()));
    payload.insert(
        "requested_chapter_count".to_string(),
        Value::from(target_chapters),
    );
    for key in [
        "inspiration_transcript",
        "generation_instruction",
        "conversation_state",
        "progress_summary",
        "next_question",
        "project",
    ] {
        if let Some(value) = variables.get(key) {
            payload.insert(key.to_string(), value.clone());
        }
    }
    Ok(Value::Object(payload))
}

pub fn increment_blueprint_version(current: &str) -> Result<String, String> {
    let (major, minor) = current
        .split_once('.')
        .ok_or_else(|| format!("蓝图版本号 {current:?} 必须使用 major.minor 格式"))?;
    if major.is_empty() || minor.is_empty() || minor.contains('.') {
        return Err(format!("蓝图版本号 {current:?} 必须使用 major.minor 格式"));
    }
    let major = major
        .parse::<u32>()
        .map_err(|_| format!("蓝图版本号 {current:?} 的 major 必须是非负整数"))?;
    let minor = minor
        .parse::<u32>()
        .map_err(|_| format!("蓝图版本号 {current:?} 的 minor 必须是非负整数"))?;
    let next_minor = minor
        .checked_add(1)
        .ok_or_else(|| format!("蓝图版本号 {current:?} 的 minor 已超出可递增范围"))?;
    Ok(format!("{major}.{next_minor}"))
}

pub fn project_status_after_blueprint_generation(previous: ProjectStatus) -> ProjectStatus {
    previous.max(ProjectStatus::BlueprintReady)
}

#[derive(Clone, Debug)]
pub struct ParsedBlueprint {
    pub title: String,
    pub genre: String,
    pub style: String,
    pub tone: String,
    pub target_audience: String,
    pub one_sentence_summary: String,
    pub full_synopsis: String,
    pub world_setting: Value,
    pub total_chapters: i32,
    pub needs_part_outlines: bool,
    pub chapters_per_part: i32,
    pub characters: Vec<ParsedBlueprintCharacter>,
    pub relationships: Vec<ParsedBlueprintRelationship>,
}

#[derive(Clone, Debug)]
pub struct ParsedBlueprintCharacter {
    pub name: String,
    pub character_type: String,
    pub gender: Option<String>,
    pub age: Option<String>,
    pub appearance: String,
    pub personality: String,
    pub background: String,
    pub motivation: String,
    pub arc_summary: String,
    pub relationship_to_protagonist: String,
    pub arc_stages: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct ParsedBlueprintRelationship {
    pub relationship_type: String,
    pub character_a: String,
    pub character_b: String,
    pub description: String,
    pub strength: Option<i32>,
    pub evolution: Option<Value>,
}

#[derive(Clone, Debug)]
pub struct ParsedBlueprintOptimization {
    pub raw: Value,
    pub style: Option<String>,
    pub full_synopsis: Option<String>,
    pub world_setting: Option<Value>,
    pub characters: Option<Vec<ParsedBlueprintCharacter>>,
    pub relationships: Option<Vec<ParsedBlueprintRelationship>>,
}

pub const fn is_valid_blueprint_total_chapters(total_chapters: i32) -> bool {
    total_chapters >= BLUEPRINT_MIN_TOTAL_CHAPTERS && total_chapters <= BLUEPRINT_MAX_TOTAL_CHAPTERS
}

pub const fn is_valid_blueprint_generation_chapters(total_chapters: i32) -> bool {
    total_chapters >= BLUEPRINT_MIN_GENERATION_CHAPTERS
        && total_chapters <= BLUEPRINT_MAX_TOTAL_CHAPTERS
}

pub const fn needs_part_outlines_for_chapters(total_chapters: i32) -> bool {
    total_chapters > PART_OUTLINE_THRESHOLD
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProjectKind {
    Text,
    Comic,
    Video,
    Galgame,
}

impl ProjectKind {
    pub const ALL: [Self; 4] = [Self::Text, Self::Comic, Self::Video, Self::Galgame];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Comic => "comic",
            Self::Video => "video",
            Self::Galgame => "galgame",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|kind| kind.as_str() == value)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ProjectSourceMode {
    #[serde(rename = "self")]
    Self_,
    Derived,
    Freeform,
}

impl ProjectSourceMode {
    pub fn from_identity(
        project_kind: &str,
        root_project_id: Option<i64>,
        source_snapshot_id: Option<i64>,
    ) -> Result<Self, ProjectSourceModeError> {
        let project_kind = ProjectKind::parse(project_kind).ok_or_else(|| {
            ProjectSourceModeError::InvalidProjectKind {
                value: project_kind.to_string(),
            }
        })?;
        match (
            project_kind,
            root_project_id.is_some(),
            source_snapshot_id.is_some(),
        ) {
            (ProjectKind::Text, false, false) => Ok(Self::Self_),
            (ProjectKind::Video, false, false) => Ok(Self::Freeform),
            (ProjectKind::Comic | ProjectKind::Video | ProjectKind::Galgame, true, true) => {
                Ok(Self::Derived)
            }
            _ => Err(ProjectSourceModeError::InvalidIdentity {
                project_kind,
                root_project_id,
                source_snapshot_id,
            }),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ProjectSourceModeError {
    #[error("项目族无效: {value}")]
    InvalidProjectKind { value: String },
    #[error(
        "项目来源身份无效: project_kind={project_kind:?}, root_project_id={root_project_id:?}, source_snapshot_id={source_snapshot_id:?}"
    )]
    InvalidIdentity {
        project_kind: ProjectKind,
        root_project_id: Option<i64>,
        source_snapshot_id: Option<i64>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("未知项目状态：{0}")]
pub struct ProjectStatusParseError(pub String);

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum ProjectStatus {
    Draft,
    BlueprintReady,
    PartOutlinesReady,
    ChapterOutlinesReady,
    Writing,
    Completed,
}

impl ProjectStatus {
    pub const ALL: [Self; 6] = [
        Self::Draft,
        Self::BlueprintReady,
        Self::PartOutlinesReady,
        Self::ChapterOutlinesReady,
        Self::Writing,
        Self::Completed,
    ];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::BlueprintReady => "blueprint_ready",
            Self::PartOutlinesReady => "part_outlines_ready",
            Self::ChapterOutlinesReady => "chapter_outlines_ready",
            Self::Writing => "writing",
            Self::Completed => "completed",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|status| status.as_str() == value)
    }

    pub fn parse_storage(value: &str) -> Result<Self, ProjectStatusParseError> {
        Self::parse(value).ok_or_else(|| ProjectStatusParseError(value.to_string()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectStatusHistoryInput {
    pub project_id: i64,
    pub from_status: ProjectStatus,
    pub to_status: ProjectStatus,
    pub reason: String,
    pub changed_by: String,
    pub changed_at: NaiveDateTime,
}

/// Data categories that must be removed when a project status rolls back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CleanupAction {
    DeleteChapters,
    DeleteChapterOutlines,
    DeletePartOutlines,
    DeleteBlueprintVersions,
}

pub fn rollback_actions(from: ProjectStatus, to: ProjectStatus) -> Vec<CleanupAction> {
    use CleanupAction::*;

    if from <= to {
        return Vec::new();
    }

    let mut actions = Vec::new();
    if from >= ProjectStatus::Writing && to < ProjectStatus::Writing {
        actions.push(DeleteChapters);
    }
    if from >= ProjectStatus::PartOutlinesReady && to < ProjectStatus::ChapterOutlinesReady {
        actions.push(DeleteChapterOutlines);
    }
    if from >= ProjectStatus::PartOutlinesReady && to < ProjectStatus::PartOutlinesReady {
        actions.push(DeletePartOutlines);
    }
    if to == ProjectStatus::Draft {
        actions.push(DeleteBlueprintVersions);
    }
    actions
}

impl std::fmt::Display for ProjectStatus {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Draft => "草稿",
            Self::BlueprintReady => "蓝图就绪",
            Self::PartOutlinesReady => "分部大纲就绪",
            Self::ChapterOutlinesReady => "章节大纲就绪",
            Self::Writing => "写作中",
            Self::Completed => "已完成",
        })
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ProjectStatusArtifacts {
    pub has_blueprint: bool,
    pub has_part_outline: bool,
    pub has_chapter_outline: bool,
    pub has_chapter_content: bool,
}

impl ProjectStatusArtifacts {
    pub const fn supports(self, status: ProjectStatus) -> bool {
        match status {
            ProjectStatus::Draft => true,
            ProjectStatus::BlueprintReady => self.has_blueprint,
            ProjectStatus::PartOutlinesReady => self.has_part_outline,
            ProjectStatus::ChapterOutlinesReady => self.has_chapter_outline,
            ProjectStatus::Writing | ProjectStatus::Completed => self.has_chapter_content,
        }
    }

    pub const fn furthest_status(self) -> ProjectStatus {
        if self.has_chapter_content {
            ProjectStatus::Writing
        } else if self.has_chapter_outline {
            ProjectStatus::ChapterOutlinesReady
        } else if self.has_part_outline {
            ProjectStatus::PartOutlinesReady
        } else if self.has_blueprint {
            ProjectStatus::BlueprintReady
        } else {
            ProjectStatus::Draft
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectStatusHistorySummary {
    pub id: i64,
    pub from_status: ProjectStatus,
    pub to_status: ProjectStatus,
    pub reason: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectStatusMachineSummary {
    pub current_status: ProjectStatus,
    pub forward_targets: Vec<ProjectStatus>,
    pub rollback_targets: Vec<ProjectStatus>,
    pub history: Vec<ProjectStatusHistorySummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectRollbackImpactSummary {
    pub title: String,
    pub count: i32,
    pub chapter_start: Option<i32>,
    pub chapter_end: Option<i32>,
    pub part_start: Option<i32>,
    pub part_end: Option<i32>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectRollbackPreview {
    pub path: Vec<ProjectStatus>,
    pub impacts: Vec<ProjectRollbackImpactSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub struct ProjectProgressSummary {
    pub label: String,
    pub detail: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SourceSnapshotChapterSummary {
    pub id: i64,
    pub snapshot_id: i64,
    pub root_project_id: i64,
    pub source_chapter_id: i64,
    pub chapter_number: i32,
    pub title: Option<String>,
    pub summary: Option<String>,
    pub content: String,
    pub analysis_data_json: Option<Value>,
    pub content_hash: String,
    pub analysis_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AssetDependencyGraph {
    pub project_id: i64,
    pub nodes: Vec<AssetGraphNode>,
    pub edges: Vec<AssetGraphEdge>,
    pub layers: Vec<AssetGraphLayer>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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
    pub mutable: bool,
    pub direct_downstream_count: u64,
    pub downstream_count: u64,
    pub mutation_policy: String,
    pub needs_visualization: bool,
    pub visualized: bool,
    pub summary: Option<String>,
    pub details: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AssetGraphEdge {
    pub from: String,
    pub to: String,
    pub relation: String,
    pub count: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AssetGraphLayer {
    pub index: i32,
    pub label: String,
    pub node_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AssetGraphNodeDetail {
    pub node_id: String,
    pub title: String,
    pub kind: String,
    pub asset_type: String,
    pub summary: Option<String>,
    pub records: Vec<AssetLineageRecord>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
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
    pub generation_inputs: Vec<GenerationInputSource>,
    pub result: Value,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AssetLineageRef {
    pub id: String,
    pub label: String,
    pub kind: String,
    pub asset_type: String,
    pub record_id: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct PromptSource {
    pub id: String,
    pub kind: String,
    pub text: String,
    pub source_record_id: Option<i64>,
    pub edited_by_user: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct GenerationInputSource {
    pub group: String,
    pub kind: String,
    pub label: String,
    pub value: Value,
}

#[cfg(test)]
mod tests {
    use super::{
        BLUEPRINT_MAX_TOTAL_CHAPTERS, BLUEPRINT_MIN_GENERATION_CHAPTERS,
        BLUEPRINT_MIN_TOTAL_CHAPTERS, CleanupAction, PART_OUTLINE_THRESHOLD, ProjectSourceMode,
        ProjectStatus, ProjectStatusArtifacts, build_blueprint_generation_payload,
        increment_blueprint_version, is_valid_blueprint_generation_chapters,
        is_valid_blueprint_total_chapters, needs_part_outlines_for_chapters,
        project_status_after_blueprint_generation, rollback_actions,
        validate_blueprint_chapter_count,
    };
    use serde_json::json;

    #[test]
    fn project_source_mode_is_derived_from_the_identity_shape() {
        assert_eq!(
            ProjectSourceMode::from_identity("text", None, None),
            Ok(ProjectSourceMode::Self_)
        );
        assert_eq!(
            ProjectSourceMode::from_identity("video", None, None),
            Ok(ProjectSourceMode::Freeform)
        );
        assert_eq!(
            ProjectSourceMode::from_identity("video", Some(1), Some(2)),
            Ok(ProjectSourceMode::Derived)
        );
        assert!(ProjectSourceMode::from_identity("comic", None, None).is_err());
        assert!(ProjectSourceMode::from_identity("text", Some(1), Some(2)).is_err());
    }

    #[test]
    fn project_status_order_matches_all_and_artifact_progress() {
        assert!(ProjectStatus::ALL.windows(2).all(|pair| pair[0] < pair[1]));
        assert_eq!(
            ProjectStatusArtifacts {
                has_blueprint: true,
                has_part_outline: true,
                has_chapter_outline: true,
                has_chapter_content: true,
            }
            .furthest_status(),
            ProjectStatus::Writing
        );
        assert!(
            ProjectStatusArtifacts {
                has_chapter_content: true,
                ..Default::default()
            }
            .supports(ProjectStatus::Completed)
        );
    }

    #[test]
    fn part_outline_threshold_is_strictly_greater_than_limit() {
        assert!(!needs_part_outlines_for_chapters(PART_OUTLINE_THRESHOLD));
        assert!(needs_part_outlines_for_chapters(PART_OUTLINE_THRESHOLD + 1));
    }

    #[test]
    fn blueprint_chapter_ranges_distinguish_persistence_from_generation() {
        assert!(is_valid_blueprint_total_chapters(
            BLUEPRINT_MIN_TOTAL_CHAPTERS
        ));
        assert!(!is_valid_blueprint_generation_chapters(
            BLUEPRINT_MIN_GENERATION_CHAPTERS - 1
        ));
        assert!(is_valid_blueprint_generation_chapters(
            BLUEPRINT_MIN_GENERATION_CHAPTERS
        ));
        assert!(!is_valid_blueprint_total_chapters(
            BLUEPRINT_MAX_TOTAL_CHAPTERS + 1
        ));
    }

    #[test]
    fn project_status_storage_parser_accepts_only_canonical_values() {
        assert_eq!(
            ProjectStatus::parse_storage("blueprint_ready"),
            Ok(ProjectStatus::BlueprintReady)
        );
        for value in ["Draft", " blueprint_ready ", "unknown"] {
            assert!(ProjectStatus::parse_storage(value).is_err(), "{value}");
        }
    }

    #[test]
    fn rollback_actions_match_the_destructive_status_edges() {
        let all_statuses = &ProjectStatus::ALL;
        let edge_count = all_statuses
            .iter()
            .flat_map(|from| all_statuses.iter().map(move |to| (*from, *to)))
            .filter(|(from, to)| !rollback_actions(*from, *to).is_empty())
            .count();
        assert_eq!(edge_count, 14);
        assert_eq!(
            rollback_actions(ProjectStatus::Writing, ProjectStatus::Draft),
            vec![
                CleanupAction::DeleteChapters,
                CleanupAction::DeleteChapterOutlines,
                CleanupAction::DeletePartOutlines,
                CleanupAction::DeleteBlueprintVersions,
            ]
        );
        assert!(rollback_actions(ProjectStatus::Completed, ProjectStatus::Writing).is_empty());
        assert!(rollback_actions(ProjectStatus::Draft, ProjectStatus::Writing).is_empty());
    }

    #[test]
    fn blueprint_rules_have_one_domain_owner() {
        assert_eq!(increment_blueprint_version("1.9"), Ok("1.10".to_string()));
        assert!(increment_blueprint_version("1.0.0").is_err());
        assert_eq!(
            project_status_after_blueprint_generation(ProjectStatus::Writing),
            ProjectStatus::Writing
        );
        assert!(validate_blueprint_chapter_count(4, "target").is_err());
        let variables = std::collections::HashMap::from([
            ("project".to_string(), json!({"id": 7})),
            ("ignored".to_string(), json!(true)),
        ]);
        let payload = build_blueprint_generation_payload("title", "genre", 5, &variables)
            .expect("valid blueprint payload");
        assert_eq!(payload["project"], json!({"id": 7}));
        assert!(payload.get("ignored").is_none());
    }
}
