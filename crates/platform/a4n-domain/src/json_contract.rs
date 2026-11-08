mod arrays;
mod fields;
mod nullable;
mod object;
mod schema;
mod strict;
mod validation;
pub use nullable::RequiredNullable;
pub use schema::{
    array_output_contract_schema, json_schema_from_output_shape, nonempty_json_schema,
    typed_output_schema,
};
pub use strict::parse_strict_json_value;
pub use validation::OutputSchemaValidator;

pub fn is_empty_json_value(value: &serde_json::Value) -> bool {
    match value {
        serde_json::Value::Null => true,
        serde_json::Value::String(value) => value.trim().is_empty(),
        serde_json::Value::Array(value) => value.is_empty(),
        serde_json::Value::Object(value) => value.is_empty(),
        serde_json::Value::Bool(_) | serde_json::Value::Number(_) => false,
    }
}

pub use arrays::{positive_i64_array, positive_i64_array_schema, string_array, string_array_value};
pub use fields::{
    optional_positive_i64, optional_string, positive_i64_schema, required_array,
    required_array_value, required_object_value, required_string,
};
pub use object::{ensure_object, parse_llm_json_object, reject_unknown_fields};

#[cfg(test)]
mod tests;
