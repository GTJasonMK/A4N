mod execution;
mod record;
#[cfg(test)]
mod tests;
mod trace;
mod validation;

pub use execution::{
    create_generated_image_record, create_generated_image_record_with_generation_run_checkpoint,
};
pub use validation::validate_create_generated_image_record_input;
