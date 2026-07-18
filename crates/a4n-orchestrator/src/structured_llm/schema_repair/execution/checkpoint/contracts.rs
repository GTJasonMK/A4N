pub(in crate::structured_llm::schema_repair::execution) const STRUCTURED_REPAIR_CHECKPOINT_SCHEMA_VERSION: u64 = 2;

pub(in crate::structured_llm::schema_repair::execution) struct CompatibleRepairCheckpointInput<
    'a,
    P,
> {
    pub(in crate::structured_llm::schema_repair::execution) workflow_run_id: i64,
    pub(in crate::structured_llm::schema_repair::execution) base_call_key: &'a str,
    pub(in crate::structured_llm::schema_repair::execution) call_key: &'a str,
    pub(in crate::structured_llm::schema_repair::execution) suffix: &'a str,
    pub(in crate::structured_llm::schema_repair::execution) base_response_sha256: &'a str,
    pub(in crate::structured_llm::schema_repair::execution) target_schema_sha256: &'a str,
    pub(in crate::structured_llm::schema_repair::execution) require_target_validation: bool,
    pub(in crate::structured_llm::schema_repair::execution) parse_output: &'a P,
}
