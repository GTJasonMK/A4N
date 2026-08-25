//! 物体发现 application。
//!
//! 统一拥有上下文、请求、structured streaming、checkpoint、严格输出校验、档案落库和证据绑定。

mod context;
mod contracts;
mod execution;
mod output;
mod persistence;
mod request;
mod rules;

pub(crate) const OBJECT_DISCOVERY_KIND: &str = "object_discovery";
pub(crate) const OBJECT_DISCOVERY_CALL_KEY: &str = "object_discovery.llm";
pub(crate) const OBJECT_DISCOVERY_PROGRESS_TOTAL: u32 = 100;

pub(crate) use contracts::{ObjectDiscoveryInput, ObjectDiscoveryProgress};

#[cfg(test)]
mod tests;
