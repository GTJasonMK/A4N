//! Import review 公共类型与 application 输入门禁。
//!
//! 数据库行为由 `context/import_review.rs` 的私有 `ImportReviewApplication` 持有。

mod rules;

pub use a4n_api_contract::import_review::DedupResult as ImportDedupResult;
pub use a4n_db::import_review::{AcceptedImportReview, ImportReviewCreateInput};

pub(crate) use rules::calculate_import_dedup;
