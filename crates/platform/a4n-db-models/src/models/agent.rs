//! Persistent records for the shared main/sub-agent runtime.

pub mod thread {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "agent_thread")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub owner_user_id: i64,
        pub thread_kind: String,
        pub profile_id: String,
        pub title: Option<String>,
        pub project_id: Option<i64>,
        pub resource_type: Option<String>,
        pub resource_id: Option<String>,
        pub canonical_binding_key: Option<String>,
        #[sea_orm(column_type = "JsonBinary")]
        pub scope_json: serde_json::Value,
        pub default_model_config_id: Option<i64>,
        pub status: String,
        pub is_default: bool,
        pub revision: i64,
        pub next_turn_sequence: i64,
        pub next_event_sequence: i64,
        #[sea_orm(column_type = "Text")]
        pub context_summary: Option<String>,
        pub created_at: DateTime,
        pub updated_at: DateTime,
        pub archived_at: Option<DateTime>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod message {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "agent_message")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub thread_id: i64,
        pub turn_id: Option<i64>,
        pub run_id: Option<i64>,
        pub role: String,
        pub sender_kind: String,
        pub sender_thread_id: Option<i64>,
        pub client_message_id: Option<String>,
        pub content_sha256: String,
        #[sea_orm(column_type = "Text")]
        pub content: String,
        pub status: String,
        #[sea_orm(column_type = "JsonBinary")]
        pub payload_json: Option<serde_json::Value>,
        pub created_at: DateTime,
        pub updated_at: DateTime,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod turn {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "agent_turn")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub thread_id: i64,
        pub sequence: i64,
        pub client_message_id: String,
        pub input_message_id: i64,
        pub final_message_id: Option<i64>,
        pub trigger: String,
        #[sea_orm(column_type = "Text")]
        pub goal: Option<String>,
        pub status: String,
        pub execution_mode: String,
        #[sea_orm(column_type = "JsonBinary")]
        pub model_override_json: Option<serde_json::Value>,
        #[sea_orm(column_type = "Text")]
        pub error_message: Option<String>,
        #[sea_orm(column_type = "JsonBinary")]
        pub result_refs_json: Option<serde_json::Value>,
        pub created_at: DateTime,
        pub updated_at: DateTime,
        pub completed_at: Option<DateTime>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod run {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "agent_run")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub thread_id: i64,
        pub turn_id: i64,
        pub root_run_id: Option<i64>,
        pub parent_run_id: Option<i64>,
        pub attempt: i32,
        pub status: String,
        pub waiting_reason: Option<String>,
        pub is_active: bool,
        pub next_execution_sequence: i64,
        #[sea_orm(column_type = "JsonBinary")]
        pub budget_json: serde_json::Value,
        #[sea_orm(column_type = "JsonBinary")]
        pub usage_json: serde_json::Value,
        pub worker_id: Option<String>,
        pub lease_token: Option<String>,
        pub lease_expires_at: Option<DateTime>,
        pub next_attempt_at: DateTime,
        #[sea_orm(column_type = "Text")]
        pub last_error: Option<String>,
        pub created_at: DateTime,
        pub updated_at: DateTime,
        pub started_at: Option<DateTime>,
        pub completed_at: Option<DateTime>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod execution {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "agent_execution")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub run_id: i64,
        pub sequence: i64,
        pub execution_key: String,
        pub execution_kind: String,
        pub status: String,
        pub tool_name: Option<String>,
        pub effect: Option<String>,
        pub replay_policy: Option<String>,
        #[sea_orm(column_type = "JsonBinary")]
        pub arguments_json: Option<serde_json::Value>,
        pub arguments_sha256: Option<String>,
        #[sea_orm(column_type = "JsonBinary")]
        pub resource_preconditions_json: Option<serde_json::Value>,
        #[sea_orm(column_type = "JsonBinary")]
        pub result_json: Option<serde_json::Value>,
        #[sea_orm(column_type = "Text")]
        pub result_summary: Option<String>,
        #[sea_orm(column_type = "JsonBinary")]
        pub result_refs_json: Option<serde_json::Value>,
        #[sea_orm(column_type = "Text")]
        pub error_message: Option<String>,
        pub created_at: DateTime,
        pub updated_at: DateTime,
        pub started_at: Option<DateTime>,
        pub completed_at: Option<DateTime>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod event {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "agent_event")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub thread_id: i64,
        pub sequence: i64,
        pub turn_id: Option<i64>,
        pub run_id: Option<i64>,
        pub execution_id: Option<i64>,
        pub delegation_id: Option<i64>,
        pub message_id: Option<i64>,
        pub actor_kind: String,
        pub actor_id: Option<String>,
        pub kind: String,
        pub status: String,
        #[sea_orm(column_type = "Text")]
        pub public_text: String,
        #[sea_orm(column_type = "JsonBinary")]
        pub payload_json: Option<serde_json::Value>,
        pub created_at: DateTime,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod inbox {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "agent_inbox")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub thread_id: i64,
        pub turn_id: Option<i64>,
        pub run_id: Option<i64>,
        pub source_kind: String,
        pub source_thread_id: Option<i64>,
        pub inbox_kind: String,
        pub delivery_mode: String,
        pub priority: i32,
        pub dedupe_key: String,
        pub content_sha256: String,
        #[sea_orm(column_type = "Text")]
        pub content: String,
        #[sea_orm(column_type = "JsonBinary")]
        pub payload_json: Option<serde_json::Value>,
        pub status: String,
        pub available_at: DateTime,
        pub claim_token: Option<String>,
        pub claim_expires_at: Option<DateTime>,
        pub created_at: DateTime,
        pub claimed_at: Option<DateTime>,
        pub consumed_at: Option<DateTime>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod delegation {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "agent_delegation")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub parent_thread_id: i64,
        pub parent_run_id: i64,
        pub parent_execution_id: Option<i64>,
        pub child_thread_id: i64,
        pub child_turn_id: Option<i64>,
        pub child_run_id: Option<i64>,
        pub delegation_key: String,
        pub capability: String,
        #[sea_orm(column_type = "Text")]
        pub objective: String,
        #[sea_orm(column_type = "JsonBinary")]
        pub success_criteria_json: serde_json::Value,
        #[sea_orm(column_type = "JsonBinary")]
        pub scope_json: serde_json::Value,
        #[sea_orm(column_type = "JsonBinary")]
        pub budget_json: serde_json::Value,
        pub status: String,
        #[sea_orm(column_type = "JsonBinary")]
        pub result_refs_json: Option<serde_json::Value>,
        #[sea_orm(column_type = "Text")]
        pub result_summary: Option<String>,
        #[sea_orm(column_type = "Text")]
        pub error_message: Option<String>,
        pub created_at: DateTime,
        pub updated_at: DateTime,
        pub completed_at: Option<DateTime>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod approval {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "agent_approval")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub thread_id: i64,
        pub run_id: i64,
        pub execution_id: i64,
        pub operation_digest: String,
        pub status: String,
        #[sea_orm(column_type = "JsonBinary")]
        pub operation_json: serde_json::Value,
        #[sea_orm(column_type = "JsonBinary")]
        pub resource_preconditions_json: Option<serde_json::Value>,
        pub decided_by_user_id: Option<i64>,
        #[sea_orm(column_type = "Text")]
        pub decision_note: Option<String>,
        pub expires_at: Option<DateTime>,
        pub created_at: DateTime,
        pub decided_at: Option<DateTime>,
        pub consumed_at: Option<DateTime>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

pub mod memory {
    use sea_orm::entity::prelude::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, DeriveEntityModel, Serialize, Deserialize)]
    #[sea_orm(table_name = "agent_memory")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i64,
        pub owner_user_id: i64,
        pub project_id: Option<i64>,
        pub thread_id: Option<i64>,
        pub scope_kind: String,
        pub scope_key: String,
        pub memory_key: String,
        pub revision: i64,
        pub status: String,
        #[sea_orm(column_type = "Text")]
        pub content: String,
        #[sea_orm(column_type = "JsonBinary")]
        pub content_json: Option<serde_json::Value>,
        pub source_message_id: Option<i64>,
        pub source_execution_id: Option<i64>,
        pub supersedes_memory_id: Option<i64>,
        pub created_at: DateTime,
        pub updated_at: DateTime,
        pub retracted_at: Option<DateTime>,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}
