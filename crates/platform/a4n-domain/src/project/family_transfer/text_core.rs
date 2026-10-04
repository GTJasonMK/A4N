use super::ProjectFamilyFactFeedbackArchive;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyNovelBlueprintRecord {
    pub id: i64,

    /// 一对一关联 novel_project
    pub project_id: i64,

    pub title: String,
    pub genre: String,
    pub style: Option<String>,
    pub tone: Option<String>,
    pub target_audience: Option<String>,

    pub one_sentence_summary: Option<String>,
    pub full_synopsis: Option<String>,

    /// 结构化世界观
    pub world_setting: Option<serde_json::Value>,

    /// 当前版本号（semver）
    pub current_version: String,

    pub needs_part_outlines: bool,
    pub total_chapters: Option<i32>,
    pub chapters_per_part: Option<i32>,

    pub avatar_svg: Option<String>,
    pub avatar_animal: Option<String>,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyBlueprintVersionRecord {
    pub id: i64,
    pub project_id: i64,

    /// 版本号：1.0 / 1.1 / 2.0
    pub version: String,
    pub change_description: Option<String>,

    /// 完整蓝图快照
    pub blueprint_data: serde_json::Value,

    pub created_at: chrono::NaiveDateTime,
    pub created_by: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyBlueprintCharacterRecord {
    pub id: i64,
    pub project_id: i64,

    pub name: String,

    /// 角色类型：protagonist / antagonist / supporting / minor
    pub character_type: String,

    pub gender: Option<String>,
    pub age: Option<String>,
    pub appearance: Option<String>,
    pub personality: Option<String>,
    pub background: Option<String>,
    pub motivation: Option<String>,
    pub arc_summary: Option<String>,

    /// 与主角的关系描述
    pub relationship_to_protagonist: Option<String>,

    /// 角色弧光阶段
    pub arc_stages: Option<serde_json::Value>,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyBlueprintRelationshipRecord {
    pub id: i64,
    pub project_id: i64,

    /// 关系类型：family / friend / enemy / lover / rival / mentor / other
    pub relationship_type: String,

    /// 角色 A 名称
    pub character_a: String,
    /// 角色 B 名称
    pub character_b: String,

    pub description: Option<String>,

    /// 关系强度（1~10）
    pub strength: Option<i32>,

    /// 关系变化弧线
    pub evolution: Option<serde_json::Value>,

    pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyPartOutlineRecord {
    pub id: i64,
    pub project_id: i64,

    pub part_number: i32,
    pub title: String,
    pub start_chapter: i32,
    pub end_chapter: i32,

    pub summary: Option<String>,
    pub theme: Option<String>,

    pub key_events: Option<serde_json::Value>,
    pub character_focus: Option<serde_json::Value>,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyChapterOutlineRecord {
    pub id: i64,
    pub project_id: i64,
    pub part_id: Option<i64>,

    pub chapter_number: i32,
    pub title: Option<String>,

    pub summary: Option<String>,

    /// 节拍表条目
    pub beats: Option<serde_json::Value>,

    /// 本章关键场景
    pub key_scenes: Option<serde_json::Value>,

    /// 节奏评分
    pub pacing_score: Option<f32>,

    /// 本章伏笔
    pub foreshadowing_setup: Option<serde_json::Value>,
    pub foreshadowing_payoff: Option<serde_json::Value>,

    /// 本章涉及的角色
    pub characters_involved: Option<serde_json::Value>,

    pub word_count_target: Option<i32>,
    pub status: String,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyChapterRecord {
    pub id: i64,
    pub project_id: i64,
    pub outline_id: Option<i64>,

    pub chapter_number: i32,
    pub title: Option<String>,

    /// 章节正文
    pub content: String,

    pub word_count: i32,

    /// 状态：draft / generating / imported / completed / reviewed / published / deleted
    pub status: String,

    /// 章节摘要
    pub summary: Option<String>,

    /// 风格标签
    pub style_tags: Option<serde_json::Value>,

    /// 章节结构化分析数据
    pub analysis_data: Option<serde_json::Value>,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyProtagonistProfileRecord {
    pub id: i64,
    pub project_id: i64,

    pub name: String,
    pub gender: Option<String>,
    pub age: Option<String>,

    /// 基础属性（JSON）
    pub base_attributes: serde_json::Value,

    /// 当前属性状态（事件溯源折叠结果）
    pub current_state: serde_json::Value,

    /// 角色弧光定义
    pub character_arc: Option<serde_json::Value>,

    /// 角色声音指纹
    pub voice_fingerprint: Option<serde_json::Value>,

    /// 外观描述
    pub appearance: Option<String>,

    /// 背景故事
    pub background: Option<String>,

    /// 人格描述
    pub personality: Option<String>,

    /// 档案已经同步到的章节号
    pub last_synced_chapter: i32,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyObjectProfileRecord {
    pub id: i64,
    pub project_id: i64,

    pub ownership_kind: String,

    pub name: String,
    pub normalized_name: String,

    pub aliases_json: serde_json::Value,

    pub object_type: String,
    pub importance: String,

    pub visual_description: String,

    pub material_and_state: String,

    pub story_usage: String,

    pub visual_usage: String,

    pub generation_usage: String,

    pub location_context: Option<String>,

    pub first_seen_chapter: i32,
    pub last_seen_chapter: i32,

    pub evidence_json: serde_json::Value,

    pub image_prompt_seed: String,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilySceneProfileRecord {
    pub id: i64,
    pub project_id: i64,

    pub ownership_kind: String,

    pub name: String,
    pub normalized_name: String,

    pub aliases_json: serde_json::Value,

    pub scene_type: String,
    pub importance: String,
    pub parent_scene_id: Option<i64>,

    pub base_description: String,

    pub visual_description: String,

    pub spatial_layout: String,

    pub atmosphere: String,

    pub entry_points_json: serde_json::Value,

    pub rules_json: serde_json::Value,

    pub current_state_json: serde_json::Value,

    pub first_seen_chapter: i32,
    pub last_seen_chapter: i32,

    pub evidence_json: serde_json::Value,

    pub image_prompt_seed: String,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilySceneOccurrenceRecord {
    pub id: i64,
    pub project_id: i64,
    pub scene_profile_id: i64,
    pub chapter_id: i64,
    pub chapter_number: i32,
    pub occurrence_order: i32,

    pub timeline_position: Option<String>,

    pub summary: String,

    pub mood: Option<String>,
    pub weather_or_lighting: Option<String>,

    pub source_span_ids_json: serde_json::Value,

    pub evidence_json: serde_json::Value,

    pub characters_json: serde_json::Value,

    pub objects_json: serde_json::Value,

    pub key_events_json: serde_json::Value,

    pub entry_state_json: serde_json::Value,

    pub exit_state_json: serde_json::Value,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilySceneTransitionRecord {
    pub id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub chapter_number: i32,
    pub from_scene_profile_id: Option<i64>,
    pub to_scene_profile_id: i64,
    pub occurrence_id: Option<i64>,
    pub transition_order: i32,
    pub movement_type: String,

    pub route_description: String,

    pub time_cost: Option<String>,
    pub distance_hint: Option<String>,

    pub participants_json: serde_json::Value,

    pub object_refs_json: serde_json::Value,

    pub source_span_ids_json: serde_json::Value,

    pub evidence_json: serde_json::Value,

    pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilySceneEntityStateRecord {
    pub id: i64,
    pub project_id: i64,
    pub scene_profile_id: i64,
    pub occurrence_id: i64,
    pub chapter_number: i32,

    pub entity_type: String,
    pub entity_id: Option<i64>,
    pub entity_name_snapshot: String,
    pub state_key: String,

    pub state_before_json: serde_json::Value,

    pub state_after_json: serde_json::Value,

    pub position_description: Option<String>,

    pub action_description: String,

    pub source_span_ids_json: serde_json::Value,

    pub evidence_json: serde_json::Value,

    pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ProjectFamilyTextCoreRows {
    #[serde(default)]
    pub fact_feedback: Option<ProjectFamilyFactFeedbackArchive>,
    pub blueprint: Option<ProjectFamilyNovelBlueprintRecord>,
    pub blueprint_versions: Vec<ProjectFamilyBlueprintVersionRecord>,
    pub blueprint_characters: Vec<ProjectFamilyBlueprintCharacterRecord>,
    pub blueprint_relationships: Vec<ProjectFamilyBlueprintRelationshipRecord>,
    pub part_outlines: Vec<ProjectFamilyPartOutlineRecord>,
    pub chapter_outlines: Vec<ProjectFamilyChapterOutlineRecord>,
    pub chapters: Vec<ProjectFamilyChapterRecord>,
    pub character_profiles: Vec<ProjectFamilyProtagonistProfileRecord>,
    pub object_profiles: Vec<ProjectFamilyObjectProfileRecord>,
    pub scene_profiles: Vec<ProjectFamilySceneProfileRecord>,
    pub scene_occurrences: Vec<ProjectFamilySceneOccurrenceRecord>,
    pub scene_transitions: Vec<ProjectFamilySceneTransitionRecord>,
    pub scene_entity_states: Vec<ProjectFamilySceneEntityStateRecord>,
}
