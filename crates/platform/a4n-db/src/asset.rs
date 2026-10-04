mod contracts;
mod generation;
mod profiles;

pub use contracts::{AssetDbError, ObjectProfileUpsertRecordResult};
pub use generation::upsert_object_profiles_with_generation_run_record;
pub(crate) use profiles::object_profile_record;
pub use profiles::{
    create_object_profile_read_record, create_object_profile_record, delete_object_profile_record,
    object_profile_by_id, object_profile_project_id, object_profile_record_by_id,
    object_profile_records_by_project, object_profiles_by_project,
    update_object_profile_read_record, update_object_profile_record,
};
