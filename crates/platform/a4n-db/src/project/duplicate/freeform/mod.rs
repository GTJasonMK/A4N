mod canvas;
mod contracts;
mod media;
mod records;
mod remap;
mod workflow;

pub use workflow::duplicate_freeform_project_record;

pub(in crate::project::duplicate::freeform) use contracts::{
    CopiedFreeformGenerationSpecs, DeferredFreeformSpecTarget, FreeformCanvasCopyMaps,
    FreeformCopyMaps, FreeformGeneratedImageCopyRecords, FreeformGeneratedVideoCopyRecords,
    FreeformProjectMediaCopyResult, FreeformProjectMediaGraphCopyResult,
};
