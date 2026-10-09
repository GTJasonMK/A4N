//! 项目族包：类型、纯校验与「校验后落库」application 入口。
//!
//! Host 仍负责 zip 解包/打包、媒体文件写入与失败回滚。

#[cfg(test)]
use crate::media::MEDIA_STATUS_PENDING;
use crate::media::{
    MEDIA_STATUS_COMPLETED, galgame_media_binding_required_media_kind,
    is_valid_galgame_media_binding_pair,
};
#[cfg(test)]
use serde_json::{Value, json};
use std::collections::{BTreeSet, HashMap, HashSet};

mod contracts;
mod export_assembly;
mod legacy;
pub(crate) use legacy::maximum_manifest_id;
mod validation;

#[cfg(test)]
mod tests;

pub(crate) use contracts::PROJECT_FAMILY_PACKAGE_VERSION;
#[cfg(test)]
pub(crate) use contracts::TextCorePackage;
pub(crate) use contracts::{MANIFEST_PATH, ProjectFamilyPackage};

pub(crate) use crate::project::{hash_json, hash_text, sha256_bytes};

pub(crate) use export_assembly::{
    assemble_project_family_export_package, family_media_package_path,
};
pub(crate) use validation::{validate_media_manifest, validate_package_manifest};

#[cfg(test)]
macro_rules! family_record_module {
    ($module:ident, $record:ty) => {
        mod $module {
            #[allow(dead_code)]
            pub type Model = $record;
        }
    };
}

#[cfg(test)]
mod chapter {
    pub type Model = a4n_domain::project::ProjectFamilyChapterRecord;
    pub const STATUS_IMPORTED: &str = a4n_domain::chapter::chapter_status::STATUS_IMPORTED;
    pub const STATUS_DELETED: &str = a4n_domain::chapter::chapter_status::STATUS_DELETED;
}

#[cfg(test)]
mod media_asset {
    #[allow(dead_code)]
    pub type Model = a4n_domain::project::ProjectFamilyMediaAssetRecord;
}

