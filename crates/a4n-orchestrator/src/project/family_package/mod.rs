//! 项目族包：类型、纯校验与「校验后落库」application 入口。
//!
//! Host 仍负责 zip 解包/打包、媒体文件写入与失败回滚。

#[cfg(test)]
use crate::media::MEDIA_STATUS_PENDING;
use crate::media::{
    MEDIA_STATUS_COMPLETED, galgame_media_binding_required_media_kind,
    is_valid_galgame_media_binding_pair,
};
use crate::source_sync::CONFLICT_STATUS_PENDING;
#[cfg(test)]
use crate::source_sync::{
    CONFLICT_STATUS_RESOLVED, SYNC_RUN_MODE_APPLY, SYNC_RUN_MODE_DRY_RUN,
    SYNC_RUN_STATUS_COMPLETED, SYNC_RUN_STATUS_PLANNED,
};
#[cfg(test)]
use crate::text_anchor::{EVIDENCE_ROLE_FIRST_SEEN, SOURCE_KIND_CHAPTER_CONTENT};
#[cfg(test)]
use a4n_core::models::{
    asset::object_profile,
    chapter::{chapter, part_outline},
    character::protagonist_profile,
    generation::{
        generation_spec_entity_ref, generation_spec_media_ref, generation_spec_text_anchor,
    },
    media::media_provenance,
    project::{
        profile_evidence, project_derivation, source_snapshot_outline, source_snapshot_profile,
    },
    scene::scene_profile,
    video_project::{assembly_timeline, video_segment},
};
use a4n_core::models::{
    chapter::chapter_outline,
    galgame::{galgame_choice, galgame_line, galgame_scene, galgame_script},
    generation::{generation_spec, prompt_artifact},
    media::{media_asset, media_binding},
    project::{novel_project, project_source_snapshot, source_snapshot_chapter, text_anchor},
    source_sync::{source_sync_conflict, source_sync_decision, source_sync_run},
};
#[cfg(test)]
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap, HashSet};

mod contracts;
mod export_assembly;
mod validation;

#[cfg(test)]
mod tests;

pub use contracts::{
    FamilyImportMediaPayload, MANIFEST_PATH, MEDIA_FILE_FIELD_FILE, MEDIA_FILE_FIELD_THUMBNAIL,
    MEDIA_FILE_FIELDS, MediaFileEntry, PROJECT_FAMILY_PACKAGE_VERSION, ProjectFamilyPackage,
    TextCorePackage,
};

// 纯规则：与 package_rules 对齐，供校验子树 `use super::*` / 相对路径使用。
#[cfg(test)]
pub(crate) use crate::project::media_file_key;
pub(crate) use crate::project::{hash_json, hash_text, sha256_bytes};

pub use export_assembly::{
    assemble_project_family_export_package, family_media_package_path, validate_export_same_user,
    validate_export_snapshot_coverage,
};
pub use validation::{
    snapshot_profile_key, validate_media_bytes, validate_media_manifest, validate_package_manifest,
};

use a4n_db::project_family_import::ProjectFamilyImportPackage;

impl ProjectFamilyPackage {
    /// 转为 DB 导入包（不含 media_files 索引）。
    pub fn into_import_package(self) -> ProjectFamilyImportPackage {
        ProjectFamilyImportPackage {
            root_project_id: self.root_project_id,
            projects: self.projects,
            text_core: self.text_core,
            project_derivations: self.project_derivations,
            source_snapshots: self.source_snapshots,
            source_snapshot_chapters: self.source_snapshot_chapters,
            source_snapshot_outlines: self.source_snapshot_outlines,
            source_snapshot_profiles: self.source_snapshot_profiles,
            text_anchors: self.text_anchors,
            profile_evidence: self.profile_evidence,
            generation_specs: self.generation_specs,
            generation_spec_text_anchors: self.generation_spec_text_anchors,
            generation_spec_entity_refs: self.generation_spec_entity_refs,
            generation_spec_media_refs: self.generation_spec_media_refs,
            prompt_artifacts: self.prompt_artifacts,
            media_assets: self.media_assets,
            media_provenance: self.media_provenance,
            media_bindings: self.media_bindings,
            galgame_scripts: self.galgame_scripts,
            galgame_scenes: self.galgame_scenes,
            galgame_lines: self.galgame_lines,
            galgame_choices: self.galgame_choices,
            video_segments: self.video_segments,
            assembly_timelines: self.assembly_timelines,
            assembly_clips: self.assembly_clips,
            source_sync_runs: self.source_sync_runs,
            source_sync_conflicts: self.source_sync_conflicts,
            source_sync_decisions: self.source_sync_decisions,
        }
    }
}
