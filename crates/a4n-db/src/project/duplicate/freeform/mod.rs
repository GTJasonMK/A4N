mod canvas;
mod contracts;
mod media;
mod records;
mod remap;
mod rules;
mod workflow;

pub use workflow::duplicate_freeform_project_record;

pub(in crate::project::duplicate::freeform) use contracts::{
    CopiedFreeformGenerationSpecs, DeferredFreeformSpecTarget, FreeformCanvasCopyMaps,
    FreeformCopyMaps, FreeformGeneratedImageCopyRecords, FreeformGeneratedVideoCopyRecords,
    FreeformProjectMediaCopyResult, FreeformProjectMediaGraphCopyResult,
};
pub(in crate::project::duplicate::freeform) use rules::ensure_freeform_source_mode;

#[cfg(test)]
mod tests;
