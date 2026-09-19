//! Object-profile normalization adapters for Orchestrator-owned discovery.

use a4n_domain::object_profile::{
    ObjectProfileInput, ObjectProfileInputError, ObjectProfileRecordInput,
};
use a4n_error::AppError;

pub(crate) use a4n_domain::object_profile::OBJECT_TYPES;

pub(crate) fn normalize_object_name(name: &str) -> Result<String, AppError> {
    a4n_domain::object_profile::normalize_object_name(name).map_err(object_profile_input_error)
}

pub(crate) fn normalize_object_type_value(value: &str) -> Result<String, AppError> {
    a4n_domain::object_profile::normalize_object_type_value(value)
        .map_err(object_profile_input_error)
}

pub(crate) fn normalize_object_profile_record_inputs(
    drafts: Vec<ObjectProfileInput>,
) -> Result<Vec<ObjectProfileRecordInput>, AppError> {
    a4n_domain::object_profile::normalize_object_profile_record_inputs(drafts)
        .map_err(object_profile_input_error)
}

fn object_profile_input_error(error: ObjectProfileInputError) -> AppError {
    AppError::InvalidArgument(error.to_string())
}
