//! SeaORM entities owned by the database boundary.
//!
//! This crate contains persistence schema types only. Application contracts
//! and pure domain values belong in `a4n-domain` or the relevant contract crate.

pub mod models;
mod source_adapters;

/// 当前应用代码要求数据库至少已经执行到的最后一项迁移。
///
/// Migration 注册顺序测试会阻止该合同与实际迁移列表漂移。
pub const REQUIRED_SCHEMA_MIGRATION: &str = "model_usage_root_started_index";
