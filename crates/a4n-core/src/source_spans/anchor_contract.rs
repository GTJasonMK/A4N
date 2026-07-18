mod chapter_numbers;
mod contracts;
mod deprecated;
mod exact_chapter;
mod lookup;

pub use chapter_numbers::{anchor_quote_with_chapter_number, anchor_quotes_with_chapter_numbers};
pub use contracts::AnchorQuoteWithChapter;
pub use deprecated::{reject_deprecated_output_fields, reject_deprecated_reference_fields};
pub use exact_chapter::anchor_quotes_in_exact_chapter;

#[cfg(test)]
mod tests;
