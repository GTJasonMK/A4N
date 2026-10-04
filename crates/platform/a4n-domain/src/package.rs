use chrono::NaiveDateTime;

#[derive(Debug, Clone, PartialEq)]
pub struct HostPackageOperationRecord {
    pub operation_id: String,
    pub host_scope: String,
    pub package_id: String,
    pub action: String,
    pub state: String,
    pub requested_version: Option<String>,
    pub requested_by_user_id: i64,
    pub status_version: i64,
    pub reason_code: Option<String>,
    pub error: Option<String>,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}
