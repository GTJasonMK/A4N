mod contracts;
mod creation;
mod queries;
mod rules;
mod updates;

pub use contracts::{
    CreateFreeformVideoProjectParams, CreateProjectRecordInput, PROJECT_KIND_COMIC,
    PROJECT_KIND_GALGAME, PROJECT_KIND_TEXT, PROJECT_KIND_VIDEO, PROJECT_TYPE_NOVEL_LONG,
    PROJECT_TYPE_NOVEL_SHORT, PROJECT_TYPE_PROSE, PROJECT_TYPE_SCRIPT, ProjectMutationError,
    SOURCE_SYNC_CONFLICT, SOURCE_SYNC_CURRENT, SOURCE_SYNC_DETACHED, SOURCE_SYNC_ROOT_UPDATED,
    UpdateProjectRecordInput,
};
pub use creation::{create_freeform_video_project_record, create_project_record};
pub use queries::{project_by_id_record, project_ids};
pub use rules::{
    is_valid_source_sync_status, normalize_derived_project_kind, normalize_project_kind,
    normalize_project_type,
};
pub use updates::update_project_record;

pub(crate) use queries::project_by_id_for_update;
pub(super) use rules::required_project_title;
pub(crate) use updates::mark_derived_projects_root_updated_in_record;
