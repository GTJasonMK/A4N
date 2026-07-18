mod analysis;
mod collector;
mod schema;
mod shape;

pub(super) use analysis::diagnose_structured_json_with_schema;
pub(super) use schema::structured_schema_sha256;
pub(crate) use shape::json_shape_schema_from_example;

#[cfg(test)]
mod tests;
