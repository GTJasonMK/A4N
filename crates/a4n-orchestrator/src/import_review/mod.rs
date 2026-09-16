//! Import review application boundary and validation rules.
//!
//! 数据库行为由 `context/import_review.rs` 的私有 `ImportReviewApplication` 持有。

mod rules;

pub(crate) use rules::calculate_import_dedup;

pub(crate) use a4n_novel_host_contract::ImportReviewIngestionApplicationPort;
