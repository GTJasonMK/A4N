//! 故事蓝图主体、版本和项目同步的 DB 边界。

pub mod context;

mod contracts;
mod manual;
mod project;
mod record;
mod refine;
mod version;

pub use contracts::{
    BlueprintDbError, BlueprintMutationStatusChange, BlueprintRefinePatchInput,
    InsertBlueprintVersionInput, ManualBlueprintUpsertInput, NovelBlueprintUpsertInput,
    SaveManualBlueprintInput, SyncProjectAfterBlueprintInput, UpdateBlueprintWorldSettingInput,
    VersionedBlueprintMutationResult,
};
pub(crate) use manual::locked_blueprint_sources;
pub use manual::{
    next_blueprint_version_record, save_manual_blueprint_records_transaction,
    update_blueprint_world_setting_records_transaction, validate_blueprint_chapter_plan,
};
pub use project::sync_project_after_blueprint_record;
pub use record::{blueprint_record, upsert_manual_blueprint_record, upsert_novel_blueprint_record};
pub use refine::apply_blueprint_refine_patch_record;
pub use version::{blueprint_version_snapshot, insert_blueprint_version_record};
