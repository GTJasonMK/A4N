mod dedup;
pub(super) use dedup::dedup_strings;
mod scalars;
pub(super) use scalars::{first_string, value_as_string};
