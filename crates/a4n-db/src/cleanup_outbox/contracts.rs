use a4n_core::models::task::cleanup_outbox;
use sea_orm::DbErr;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub const PROJECT_DELETE_CLEANUP_KIND: &str = "project_delete";
pub const MEDIA_WORKFLOW_CONTRACT_V2_RESET_KIND: &str = "media_workflow_contract_v2_reset";
pub const CLEANUP_STATUS_PENDING: &str = "pending";
pub const CLEANUP_STATUS_RUNNING: &str = "running";
pub const CLEANUP_STATUS_COMPLETED: &str = "completed";
pub const CLEANUP_STATUS_FAILED: &str = "failed";
pub(super) const CLEANUP_LEASE_SECONDS: i64 = 300;
const CLEANUP_PAYLOAD_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct ProjectDeletionCleanupPayload {
    pub version: u32,
    pub project_id: i64,
    pub media_paths: Vec<String>,
}

impl ProjectDeletionCleanupPayload {
    pub fn new(project_id: i64, media_paths: Vec<String>) -> Self {
        let media_paths = media_paths
            .into_iter()
            .filter(|path| !path.trim().is_empty())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        Self {
            version: CLEANUP_PAYLOAD_VERSION,
            project_id,
            media_paths,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClaimedProjectCleanup {
    pub record: cleanup_outbox::Model,
    pub payload: ProjectDeletionCleanupPayload,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct MediaWorkflowV2ResetPayload {
    pub version: u32,
    pub project_id: i64,
}

impl MediaWorkflowV2ResetPayload {
    pub fn new(project_id: i64) -> Self {
        Self {
            version: CLEANUP_PAYLOAD_VERSION,
            project_id,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClaimedMediaWorkflowV2Reset {
    pub record: cleanup_outbox::Model,
    pub payload: MediaWorkflowV2ResetPayload,
}

pub(super) fn parse_project_deletion_payload(
    record: &cleanup_outbox::Model,
) -> Result<ProjectDeletionCleanupPayload, DbErr> {
    let payload: ProjectDeletionCleanupPayload =
        serde_json::from_value(record.payload_json.clone()).map_err(|error| {
            DbErr::Custom(format!("解析项目删除 cleanup payload 失败: {error}"))
        })?;
    if payload.version != CLEANUP_PAYLOAD_VERSION || payload.project_id != record.project_id {
        return Err(DbErr::Custom(format!(
            "项目删除 cleanup payload 合同不一致: id={}",
            record.id
        )));
    }
    Ok(payload)
}

pub(super) fn parse_media_workflow_v2_reset_payload(
    record: &cleanup_outbox::Model,
) -> Result<MediaWorkflowV2ResetPayload, DbErr> {
    let payload: MediaWorkflowV2ResetPayload = serde_json::from_value(record.payload_json.clone())
        .map_err(|error| DbErr::Custom(format!("解析媒体工作流 v2 reset payload 失败: {error}")))?;
    if payload.version != CLEANUP_PAYLOAD_VERSION || payload.project_id != record.project_id {
        return Err(DbErr::Custom(format!(
            "媒体工作流 v2 reset payload 合同不一致: id={}",
            record.id
        )));
    }
    Ok(payload)
}
