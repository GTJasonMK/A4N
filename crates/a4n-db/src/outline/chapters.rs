mod assignments;
mod deletion;
mod records;
mod rules;
mod sync;

pub use assignments::sync_chapter_outline_part_assignments;
pub use deletion::delete_chapter_outlines_in_part_range;
pub use records::{insert_chapter_outline_record, update_chapter_outline_record};
pub use sync::{
    sync_chapter_outline_records_by_number, sync_chapter_outline_records_by_number_transaction,
};

#[cfg(test)]
mod tests;
