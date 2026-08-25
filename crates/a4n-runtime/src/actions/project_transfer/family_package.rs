//! 项目族包 ZIP、manifest 与媒体 payload 适配。

use a4n_orchestrator::project::{
    FAMILY_PACKAGE_MANIFEST_PATH as MANIFEST_PATH, ProjectFamilyPackage,
};

mod archive;
mod package_io;

pub(in crate::actions::project_transfer) use archive::read_project_family_package_bytes;
pub(in crate::actions::project_transfer) use package_io::package_zip_bytes;
