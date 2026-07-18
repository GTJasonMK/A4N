//! Pure domain value objects.
//!
//! This crate must stay free of database, provider, sidecar transport, and UI
//! dependencies.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ProjectId(i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UserId(i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ChapterId(i64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct WorkflowRunId(i64);

#[derive(Debug, thiserror::Error)]
pub enum DomainError {
    #[error("{field} 必须是正数")]
    NonPositiveId { field: &'static str },
}

impl ProjectId {
    pub fn new(value: i64) -> Result<Self, DomainError> {
        positive_id(value, "project_id").map(Self)
    }

    pub fn get(self) -> i64 {
        self.0
    }
}

impl UserId {
    pub fn new(value: i64) -> Result<Self, DomainError> {
        positive_id(value, "user_id").map(Self)
    }

    pub fn get(self) -> i64 {
        self.0
    }
}

impl ChapterId {
    pub fn new(value: i64) -> Result<Self, DomainError> {
        positive_id(value, "chapter_id").map(Self)
    }

    pub fn get(self) -> i64 {
        self.0
    }
}

impl WorkflowRunId {
    pub fn new(value: i64) -> Result<Self, DomainError> {
        positive_id(value, "workflow_run_id").map(Self)
    }

    pub fn get(self) -> i64 {
        self.0
    }
}

fn positive_id(value: i64, field: &'static str) -> Result<i64, DomainError> {
    if value <= 0 {
        return Err(DomainError::NonPositiveId { field });
    }
    Ok(value)
}
