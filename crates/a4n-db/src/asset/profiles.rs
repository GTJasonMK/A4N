mod mutations;
mod read;
mod record;
mod upsert;

pub use mutations::{
    create_object_profile_record, delete_object_profile_record, update_object_profile_record,
};
pub(in crate::asset) use read::ensure_object_profile_project_exists;
pub use read::{object_profile_by_id, object_profiles_by_project};
pub(in crate::asset) use upsert::upsert_object_profiles_in;
