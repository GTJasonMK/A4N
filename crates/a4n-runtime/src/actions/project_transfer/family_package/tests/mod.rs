//! Host 侧 remap 烟雾测；pure validation 单测已迁 orch `project::family_package`。

use super::*;
use a4n_orchestrator::project::*;
use chrono::NaiveDate;
use serde_json::json;

mod fixtures {
    mod media_generation;
    mod source_snapshot;
    mod source_sync;

    pub(super) use media_generation::*;
    pub(super) use source_snapshot::*;
    pub(super) use source_sync::*;
}

use fixtures::*;

fn dt() -> chrono::NaiveDateTime {
    NaiveDate::from_ymd_opt(2026, 1, 2)
        .unwrap()
        .and_hms_opt(3, 4, 5)
        .unwrap()
}

mod remap;
