use a4n_core::models::generation::{generation_run, llm_call_trace};
use chrono::{Duration, NaiveDate, NaiveDateTime, Utc};
use sea_orm::{ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder};
use std::collections::BTreeMap;

use super::AdminDbError;

mod aggregate;
mod metrics;
mod projections;
mod query;
mod types;

pub use query::generation_call_analytics;
use types::*;

#[cfg(test)]
mod tests;
