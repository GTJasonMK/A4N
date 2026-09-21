mod artifacts;
pub(crate) use artifacts::{
    inspect_data_file_facts, inspect_generated_media_file, promote_data_file,
};

#[cfg(test)]
mod tests;
