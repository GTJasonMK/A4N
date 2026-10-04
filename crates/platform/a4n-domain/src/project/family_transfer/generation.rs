use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyGeneratedVideoRecord {
    pub id: i64,
    pub project_id: i64,
    pub chapter_id: Option<i64>,

    /// 视频记录类型由 `a4n_domain::video_workflow` 的常量定义。
    pub video_type: String,

    /// 视频工作流模式：boundary_frames / first_frame_chain / reference_media / chat_video
    pub workflow_mode: Option<String>,

    /// 文件路径
    pub file_path: Option<String>,

    /// 缩略图路径
    pub thumbnail_path: Option<String>,

    /// 生成提示词
    pub prompt: String,

    pub negative_prompt: Option<String>,

    /// 使用的模型
    pub model: String,

    pub width: i32,
    pub height: i32,
    pub duration_secs: f32,
    pub fps: i32,

    /// 文件大小（字节）
    pub file_size: Option<i64>,

    pub source_video_id: Option<i64>,
    pub source_image_id: Option<i64>,

    /// pending / running / interrupted / completed / failed / cancelled
    pub status: String,
    pub provider_job_id: Option<String>,

    pub error_message: Option<String>,

    /// 插入片段视频对应的稳定时间线项。
    pub timeline_item_id: Option<i64>,

    pub metadata_json: Option<serde_json::Value>,

    /// 是否被用户收藏
    pub is_favorite: bool,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyGenerationSpecRecord {
    pub id: i64,
    pub project_id: i64,
    pub root_project_id: i64,
    pub source_context_kind: Option<String>,
    pub source_snapshot_id: Option<i64>,
    pub source_chapter_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub canvas_text_asset_id: Option<i64>,
    pub target_kind: String,
    pub target_entity_kind: String,
    pub target_entity_id: Option<i64>,
    pub status: String,
    pub source_version_hash: String,
    pub params_json: serde_json::Value,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

impl From<&ProjectFamilyGenerationSpecRecord> for crate::generation::GenerationSpecRecord {
    fn from(record: &ProjectFamilyGenerationSpecRecord) -> Self {
        Self {
            id: record.id,
            project_id: record.project_id,
            root_project_id: record.root_project_id,
            source_context_kind: record.source_context_kind.clone(),
            source_snapshot_id: record.source_snapshot_id,
            source_chapter_id: record.source_chapter_id,
            source_snapshot_chapter_id: record.source_snapshot_chapter_id,
            canvas_text_asset_id: record.canvas_text_asset_id,
            target_kind: record.target_kind.clone(),
            target_entity_kind: record.target_entity_kind.clone(),
            target_entity_id: record.target_entity_id,
            status: record.status.clone(),
            source_version_hash: record.source_version_hash.clone(),
            params_json: record.params_json.clone(),
            created_at: record.created_at,
            updated_at: record.updated_at,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyGenerationSpecTextAnchorRecord {
    pub generation_spec_id: i64,
    pub text_anchor_id: i64,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyGenerationSpecEntityRefRecord {
    pub generation_spec_id: i64,
    pub entity_kind: String,
    pub entity_id: i64,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyGenerationSpecMediaRefRecord {
    pub generation_spec_id: i64,
    pub media_asset_id: i64,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyPromptArtifactRecord {
    pub id: i64,
    pub generation_spec_id: i64,
    pub prompt_kind: String,
    pub prompt_text: String,
    pub renderer_version: String,
    pub edited_by_user: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyMediaAssetRecord {
    pub id: i64,
    pub project_id: i64,
    pub root_project_id: i64,
    pub source_snapshot_id: Option<i64>,
    pub source_chapter_id: Option<i64>,
    pub source_snapshot_chapter_id: Option<i64>,
    pub media_kind: String,
    pub origin: String,
    pub file_path: Option<String>,
    pub thumbnail_path: Option<String>,
    pub width: Option<i32>,
    pub height: Option<i32>,
    pub duration_secs: Option<f32>,
    pub fps: Option<f32>,
    pub file_size: Option<i64>,
    pub checksum: Option<String>,
    pub status: String,
    pub error_message: Option<String>,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyMediaProvenanceRecord {
    pub media_asset_id: i64,
    pub generation_spec_id: Option<i64>,
    pub prompt_artifact_id: Option<i64>,
    pub provider_job_id: Option<String>,
    pub source_media_asset_id: Option<i64>,
    pub source_external_path: Option<String>,
    pub imported_from_user: bool,
    pub request_snapshot_json: Option<serde_json::Value>,
    pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyMediaBindingRecord {
    pub id: i64,
    pub media_asset_id: i64,
    pub target_kind: String,
    pub target_id: i64,
    pub role: String,
    pub created_at: chrono::NaiveDateTime,
}
