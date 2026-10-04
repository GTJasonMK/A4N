mod contracts;
mod creation;
mod queries;
mod rules;
mod updates;

pub use contracts::{
    CreateCanvasProjectParams, CreateProjectRecordInput, CreateStandaloneComicProjectParams,
    PROJECT_KIND_CANVAS, PROJECT_KIND_COMIC, PROJECT_KIND_TEXT, PROJECT_KIND_VIDEO,
    ProjectMutationError, SOURCE_SYNC_CURRENT, UpdateProjectRecordInput,
};
pub(crate) use contracts::{SOURCE_SYNC_CONFLICT, SOURCE_SYNC_DETACHED, SOURCE_SYNC_ROOT_UPDATED};
pub use creation::{
    create_canvas_project_record, create_project_record, create_standalone_comic_project_record,
};
pub use queries::{project_by_id_record, project_ids, project_summary_by_id_record};
pub use rules::{
    is_valid_source_sync_status, normalize_derived_project_kind, normalize_project_kind,
    normalize_project_type,
};
pub use updates::update_project_record;

pub(crate) use queries::{project_by_id_for_update, project_write_identity_for_update};
pub(super) use rules::required_project_title;
pub(crate) use updates::mark_derived_projects_root_updated_in_record;
