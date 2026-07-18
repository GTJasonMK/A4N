//! 故事蓝图主体、版本和项目同步的 DB 边界。

mod contracts;
mod project;
mod record;
mod refine;
mod version;

pub use contracts::{
    BlueprintDbError, BlueprintRefinePatchInput, InsertBlueprintVersionInput,
    NovelBlueprintUpsertInput, SyncProjectAfterBlueprintInput,
};
pub use project::sync_project_after_blueprint_record;
pub use record::upsert_novel_blueprint_record;
pub use refine::apply_blueprint_refine_patch_record;
pub use version::insert_blueprint_version_record;

#[cfg(test)]
mod tests;
