mod common;
mod preprocessing;

pub(super) use common::required_manga_string;
pub(in crate::manga) use preprocessing::validate_manga_preprocessing;
