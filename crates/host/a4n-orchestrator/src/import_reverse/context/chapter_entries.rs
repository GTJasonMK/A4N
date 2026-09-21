// Chapter entry helpers share one compact projection contract across reverse targets.
mod analysis;
mod formatting;
mod sample;
mod sampling;
mod structure;
mod summary;

#[cfg(test)]
mod tests;

pub(super) use sample::sample_chapter_entry;
pub(super) use sampling::sampled_chapters;
pub(super) use structure::{chapter_outline_entry, chapter_part_entry};
pub(super) use summary::chapter_summary_for_reverse;
