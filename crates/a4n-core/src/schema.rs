//! Runtime 与 migration 共享的数据库 schema 兼容性边界。

/// 当前应用代码要求数据库至少已经执行到的最后一项迁移。
///
/// 新增迁移时必须同步更新；`a4n-migration` 的注册顺序测试会阻止二者漂移。
pub const REQUIRED_SCHEMA_MIGRATION: &str = "project_derivation_identity_cleanup";
