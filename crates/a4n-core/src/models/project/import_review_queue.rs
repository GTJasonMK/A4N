//! 导入审核队列表实体【新增】

use sea_orm::entity::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
#[sea_orm(table_name = "import_review_queue")]
pub struct Model {
    #[sea_orm(primary_key)]
    pub id: i64,
    pub project_id: i64,

    /// 原始文件名
    pub filename: String,

    /// 文件类型：txt / epub / docx
    pub file_type: String,

    /// 检测到的编码
    pub detected_encoding: Option<String>,

    /// 解析出的章节数
    pub parsed_chapters: i32,

    /// 去重后的段落数
    pub dedup_paragraphs: i32,

    /// SimHash 去重率
    pub dedup_rate: Option<f32>,

    /// AI 分析结果
    #[sea_orm(column_type = "Json")]
    pub analysis_result: Option<serde_json::Value>,

    /// 审核状态：pending / analyzing / reviewing / accepted / rejected / modified
    pub review_status: String,

    pub created_at: chrono::NaiveDateTime,
    pub updated_at: chrono::NaiveDateTime,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

impl ActiveModelBehavior for ActiveModel {}
