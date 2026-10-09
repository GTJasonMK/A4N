use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyCharacterPortraitRecord {
    pub id: i64,
    pub profile_id: i64,
    pub project_id: i64,

    /// 立绘类型：full_body / half_body / face / chibi
    pub portrait_type: String,

    /// 图片路径
    pub image_path: String,

    /// 生成提示词
    pub prompt: String,

    /// 表情/姿态
    pub expression: Option<String>,
    pub pose: Option<String>,

    /// 是否为主立绘
    pub is_primary: bool,

    pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyGeneratedImageRecord {
    pub id: i64,
    pub project_id: i64,

    /// 图片类型：portrait / manga_page / scene / cover / illustration
    pub image_type: String,

    /// 文件路径
    pub file_path: String,

    /// 生成提示词
    pub prompt: String,

    /// 使用的模型
    pub model: String,

    /// 图片宽度
    pub width: i32,
    /// 图片高度
    pub height: i32,

    /// 文件大小（字节）
    pub file_size: Option<i64>,

    /// AI 评分
    pub ai_score: Option<f32>,

    /// 是否被用户收藏
    pub is_favorite: bool,

    /// 关联章节
    pub chapter_id: Option<i64>,

    /// 关联漫画分镜/页面标识
    pub panel_id: Option<String>,

    /// 生成元数据
    pub metadata_json: Option<serde_json::Value>,

    pub created_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyMangaPromptRecord {
    pub id: i64,
    pub chapter_id: i64,
    pub project_id: i64,

    /// 页序号
    pub page_number: i32,

    /// 固定为 page_generation
    pub prompt_type: String,

    /// 图片生成提示词
    pub image_prompt: String,

    /// 对话/旁白文本
    pub dialogue_text: Option<String>,

    /// 角色出镜列表
    pub characters_in_frame: Option<serde_json::Value>,

    /// 页面计划及来源、导演计划、参考素材绑定
    pub layout: Option<serde_json::Value>,

    /// 对应生成图片 ID
    pub generated_image_id: Option<i64>,

    /// 流水线状态：pending / generating / completed / failed / cancelled
    pub status: String,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProjectFamilyMangaPipelineArtifactRecord {
    pub id: i64,
    pub project_id: i64,
    pub chapter_id: i64,
    pub stage: String,
    pub page_number: i32,
    pub input_fingerprint: String,
    pub output_fingerprint: String,
    pub payload: serde_json::Value,
    pub stale: bool,
    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}
