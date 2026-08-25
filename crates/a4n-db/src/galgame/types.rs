mod errors;
mod generation_specs;
mod read_models;
mod script_records;
mod targets;

pub use self::errors::{
    GalgameAssetPlanSpecRecordError, GalgameContextRecordError, GalgameGenerationRecordError,
    GalgameLineRecordError, GalgameLineReorderRecordError, GalgameMediaRecordError,
    GalgameProjectRecordError, GalgameSceneRecordError, GalgameScriptRecordError,
};
pub use self::generation_specs::{
    GalgameAssetPlanSpecRecordInput, GalgameGenerationSpecUpsert, GalgameGenerationSpecUpsertInput,
    GalgameGenerationSpecUpsertOutcome,
};
pub(in crate::galgame) use self::generation_specs::{
    GalgameGenerationSpecEntityRefRecordInput, GalgameGenerationSpecRefsRecordInput,
    GalgameGenerationSpecTextAnchorRefRecordInput,
};
pub use self::read_models::{
    CompletedGalgameVisualBindingRecords, GalgameAssetPlanningReadModel,
    GalgameAssetPlanningSceneRecord, GalgameAssetPlanningSourceChapterRecord,
    GalgameCompletedVisualAssetRecord, GalgameCompletedVisualBindingRecord,
    GalgameMediaAssetReadRecord, GalgameMediaBindingReadModel, GalgameMediaBindingReadRecord,
    GalgameMediaBindingResult, GalgamePersistValidationScopeReadModel,
    GalgamePersistValidationSourceSnapshotChapterRecord,
    GalgamePersistValidationSpeakerProfileRecord, GalgamePersistValidationTextAnchorRecord,
    GalgamePipelinePromptChapterRecord, GalgamePipelinePromptChapterSourceRecord,
    GalgamePipelinePromptContextRecord, GalgamePipelinePromptSpeakerProfileRecord,
    GalgamePipelinePromptTextAnchorRecord, GalgameScriptDetailChoiceRecord,
    GalgameScriptDetailReadModel, GalgameScriptDetailSceneRecord, GalgameScriptDetailScriptRecord,
};
pub use self::script_records::{
    InsertGalgameLineRecordInput, PersistGalgameChoiceRecordInput, PersistGalgameLineRecordInput,
    PersistGalgameSceneRecordInput, PersistGalgameScriptRecordInput,
    ReplaceGalgameChoiceRecordInput, UpdateGalgameLineRecordInput, UpdateGalgameSceneRecordInput,
};
pub use self::targets::{
    GalgamePortraitSpecTargetInput, GalgamePortraitSpecTargetProfile,
    GalgamePromptArtifactUpsertInput, GalgameTargetContext,
};
