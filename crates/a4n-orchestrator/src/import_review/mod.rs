//! Import review 公共类型与 application 输入门禁。
//!
//! 数据库行为由 `context/import_review.rs` 的私有 `ImportReviewApplication` 持有。

mod rules;

pub use a4n_db::import_review::{
    AcceptedImportReview, ImportReviewCreateInput, ModifyImportReviewInput,
    normalize_import_review_status,
};

/// 导入审核去重结果；阈值和匹配规则由 Orchestrator 统一持有，Runtime 只做 DTO 投影。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ImportDedupResult {
    pub content_hash: String,
    pub is_duplicate: bool,
    pub closest_distance: u32,
    pub matched_chapter_id: Option<i64>,
}

pub(crate) use rules::{
    calculate_import_dedup, ensure_import_review_ready_for_accept,
    normalize_modify_import_review_input,
};
