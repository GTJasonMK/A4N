use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::wire::{WorkerContractError, validate_non_blank, validate_worker_schema_version};

pub const WORKER_HEALTH_PATH: &str = "/healthz";
pub const WORKER_CAPABILITIES_PATH: &str = "/capabilities";

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkerHealth {
    pub schema_version: u32,
    pub worker_id: String,
    pub status: WorkerHealthStatus,
    pub version: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum WorkerHealthStatus {
    Healthy,
    Degraded,
    Unavailable,
}

impl WorkerHealth {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        validate_worker_schema_version(self.schema_version)?;
        validate_non_blank("worker_id", &self.worker_id)?;
        validate_non_blank("version", &self.version)
    }

    pub fn validate_for(&self, expected_worker_id: &str) -> Result<(), WorkerContractError> {
        self.validate()?;
        validate_non_blank("expected_worker_id", expected_worker_id)?;
        if self.worker_id != expected_worker_id {
            return Err(WorkerContractError::InvalidRelation(
                "worker health worker_id does not match expected worker",
            ));
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkerCapability {
    pub name: String,
    pub version: String,
    pub enabled: bool,
}

impl WorkerCapability {
    pub fn validate(&self) -> Result<(), WorkerContractError> {
        validate_non_blank("capability.name", &self.name)?;
        validate_non_blank("capability.version", &self.version)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct WorkerCapabilities {
    pub schema_version: u32,
    pub worker_id: String,
    #[serde(default)]
    pub capabilities: Vec<WorkerCapability>,
}

impl WorkerCapabilities {
    pub fn has_enabled_capability(&self, name: &str, version: &str) -> bool {
        self.capabilities.iter().any(|capability| {
            capability.name == name && capability.version == version && capability.enabled
        })
    }

    pub fn validate(&self) -> Result<(), WorkerContractError> {
        validate_worker_schema_version(self.schema_version)?;
        validate_non_blank("worker_id", &self.worker_id)?;
        let mut names = HashSet::with_capacity(self.capabilities.len());
        for capability in &self.capabilities {
            capability.validate()?;
            if !names.insert(&capability.name) {
                return Err(WorkerContractError::InvalidRelation(
                    "capabilities must not contain duplicate names",
                ));
            }
        }
        Ok(())
    }

    pub fn validate_for(&self, expected_worker_id: &str) -> Result<(), WorkerContractError> {
        self.validate()?;
        validate_non_blank("expected_worker_id", expected_worker_id)?;
        if self.worker_id != expected_worker_id {
            return Err(WorkerContractError::InvalidRelation(
                "worker capabilities worker_id does not match expected worker",
            ));
        }
        Ok(())
    }
}