#[cfg(test)]
family_record_module!(
    chapter_manga_prompt,
    a4n_domain::project::ProjectFamilyMangaPromptRecord
);
#[cfg(test)]
mod chapter_outline {
    pub type Model = a4n_domain::project::ProjectFamilyChapterOutlineRecord;
    pub const STATUS_COMPLETED: &str =
        a4n_domain::chapter::chapter_outline_status::STATUS_COMPLETED;
}
#[cfg(test)]
family_record_module!(
    chapter_visual_plan,
    a4n_domain::project::ProjectFamilyVisualPlanRecord
);
#[cfg(test)]
family_record_module!(
    character_portrait,
    a4n_domain::project::ProjectFamilyCharacterPortraitRecord
);
#[cfg(test)]
family_record_module!(
    galgame_choice,
    a4n_domain::project::ProjectFamilyGalgameChoiceRecord
);
#[cfg(test)]
family_record_module!(
    galgame_line,
    a4n_domain::project::ProjectFamilyGalgameLineRecord
);
#[cfg(test)]
family_record_module!(
    galgame_scene,
    a4n_domain::project::ProjectFamilyGalgameSceneRecord
);
#[cfg(test)]
family_record_module!(
    galgame_script,
    a4n_domain::project::ProjectFamilyGalgameScriptRecord
);
#[cfg(test)]
family_record_module!(
    generated_image,
    a4n_domain::project::ProjectFamilyGeneratedImageRecord
);
#[cfg(test)]
family_record_module!(
    generated_video,
    a4n_domain::project::ProjectFamilyGeneratedVideoRecord
);
#[cfg(test)]
family_record_module!(
    generation_spec,
    a4n_domain::project::ProjectFamilyGenerationSpecRecord
);
#[cfg(test)]
family_record_module!(
    generation_spec_entity_ref,
    a4n_domain::project::ProjectFamilyGenerationSpecEntityRefRecord
);
#[cfg(test)]
family_record_module!(
    generation_spec_media_ref,
    a4n_domain::project::ProjectFamilyGenerationSpecMediaRefRecord
);
#[cfg(test)]
family_record_module!(
    generation_spec_text_anchor,
    a4n_domain::project::ProjectFamilyGenerationSpecTextAnchorRecord
);
#[cfg(test)]
family_record_module!(
    media_authoring_document,
    a4n_domain::project::ProjectFamilyAuthoringDocumentRecord
);
#[cfg(test)]
family_record_module!(
    media_binding,
    a4n_domain::project::ProjectFamilyMediaBindingRecord
);
#[cfg(test)]
family_record_module!(
    media_provenance,
    a4n_domain::project::ProjectFamilyMediaProvenanceRecord
);
#[cfg(test)]
family_record_module!(
    object_profile,
    a4n_domain::project::ProjectFamilyObjectProfileRecord
);
#[cfg(test)]
family_record_module!(
    part_outline,
    a4n_domain::project::ProjectFamilyPartOutlineRecord
);
#[cfg(test)]
family_record_module!(
    profile_evidence,
    a4n_domain::project::ProjectFamilyProfileEvidenceRecord
);
#[cfg(test)]
family_record_module!(
    project_source_snapshot,
    a4n_domain::project::ProjectFamilySourceSnapshotRecord
);
#[cfg(test)]
family_record_module!(
    prompt_artifact,
    a4n_domain::project::ProjectFamilyPromptArtifactRecord
);
#[cfg(test)]
family_record_module!(
    protagonist_profile,
    a4n_domain::project::ProjectFamilyProtagonistProfileRecord
);
#[cfg(test)]
family_record_module!(
    scene_profile,
    a4n_domain::project::ProjectFamilySceneProfileRecord
);
#[cfg(test)]
family_record_module!(
    source_snapshot_chapter,
    a4n_domain::project::ProjectFamilySourceSnapshotChapterRecord
);
#[cfg(test)]
family_record_module!(
    source_snapshot_outline,
    a4n_domain::project::ProjectFamilySourceSnapshotOutlineRecord
);
#[cfg(test)]
family_record_module!(
    source_snapshot_profile,
    a4n_domain::project::ProjectFamilySourceSnapshotProfileRecord
);
#[cfg(test)]
family_record_module!(
    source_sync_conflict,
    a4n_domain::project::ProjectFamilySourceSyncConflictRecord
);
#[cfg(test)]
family_record_module!(
    source_sync_decision,
    a4n_domain::project::ProjectFamilySourceSyncDecisionRecord
);
#[cfg(test)]
family_record_module!(
    source_sync_run,
    a4n_domain::project::ProjectFamilySourceSyncRunRecord
);
#[cfg(test)]
family_record_module!(
    text_anchor,
    a4n_domain::project::ProjectFamilyTextAnchorRecord
);
#[cfg(test)]
family_record_module!(
    video_segment,
    a4n_domain::project::ProjectFamilyVideoSegmentRecord
);
#[cfg(test)]
family_record_module!(
    assembly_timeline,
    a4n_domain::project::ProjectFamilyAssemblyTimelineRecord
);

use a4n_db::project::family::import::ProjectFamilyImportPackage;

impl ProjectFamilyPackage {
    /// 转为 DB 导入包（不含 media_files 索引）。
    pub(crate) fn into_import_package(self) -> ProjectFamilyImportPackage {
        ProjectFamilyImportPackage {
            root_project_id: self.root_project_id,
            projects: self.projects,
            text_core: self.text_core,
            source_snapshots: self.source_snapshots,
            source_snapshot_chapters: self.source_snapshot_chapters,
            source_snapshot_outlines: self.source_snapshot_outlines,
            source_snapshot_profiles: self.source_snapshot_profiles,
            text_anchors: self.text_anchors,
            profile_evidence: self.profile_evidence,
            chapter_visual_plans: self.chapter_visual_plans,
            chapter_shooting_scripts: self.chapter_shooting_scripts,
            authoring_documents: self.authoring_documents,
            authoring_receipts: self.authoring_receipts,
            character_portraits: self.character_portraits,
            generated_images: self.generated_images,
            chapter_manga_prompts: self.chapter_manga_prompts,
            manga_pipeline_artifacts: self.manga_pipeline_artifacts,
            chapter_video_prompts: self.chapter_video_prompts,
            generated_videos: self.generated_videos,
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
