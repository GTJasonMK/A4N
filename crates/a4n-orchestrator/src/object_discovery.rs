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

pub use contracts::{ObjectDiscoveryInput, ObjectDiscoveryProgress, ObjectDiscoveryRequestPreview};
pub use rules::{
    OBJECT_DISCOVERY_CALL_KEY, object_discovery_business_key,
    object_discovery_call_fingerprint_json, object_discovery_workflow_fingerprint,
    select_object_discovery_chapters, validate_object_discovery_chapter_scope,
};

#[cfg(test)]
mod tests;
