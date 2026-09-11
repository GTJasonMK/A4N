//! Pure inputs for inspiration-domain records.

#[derive(Debug, Clone, PartialEq)]
pub struct InspirationExchangeInsertInput {
    pub project_id: i64,
    pub user_input: String,
    pub assistant_output: String,
}
