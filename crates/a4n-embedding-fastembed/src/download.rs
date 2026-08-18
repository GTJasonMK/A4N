mod cache;
mod client;
mod contracts;
mod execution;
mod info;
mod metadata;
mod progress;
mod transfer;

pub use execution::download_fastembed_model;
pub use info::fastembed_model_download_info;

pub(in crate::download) use contracts::{DOWNLOAD_CANCELLED_MESSAGE, FastEmbedDownloadFile};
