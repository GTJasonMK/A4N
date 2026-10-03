use std::fmt;

use chrono::NaiveDateTime;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq)]
pub struct SourceSyncRunRecord {
    pub id: i64,
    pub derived_project_id: i64,
    pub root_project_id: i64,
    pub from_snapshot_id: i64,
    pub to_snapshot_id: i64,
    pub mode: String,
    pub status: String,
    pub summary_json: Value,
    pub error_message: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SourceSyncConflictRecord {
    pub id: i64,
    pub sync_run_id: i64,
    pub derived_project_id: i64,
    pub root_project_id: i64,
    pub from_snapshot_id: i64,
    pub to_snapshot_id: i64,
    pub conflict_scope: String,
    pub target_kind: String,
    pub target_id: i64,
    pub source_change_kind: String,
    pub local_state_kind: String,
    pub severity: String,
    pub status: String,
    pub resolution_strategy: Option<String>,
    pub source_summary_json: Value,
    pub local_summary_json: Value,
    pub affected_refs_json: Value,
    pub decision_json: Option<Value>,
    pub created_at: NaiveDateTime,
    pub resolved_at: Option<NaiveDateTime>,
}

pub const GALGAME_REGENERATABLE_ENTRY_KIND: crate::galgame::GalgameSceneEntryKind =
    crate::galgame::GalgameSceneEntryKind::Main;

macro_rules! source_sync_enum {
    ($name:ident { $($variant:ident => $value:literal),+ $(,)? }) => {
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, JsonSchema)]
        pub enum $name {
            $(#[serde(rename = $value)] $variant),+
        }

        impl $name {
            pub const ALL: &'static [Self] = &[$(Self::$variant),+];
            pub const VALUES: &'static [&'static str] = &[$($value),+];

            pub const fn as_str(self) -> &'static str {
                match self {
                    $(Self::$variant => $value),+
                }
            }

            pub fn parse(value: &str) -> Option<Self> {
                Self::ALL
                    .iter()
                    .copied()
                    .find(|candidate| candidate.as_str() == value)
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }
    };
}

source_sync_enum!(SourceSyncRunMode {
    DryRun => "dry_run",
    Apply => "apply",
});

source_sync_enum!(SourceSyncRunStatus {
    Planned => "planned",
    Applying => "applying",
    Completed => "completed",
});

source_sync_enum!(SourceSyncChangeKind {
    Added => "added",
    Updated => "updated",
    Reordered => "reordered",
    Deleted => "deleted",
    Moved => "moved",
    Split => "split",
});

source_sync_enum!(SourceSyncLocalStateKind {
    UserEdited => "user_edited",
    Generated => "generated",
    Imported => "imported",
    Assembled => "assembled",
    Clean => "clean",
    Draft => "draft",
});

source_sync_enum!(SourceSyncConflictSeverity {
    Blocking => "blocking",
    Warning => "warning",
    Info => "info",
});

source_sync_enum!(SourceSyncConflictStatus {
    Pending => "pending",
    Resolved => "resolved",
    Ignored => "ignored",
    Obsolete => "obsolete",
});

source_sync_enum!(SourceSyncResolutionStrategy {
    KeepLocal => "keep_local",
    AdoptSource => "adopt_source",
    RebuildPrompt => "rebuild_prompt",
    RebindAnchor => "rebind_anchor",
    RegenerateScene => "regenerate_scene",
    CreateReplacement => "create_replacement",
    Manual => "manual",
    Detach => "detach",
    DeleteLocal => "delete_local",
});

source_sync_enum!(SourceSyncDecisionActor {
    User => "user",
    System => "system",
});

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CreateSourceSyncDryRunRecordInput {
    pub derived_project_id: i64,
    pub root_project_id: i64,
    pub from_snapshot_id: i64,
    pub to_snapshot_id: i64,
    pub chapter_change_count: usize,
    pub profile_change_count: usize,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_sync_values_accept_only_canonical_spelling() {
        assert_eq!(
            SourceSyncResolutionStrategy::parse("regenerate_scene"),
            Some(SourceSyncResolutionStrategy::RegenerateScene)
        );
        assert!(SourceSyncResolutionStrategy::parse(" regenerate_scene ").is_none());
        assert!(!SourceSyncLocalStateKind::VALUES.contains(&"exported"));
    }
}
