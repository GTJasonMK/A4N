use a4n_api_contract::command_manifest::{
    API_COMMANDS, RuntimeConfigPolicy, command_runtime_config_policy,
};
use a4n_api_contract::generation_tasks::GENERATION_START_ROUTES;
use a4n_api_contract::{
    admin::{
        AdminOverviewSummary, AdminProjectSummary, CallKindOverview, CreateInviteCodeInput,
        CreateUserInput, DailyCallMetric, DailyUsagePoint, GenerationCallAnalytics,
        GenerationCallAnalyticsInput, InviteCodeSummary, LlmWorkflowMetric, MediaWorkflowMetric,
        RecentLlmCall, RecentMediaCall, TaskQueueSummary, ThemeInfo, UpdateUserRoleInput,
        UsageLogSummary, UsageTrendsInput,
    },
    auth::{
        AuthResult, ChangePasswordInput, LoginInput, RefreshTokenInput, RegisterInput,
        RevokeSessionInput, SessionSummary, TokenPairSummary, UpdateProfileInput, UserSummary,
    },
    blueprint::{
        BlueprintDiffSummary, BlueprintMutationResult, BlueprintSummary, BlueprintVersionSummary,
        DiffBlueprintVersionsInput, ListBlueprintVersionsInput, PreviewBlueprintGenerationInput,
        PreviewBlueprintRefinementInput, RestoreBlueprintVersionInput, SaveBlueprintInput,
        UpdateWorldSettingInput,
    },
    canvas::{
        CanvasEdgeInput, CanvasEdgeSummary, CanvasGraphDocument, CanvasGraphInput,
        CanvasGraphSummary, CanvasNodeContract, CanvasNodeInput, CanvasNodeRequestPreview,
        CanvasNodeSummary, CanvasPortContract, CanvasRunSummary, CanvasTextAssetInput,
        CanvasTextAssetSummary, GetCanvasGraphInput, GetCanvasNodeContractsInput,
        ImportCanvasMediaInput, ImportedCanvasMedia, PreviewCanvasNodeRequestInput,
        RunCanvasGraphInput, RunCanvasNodeInput, SaveCanvasGraphInput, SaveCanvasTextAssetInput,
    },
    chapter::{
        CHAPTER_ANALYSIS_PROGRESS_TOTAL, CHAPTER_SCENE_PROGRESS_TOTAL, ChapterAnalysisInput,
        ChapterAnalysisMutationResult, ChapterMaterialSyncState, ChapterMaterialSyncStatus,
        ChapterMaterialUpdate, ChapterMaterialUpdateAction, ChapterMaterialUpdateKind,
        ChapterSceneSummary, ChapterSummary, GenerateChapterSceneInput, ImportChapterInput,
        ReorderChapterScenesInput, UpdateChapterContentInput, UpdateChapterSceneInput,
    },
    character::{
        AddProfileAttributeInput, ApplyImplicitUpdateDecisionInput,
        ApplyImplicitUpdateDecisionResultSummary, AttributeCountsSummary, AttributeEventSummary,
        AttributeHistoryInput, BehaviorRecordSummary, CharacterProfileCreateResult,
        CharacterProfileDeleteResult, CharacterProfileMutationResult,
        CharacterProfileProtectedAttribute, CharacterProfileSkippedResult, CharacterProfileSummary,
        CharacterProfileSyncBatchStatus, CharacterProfileSyncFailure, CharacterProfileSyncRange,
        CharacterProfileSyncSkipReason, CharacterProfileSyncedResult, CheckImplicitUpdateInput,
        CheckProfileSyncConflictInput, CreateProfileInput, DeletionMarkSummary,
        DiffProfileChaptersInput, ExecuteAttributeDeletionInput, GetImplicitReviewInput,
        GetProfileStateAtChapterInput, GetSnapshotInput, ImplicitAttributeReviewSummary,
        ImplicitEvidenceSummary, ImplicitReviewSummary, ImplicitUpdateDecisionSummary,
        ListBehaviorsInput, ListDeletionMarksInput, ListProfilesInput, ListSnapshotsInput,
        ModifyProfileAttributeInput, ProfileChapterStateSummary, ProfileSyncConflictSummary,
        RequestAttributeDeletionInput, ResetDeletionMarksInput, RollbackResultSummary,
        RollbackToChapterInput, SnapshotDeltaSummary, SnapshotReferenceSummary, SnapshotSummary,
        SyncAllCharacterProfilesInput, SyncAllCharacterProfilesResult, SyncCharacterProfileInput,
        SyncCharacterProfileResult, UpdateCharacterProfileInput,
    },
    collaboration::{
        InviteProjectMemberInput, ProjectAccessRole, ProjectAccessScope, ProjectInviteSummary,
        ProjectMemberRole, ProjectMemberSummary, UpdateProjectMemberRoleInput,
    },
    config::{
        CreateEmbeddingInput, CreateLlmInput, DiscoveredModel, EmbeddingConfigSummary,
        ImageConfigInput, ImageConfigSummary, ImportAllConfigsInput, LlmConfigSummary,
        LlmMaxTokensConfig, ModelDiscoveryInput, ModelDiscoveryResult, UpdateEmbeddingInput,
        UpdateImageConfigInput, UpdateLlmInput, UpdateVideoConfigInput, VideoConfigInput,
        VideoConfigSummary, VideoDurationMode,
    },
    embedding::{
        CancelEmbeddingModelDownloadInput, DownloadEmbeddingModelInput,
        EmbeddingModelDownloadInfoSummary, EmbeddingModelIdentity, EmbeddingModelInput,
        LocalEmbedTextsInput, LocalEmbedTextsOutput, LocalEmbeddingConfigInput,
        LocalEmbeddingModelSummary,
    },
    export::{
        BatchExportFormat, BatchExportInput, ExportFileData, ExportProjectInput,
        ExportPublicationInput, ExportPublicationMetadata,
    },
    galgame::{
        BindGalgameMediaInput, DeleteGalgameLineInput, GalgameAssetGenerationItem,
        GalgameAssetGenerationSummary, GalgameAssetPlan, GalgameAssetPlanItem,
        GalgameAssetPlanOutcome, GalgameAssetPlanSummary, GalgameChoice, GalgameLine,
        GalgameMediaAsset, GalgameMediaBinding, GalgameMediaBindingSummary,
        GalgamePipelineStatusSummary, GalgameProjectInput, GalgameScene, GalgameSceneDetail,
        GalgameScriptDetail, GalgameSpeakerProfile, GalgameTextAnchorSummary,
        GalgameWorkflowCheckpointSummary, GalgameWorkflowRunSummary, ImportGalgameAudioInput,
        InsertGalgameLineInput, PreviewGalgamePipelineInput, ReorderGalgameLinesInput,
        RunGalgameAssetGenerationInput, StartGalgamePipelineInput, UnbindGalgameMediaInput,
        UpdateGalgameChoiceOptionInput, UpdateGalgameChoicesInput, UpdateGalgameLineInput,
        UpdateGalgameSceneInput,
    },
    image::{
        DeleteImageInput, GeneratedImageSummary, ListImagesByChapterInput,
        ListImagesByProjectInput, ListVideoReferenceImagesInput, UpdateGeneratedImageInput,
        UploadImageInput,
    },
    image_generation::{
        GenerateBatchImagesInput, GeneratePageImageInput, GenerateSceneImageInput,
        ObjectProfileImageInput, ObjectProfileImagePromptOutput, SceneProfileImageInput,
        SceneProfileImagePromptOutput,
    },
    import::{ImportProjectTextUploadInput, MAX_PROJECT_TEXT_UPLOAD_BYTES},
    import_reverse::{ConfirmReverseEngineeringCandidateInput, ReverseEngineerInput},
    import_review::{
        AcceptImportReviewInput, BatchAcceptImportReviewsInput, CheckImportDedupInput, DedupResult,
        ImportProjectReviewsInput, ImportReviewIdInput, ImportReviewSummary,
    },
    inspiration::{
        InspirationHistoryInput, InspirationMessageSummary, PreviewInspirationRequestInput,
    },
    logging::{LogReadResult, ReadLogsInput, SetLogLevelInput},
    manga::{
        BindMangaReferenceImageInput, DeleteMangaPromptInput, MangaBoundReferenceAsset,
        MangaCameraAngleV2, MangaChapterInput, MangaPageDrawingPromptSchemaVersion,
        MangaPageDrawingPromptV2, MangaPageImportanceV2, MangaPagePacingV2,
        MangaPagePlanSchemaVersion, MangaPagePromptSourceSummary, MangaPagePromptSummary,
        MangaPageRoleV2, MangaPageV3, MangaPanelFunctionV2, MangaPanelSizeV2, MangaPanelV2,
        MangaPipelineResumeInput, MangaPipelineStartInput, MangaPipelineStatusSummary,
        MangaReadingFlowV2, MangaReferenceAssetKind, MangaReferenceBindingSummary,
        MangaShootingBeatRefV3, MangaShotScaleV2, MangaTextLayerKindV2, MangaTextLayerV2,
        MangaTextPlacementV2, OptimizeMangaPromptInput, UpdateMangaPromptInput,
    },
    optimize::{
        AnalyzeChapterInput, ApplyOptimizationPatchesInput, ApplyOptimizationPatchesResponse,
        CreateOptimizationChatSessionInput, OptimizationAgentPermissions, OptimizationPatch,
        OptimizationPatchSetSummary, OptimizationSessionSummary, OptimizationTurnEventSummary,
        OptimizationTurnSummary, PrepareOptimizationAnalysisSessionInput,
        ProposeOptimizationPatchesInput, ProposeOptimizationPatchesResponse,
        SendOptimizationMessageInput,
    },
    outline::{
        ApplyBeatMappingInput, BatchGeneratePartOutlinesInput, ChapterOutlinePacing,
        ChapterOutlinePacingAnalysis, ChapterOutlineSummary, ContinuePartOutlinesInput,
        DeleteLatestOutlineRecordsInput, GenerateChapterOutlinesByCountInput,
        GeneratePartChapterOutlinesInput, GeneratePartsInput, OutlineDeleteTarget,
        OutlineGenerationProgressSummary, OutlineMutationResult, PartOutlineSummary,
        RegenerateChapterOutlineInput, RegenerateLastPartInput, RegeneratePartOutlineInput,
        RegeneratePartsInput, SaveChapterOutlineInput, SaveChapterOutlineResult,
        SavePartOutlineInput, SavePartOutlineResult, UpdateChapterOutlineInput,
        UpdateChapterOutlineResult,
    },
    portrait::{
        GenerateAgeVariantInput, GenerateMissingPortraitsInput, GeneratePortraitInput,
        GeneratePortraitPromptInput, GetPrimaryPortraitInput, ListPortraitsByProjectInput,
        PortraitSummary, PortraitType, RegeneratePortraitInput, SetPrimaryPortraitInput,
        UploadPortraitInput,
    },
    profile::{
        CreateForeshadowingInput, CreateObjectProfileInput, CreateSceneProfileInput,
        DeleteForeshadowingInput, DeleteObjectProfileInput, DeleteSceneProfileInput,
        ForeshadowingSummary, ForeshadowingWriteResult, ListForeshadowingInput,
        ListObjectProfilesInput, ListProfileMaterialsInput, ListSceneProfilesInput,
        ObjectProfileDeleteResult, ObjectProfileInput, ObjectProfileMutationResult,
        ObjectProfileSummary, ProfileMaterialGroup, ProfileMaterialImageSummary,
        ProfileMaterialProfileSummary, SceneProfileDeleteResult, SceneProfileInput,
        SceneProfileMutationResult, SceneProfileSummary, SetProfilePrimaryMaterialInput,
        SyncSceneProfilesInput, UpdateForeshadowingInput, UpdateObjectProfileInput,
        UpdateSceneProfileInput, UploadProfileMaterialImageInput,
    },
    project::{
        AssetDependencyGraph, AssetGraphEdge, AssetGraphLayer, AssetGraphNode,
        AssetGraphNodeDetail, AssetLineageRecord, AssetLineageRef, CreateDerivedProjectInput,
        CreateFreeformVideoProjectInput, CreateProjectInput, DuplicateProjectInput,
        GenerationInputSource, GenerationRunSnapshot, PaginatedProjects,
        ProjectGeneratedDataResetResponse, ProjectGeneratedDataResetStatus,
        ProjectGeneratedDataResetSummary, ProjectKind, ProjectProgressSummary,
        ProjectRollbackImpactSummary, ProjectRollbackPreview, ProjectStatus,
        ProjectStatusHistorySummary, ProjectStatusMachineSummary, ProjectSummary, PromptSource,
        UpdateProjectInput,
    },
    project_transfer::{
        ExportDerivedProjectDataInput, ExportProjectFamilyDataInput, ExportProjectPackageData,
        ExportVideoStudioProjectDataInput, ImportProjectPackageUploadInput,
        PROJECT_FAMILY_PACKAGE_SUFFIX, VIDEO_STUDIO_PROJECT_PACKAGE_SUFFIX,
    },
    prompt::{
        AbRenderResult, CategoryInfo, ProjectPromptOverrideSummary, PromptEntrySummary,
        PromptVersionSummary, RemoveOverrideInput, RenderInput, SetOverrideInput,
        UpdatePromptInput, ValidateVarsInput, VersionDiffInput,
    },
    provider_catalog::{
        EmbeddingProviderDescriptor, ImageProviderDescriptor, LlmProviderDescriptor,
        ModelProviderCatalog, VideoProviderDescriptor,
    },
    rag::{
        EnhancedSearchInput, RagEnhancedQuerySummary, RagEnhancedSearchOutput,
        RagEnhancedSearchWithVectorsInput, RagPreparedEnhancedSearchOutput, RagSearchResult,
    },
    request_preview::{ModelRequestPreview, ModelRequestPreviewStep},
    shooting_script::{
        ChapterShootingScriptRecordSummary, ChapterShootingScriptSchemaVersion,
        ChapterShootingScriptSourceInput, ChapterShootingScriptUnitProgressView,
        ChapterShootingScriptV1, ChapterShootingScriptView, EditChapterShootingScriptInput,
        EditedChapterShootingScriptView, ShootingBlockingV1, ShootingCameraAngleV1,
        ShootingCameraHeightV1, ShootingCameraMovementV1, ShootingContinuityIntentV1,
        ShootingCoverageV1, ShootingDialogueCueV1, ShootingDirectingStatementV1,
        ShootingEditingModeV1, ShootingPerformanceBeatV1, ShootingSceneV1, ShootingShotSizeV1,
        ShootingShotV1, ShootingSoundDesignV1, ShootingSpatialContinuityV1,
        ShootingTransitionContractV1, ShootingViewpointModeV1, ShootingViewpointV1,
    },
    source_sync::{
        ApplySourceSyncInput, SourceSyncConflictSummary, SourceSyncDecisionInput,
        SourceSyncPlanSummary, SourceSyncRunSummary,
    },
    storage::{OrphanProjectDataDirSummary, OrphanProjectDataDirsSummary, StoragePathsSummary},
    task::{
        CancelTaskInput, GetTaskHistoryInput, QueueSlotStatsSummary, QueueStatsSummary,
        TaskInfoSummary, TaskProgressSummary,
    },
    theme::{CreateThemeInput, ThemeMode, ThemeSummary, UpdateThemeInput},
    video::{
        ApplyAssemblyPostprocessEditInput, AssembledChapterVideoClipSummary,
        AssembledChapterVideoPreview, AssemblyPostprocessOperation, AssemblyPushAction,
        AssemblyPushResult, ChapterVideoPromptSummary, CommitInsertedSegmentDraftInput,
        DeleteChapterVideoPromptInput, DeleteInsertedSegmentInput, DownstreamConflictStrategy,
        GenerateChapterVideosBatchResult, GeneratedVideoSummary, GetVideoTimelineInput,
        ImportCanvasVideoInput, MoveInsertedSegmentInput, OptimizeChapterVideoPromptInput,
        PreviewChapterVideoPromptOverrideInput, PushAssembledChapterVideoInput,
        UpdateAssemblyPushStatusInput, UpdateChapterVideoPromptInput, UpdateGeneratedVideoInput,
        UploadAssemblyInsertVideoInput, UploadChapterVideoInput, UploadReferenceMediaVideoInput,
        VideoPlaybackData, VideoScrubFramesData, VideoSegmentTarget, VideoTimelineItemKind,
        VideoTimelineItemStatus, VideoTimelineItemSummary, VideoTimelineSummary,
    },
    workflow::{
        CancelWorkflowInput, GenerationRunSummary, GetWorkflowRunInput, ListWorkflowRunsInput,
        ResumeWorkflowRunInput, WorkflowCheckpointSummary, WorkflowProgressEvent,
        WorkflowProgressEventsInput, WorkflowProgressEventsResponse, WorkflowRunDetailSummary,
        WorkflowRunProgressSummary, WorkflowRunSummary, WorkflowStateSummary,
    },
};
use a4n_domain::character::{
    AppliedImplicitUpdateAction, CHARACTER_DELETION_THRESHOLD, CharacterAttributeCategory,
    CharacterAttributeOperation, CharacterTimelineSourceKind, DeletionMarkStatus,
    ForeshadowingRecordInput, ForeshadowingStatus, ImplicitUpdateDecision,
};
use a4n_domain::galgame::{
    GalgameAudioRole, GalgameCgImportance, GalgameFacingV2, GalgameGenerationTargetKind,
    GalgameLinePresentationDocumentV2, GalgameLinePresentationSchemaVersion, GalgameLineType,
    GalgameMediaRole, GalgameMediaTargetKind, GalgamePipelineBeatRefV3, GalgamePipelineStartStage,
    GalgameSceneEntryKind, GalgameStageLayerV2, GalgameStagePositionV2, GalgameTargetEntityKind,
    GalgameTransition, GalgameVisibleCastMemberV2,
};
use a4n_domain::import_review::ImportReviewStatus;
use a4n_domain::profile_material::{
    ProfileMaterialFilter, ProfileMaterialImageStaleness, ProfileMaterialKind,
    ProfileMaterialKindFilter, ProfileMaterialSort, ProfileMaterialState,
};
use a4n_domain::progress::{ProgressMeta, ProgressPhaseKind, ProgressSeverity};
use a4n_domain::{
    DEFAULT_LLM_MAX_CONTEXT_TOKENS, DEFAULT_LLM_MAX_OUTPUT_TOKENS, LLM_MAX_TOKEN_LIMIT,
    LLM_RESERVED_REQUEST_OPTION_KEYS,
};
use schemars::JsonSchema;
use serde_json::Value;
use std::{env, fs, path::PathBuf};

fn main() {
    let theme_output = [
        "// @generated by `cargo run -p a4n-api-contract --bin generate_types -- --write`.\n",
        "// Do not edit this file; change the Rust contract and regenerate it.\n\n",
        &typescript_enum::<ThemeMode>("ThemeMode"),
        &typescript_interface::<CreateThemeInput>("CreateThemeInput"),
        &typescript_interface::<UpdateThemeInput>("UpdateThemeInput"),
        &typescript_interface::<ThemeSummary>("ThemeSummary"),
        "export type Theme = ThemeSummary;\n",
    ]
    .concat();

    let auth_output = [
        "// @generated by `cargo run -p a4n-api-contract --bin generate_types -- --write`.\n",
        "// Do not edit this file; change the Rust contract and regenerate it.\n\n",
        &typescript_interface::<UserSummary>("UserSummary"),
        &typescript_interface::<RegisterInput>("RegisterInput"),
        &typescript_interface::<LoginInput>("LoginInput"),
        &typescript_interface::<UpdateProfileInput>("UpdateProfileInput"),
        &typescript_interface::<ChangePasswordInput>("ChangePasswordInput"),
        &typescript_interface::<RevokeSessionInput>("RevokeSessionInput"),
        &typescript_interface::<SessionSummary>("SessionSummary"),
        &typescript_interface::<TokenPairSummary>("TokenPairSummary"),
        &typescript_interface::<AuthResult>("AuthResult"),
        &typescript_interface::<RefreshTokenInput>("RefreshTokenInput"),
    ]
    .concat();

    let config_output = [
        "// @generated by `cargo run -p a4n-api-contract --bin generate_types -- --write`.\n",
        "// Do not edit this file; change the Rust contract and regenerate it.\n\n",
        &typescript_enum::<VideoDurationMode>("VideoDurationMode"),
        &typescript_interface::<ModelDiscoveryInput>("ModelDiscoveryInput"),
        &typescript_interface::<DiscoveredModel>("DiscoveredModel"),
        &typescript_interface::<ModelDiscoveryResult>("ModelDiscoveryResult"),
        &typescript_interface::<EmbeddingConfigSummary>("EmbeddingConfigSummary"),
        &typescript_interface::<CreateEmbeddingInput>("CreateEmbeddingInput"),
        &typescript_interface::<UpdateEmbeddingInput>("UpdateEmbeddingInput"),
        &typescript_interface::<ImageConfigSummary>("ImageConfigSummary"),
        &typescript_interface::<ImageConfigInput>("ImageConfigInput"),
        &typescript_interface::<UpdateImageConfigInput>("UpdateImageConfigInput"),
        &typescript_interface::<LlmConfigSummary>("LlmConfigSummary"),
        &typescript_interface::<CreateLlmInput>("CreateLlmInput"),
        &typescript_interface::<UpdateLlmInput>("UpdateLlmInput"),
        &typescript_interface::<LlmMaxTokensConfig>("LlmMaxTokensConfig"),
        &typescript_llm_constants(),
        &typescript_interface::<VideoConfigSummary>("VideoConfigSummary"),
        &typescript_interface::<VideoConfigInput>("VideoConfigInput"),
        &typescript_interface::<UpdateVideoConfigInput>("UpdateVideoConfigInput"),
        &typescript_interface::<ImportAllConfigsInput>("ImportAllConfigsInput"),
        &typescript_interface::<LlmProviderDescriptor>("LlmProviderDescriptor"),
        &typescript_interface::<ImageProviderDescriptor>("ImageProviderDescriptor"),
        &typescript_interface::<VideoProviderDescriptor>("VideoProviderDescriptor"),
        &typescript_interface::<EmbeddingProviderDescriptor>("EmbeddingProviderDescriptor"),
        &typescript_interface::<ModelProviderCatalog>("ModelProviderCatalog"),
    ]
    .concat();

    let host_output = [
        "// @generated by `cargo run -p a4n-api-contract --bin generate_types -- --write`.\n",
        "// Do not edit this file; change the Rust contract and regenerate it.\n\n",
        "import type { ThemeMode } from \"./theme\";\n\n",
        &typescript_api_commands(),
        &typescript_project_package_suffixes(),
        &typescript_import_constants(),
        &typescript_generation_start_paths(),
        &typescript_runtime_config_policies(),
        &typescript_interface::<SetLogLevelInput>("SetLogLevelInput"),
        &typescript_interface::<ReadLogsInput>("ReadLogsInput"),
        &typescript_interface::<LogReadResult>("LogReadResult"),
        &typescript_interface::<StoragePathsSummary>("StoragePathsSummary"),
        &typescript_interface::<OrphanProjectDataDirSummary>("OrphanProjectDataDirSummary"),
        &typescript_interface::<OrphanProjectDataDirsSummary>("OrphanProjectDataDirsSummary"),
        &typescript_interface::<ImportProjectTextUploadInput>("ImportProjectTextUploadInput"),
        &typescript_chapter_constants(),
        &typescript_interface::<ChapterAnalysisInput>("ChapterAnalysisInput"),
        &typescript_interface::<GenerateChapterSceneInput>("GenerateChapterSceneInput"),
        &typescript_interface::<UpdateChapterSceneInput>("UpdateChapterSceneInput"),
        &typescript_interface::<ReorderChapterScenesInput>("ReorderChapterScenesInput"),
        &typescript_interface::<ImportChapterInput>("ImportChapterInput"),
        &typescript_interface::<UpdateChapterContentInput>("UpdateChapterContentInput"),
        &typescript_enum::<ChapterMaterialSyncStatus>("ChapterMaterialSyncStatus"),
        &typescript_interface::<ChapterMaterialSyncState>("ChapterMaterialSyncState"),
        &typescript_enum::<ChapterMaterialUpdateKind>("ChapterMaterialUpdateKind"),
        &typescript_enum::<ChapterMaterialUpdateAction>("ChapterMaterialUpdateAction"),
        &typescript_interface::<ChapterMaterialUpdate>("ChapterMaterialUpdate"),
        &typescript_interface::<ChapterSummary>("ChapterSummary"),
        &typescript_interface::<ChapterAnalysisMutationResult>("ChapterAnalysisMutationResult"),
        &typescript_interface::<ChapterSceneSummary>("ChapterSceneSummary"),
        &typescript_interface::<ReverseEngineerInput>("ReverseEngineerInput"),
        &typescript_interface::<ConfirmReverseEngineeringCandidateInput>(
            "ConfirmReverseEngineeringCandidateInput",
        ),
        &typescript_interface::<ModelRequestPreview>("ModelRequestPreview"),
        &typescript_interface::<ModelRequestPreviewStep>("ModelRequestPreviewStep"),
        &typescript_interface::<EmbeddingModelIdentity>("EmbeddingModelIdentity"),
        &typescript_interface::<LocalEmbeddingModelSummary>("LocalEmbeddingModelSummary"),
        &typescript_interface::<EmbeddingModelDownloadInfoSummary>(
            "EmbeddingModelDownloadInfoSummary",
        ),
        &typescript_interface::<EmbeddingModelInput>("EmbeddingModelInput"),
        &typescript_interface::<DownloadEmbeddingModelInput>("DownloadEmbeddingModelInput"),
        &typescript_interface::<CancelEmbeddingModelDownloadInput>(
            "CancelEmbeddingModelDownloadInput",
        ),
        &typescript_interface::<LocalEmbeddingConfigInput>("LocalEmbeddingConfigInput"),
        &typescript_interface::<LocalEmbedTextsInput>("LocalEmbedTextsInput"),
        &typescript_interface::<LocalEmbedTextsOutput>("LocalEmbedTextsOutput"),
        &typescript_interface::<InspirationHistoryInput>("InspirationHistoryInput"),
        &typescript_interface::<PreviewInspirationRequestInput>("PreviewInspirationRequestInput"),
        &typescript_interface::<InspirationMessageSummary>("InspirationMessageSummary"),
        &typescript_interface::<PromptEntrySummary>("PromptEntrySummary"),
        &typescript_interface::<PromptVersionSummary>("PromptVersionSummary"),
        &typescript_interface::<AbRenderResult>("AbRenderResult"),
        &typescript_interface::<RenderInput>("RenderInput"),
        &typescript_interface::<UpdatePromptInput>("UpdatePromptInput"),
        &typescript_interface::<SetOverrideInput>("SetOverrideInput"),
        &typescript_interface::<RemoveOverrideInput>("RemoveOverrideInput"),
        &typescript_interface::<VersionDiffInput>("VersionDiffInput"),
        &typescript_interface::<ValidateVarsInput>("ValidateVarsInput"),
        &typescript_interface::<CategoryInfo>("CategoryInfo"),
        &typescript_interface::<ProjectPromptOverrideSummary>("ProjectPromptOverrideSummary"),
        &typescript_interface::<CreateInviteCodeInput>("CreateInviteCodeInput"),
        &typescript_interface::<InviteCodeSummary>("InviteCodeSummary"),
        &typescript_interface::<CreateUserInput>("CreateUserInput"),
        &typescript_interface::<UpdateUserRoleInput>("UpdateUserRoleInput"),
        &typescript_interface::<UsageLogSummary>("UsageLogSummary"),
        &typescript_interface::<TaskQueueSummary>("TaskQueueSummary"),
        &typescript_interface::<AdminProjectSummary>("AdminProjectSummary"),
        &typescript_interface::<UsageTrendsInput>("UsageTrendsInput"),
        &typescript_interface::<DailyUsagePoint>("DailyUsagePoint"),
        &typescript_interface::<ThemeInfo>("ThemeInfo"),
        &typescript_interface::<AdminOverviewSummary>("AdminOverviewSummary"),
        &typescript_interface::<GenerationCallAnalyticsInput>("GenerationCallAnalyticsInput"),
        &typescript_interface::<CallKindOverview>("CallKindOverview"),
        &typescript_interface::<DailyCallMetric>("DailyCallMetric"),
        &typescript_interface::<LlmWorkflowMetric>("LlmWorkflowMetric"),
        &typescript_interface::<MediaWorkflowMetric>("MediaWorkflowMetric"),
        &typescript_interface::<RecentLlmCall>("RecentLlmCall"),
        &typescript_interface::<RecentMediaCall>("RecentMediaCall"),
        &typescript_interface::<GenerationCallAnalytics>("GenerationCallAnalytics"),
        &typescript_interface::<ListImagesByProjectInput>("ListImagesByProjectInput"),
        &typescript_interface::<ListImagesByChapterInput>("ListImagesByChapterInput"),
        &typescript_interface::<ListVideoReferenceImagesInput>("ListVideoReferenceImagesInput"),
        &typescript_interface::<GeneratedImageSummary>("GeneratedImage"),
        &typescript_interface::<UpdateGeneratedImageInput>("UpdateGeneratedImageInput"),
        &typescript_interface::<DeleteImageInput>("DeleteImageInput"),
        &typescript_interface::<UploadImageInput>("UploadImageInput"),
        &typescript_interface::<GenerateSceneImageInput>("GenerateSceneImageInput"),
        &typescript_interface::<GeneratePageImageInput>("GeneratePageImageInput"),
        &typescript_interface::<GenerateBatchImagesInput>("GenerateBatchImagesInput"),
        &typescript_enum::<PortraitType>("PortraitType"),
        &typescript_interface::<PortraitSummary>("PortraitSummary"),
        &typescript_interface::<ListPortraitsByProjectInput>("ListPortraitsByProjectInput"),
        &typescript_interface::<GetPrimaryPortraitInput>("GetPrimaryPortraitInput"),
        &typescript_interface::<SetPrimaryPortraitInput>("SetPrimaryPortraitInput"),
        &typescript_interface::<GeneratePortraitPromptInput>("GeneratePortraitPromptInput"),
        &typescript_interface::<GeneratePortraitInput>("GeneratePortraitInput"),
        &typescript_interface::<RegeneratePortraitInput>("RegeneratePortraitInput"),
        &typescript_interface::<GenerateAgeVariantInput>("GenerateAgeVariantInput"),
        &typescript_interface::<GenerateMissingPortraitsInput>("GenerateMissingPortraitsInput"),
        &typescript_interface::<UploadPortraitInput>("UploadPortraitInput"),
        &typescript_interface::<ExportProjectInput>("ExportProjectInput"),
        &typescript_interface::<ExportPublicationMetadata>("ExportPublicationMetadata"),
        &typescript_interface::<ExportPublicationInput>("ExportPublicationInput"),
        &typescript_enum::<BatchExportFormat>("BatchExportFormat"),
        &typescript_interface::<BatchExportInput>("BatchExportInput"),
        &typescript_interface::<ExportFileData>("ExportFileData"),
        &typescript_interface::<MangaPipelineStartInput>("MangaPipelineStartInput"),
        &typescript_interface::<MangaPipelineResumeInput>("MangaPipelineResumeInput"),
        &typescript_interface::<MangaChapterInput>("MangaChapterInput"),
        &typescript_interface::<BindMangaReferenceImageInput>("BindMangaReferenceImageInput"),
        &typescript_interface::<UpdateMangaPromptInput>("UpdateMangaPromptInput"),
        &typescript_interface::<OptimizeMangaPromptInput>("OptimizeMangaPromptInput"),
        &typescript_interface::<DeleteMangaPromptInput>("DeleteMangaPromptInput"),
        &typescript_interface::<MangaPipelineStatusSummary>("MangaPipelineStatusSummary"),
        &typescript_enum::<MangaPagePlanSchemaVersion>("MangaPagePlanSchemaVersion"),
        &typescript_enum::<MangaPageDrawingPromptSchemaVersion>(
            "MangaPageDrawingPromptSchemaVersion",
        ),
        &typescript_interface::<MangaShootingBeatRefV3>("MangaShootingBeatRefV3"),
        &typescript_enum::<MangaPageRoleV2>("MangaPageRoleV2"),
        &typescript_enum::<MangaPagePacingV2>("MangaPagePacingV2"),
        &typescript_enum::<MangaPageImportanceV2>("MangaPageImportanceV2"),
        &typescript_enum::<MangaReadingFlowV2>("MangaReadingFlowV2"),
        &typescript_enum::<MangaPanelFunctionV2>("MangaPanelFunctionV2"),
        &typescript_enum::<MangaPanelSizeV2>("MangaPanelSizeV2"),
        &typescript_enum::<MangaShotScaleV2>("MangaShotScaleV2"),
        &typescript_enum::<MangaCameraAngleV2>("MangaCameraAngleV2"),
        &typescript_enum::<MangaTextLayerKindV2>("MangaTextLayerKindV2"),
        &typescript_enum::<MangaTextPlacementV2>("MangaTextPlacementV2"),
        &typescript_interface::<MangaTextLayerV2>("MangaTextLayerV2"),
        &typescript_interface::<MangaPanelV2>("MangaPanelV2"),
        &typescript_interface::<MangaPageV3>("MangaPageV3"),
        &typescript_interface::<MangaPageDrawingPromptV2>("MangaPageDrawingPromptV2"),
        &typescript_enum::<MangaReferenceAssetKind>("MangaReferenceAssetKind"),
        &typescript_interface::<MangaBoundReferenceAsset>("MangaBoundReferenceAsset"),
        &typescript_interface::<MangaReferenceBindingSummary>("MangaReferenceBindingSummary"),
        &typescript_interface::<MangaPagePromptSourceSummary>("MangaPagePromptSourceSummary"),
        &typescript_interface::<MangaPagePromptSummary>("MangaPagePromptSummary"),
        &typescript_interface::<GalgameProjectInput>("GalgameProjectInput"),
        &typescript_interface::<StartGalgamePipelineInput>("StartGalgamePipelineInput"),
        &typescript_interface::<PreviewGalgamePipelineInput>("PreviewGalgamePipelineInput"),
        &typescript_enum::<GalgamePipelineStartStage>("GalgamePipelineStartStage"),
        &typescript_interface::<GalgamePipelineStatusSummary>("GalgamePipelineStatusSummary"),
        &typescript_interface::<GalgameWorkflowRunSummary>("GalgameWorkflowRunSummary"),
        &typescript_interface::<GalgameWorkflowCheckpointSummary>(
            "GalgameWorkflowCheckpointSummary",
        ),
        &typescript_interface::<GalgameScriptDetail>("GalgameScriptDetail"),
        &typescript_interface::<GalgameSpeakerProfile>("GalgameSpeakerProfile"),
        &typescript_interface::<GalgameTextAnchorSummary>("GalgameTextAnchorSummary"),
        &typescript_interface::<GalgameSceneDetail>("GalgameSceneDetail"),
        &typescript_interface::<GalgameScene>("GalgameScene"),
        &typescript_enum::<GalgameSceneEntryKind>("GalgameSceneEntryKind"),
        &typescript_interface::<GalgameLine>("GalgameLine"),
        &typescript_enum::<GalgameLineType>("GalgameLineType"),
        &typescript_enum::<GalgameStagePositionV2>("GalgameStagePositionV2"),
        &typescript_interface::<GalgameLinePresentationDocumentV2>(
            "GalgameLinePresentationDocumentV2",
        ),
        &typescript_enum::<GalgameLinePresentationSchemaVersion>(
            "GalgameLinePresentationSchemaVersion",
        ),
        &typescript_interface::<GalgameVisibleCastMemberV2>("GalgameVisibleCastMemberV2"),
        &typescript_enum::<GalgameFacingV2>("GalgameFacingV2"),
        &typescript_enum::<GalgameStageLayerV2>("GalgameStageLayerV2"),
        &typescript_enum::<GalgameCgImportance>("GalgameCgImportance"),
        &typescript_enum::<GalgameTransition>("GalgameTransition"),
        &typescript_interface::<GalgamePipelineBeatRefV3>("GalgamePipelineBeatRefV3"),
        &typescript_interface::<GalgameChoice>("GalgameChoice"),
        &typescript_interface::<GalgameMediaBindingSummary>("GalgameMediaBindingSummary"),
        &typescript_interface::<GalgameMediaBinding>("GalgameMediaBinding"),
        &typescript_enum::<GalgameMediaTargetKind>("GalgameMediaTargetKind"),
        &typescript_enum::<GalgameMediaRole>("GalgameMediaRole"),
        &typescript_interface::<GalgameMediaAsset>("GalgameMediaAsset"),
        &typescript_interface::<GalgameAssetPlan>("GalgameAssetPlan"),
        &typescript_interface::<GalgameAssetPlanSummary>("GalgameAssetPlanSummary"),
        &typescript_interface::<GalgameAssetPlanItem>("GalgameAssetPlanItem"),
        &typescript_enum::<GalgameAssetPlanOutcome>("GalgameAssetPlanOutcome"),
        &typescript_enum::<GalgameGenerationTargetKind>("GalgameGenerationTargetKind"),
        &typescript_enum::<GalgameTargetEntityKind>("GalgameTargetEntityKind"),
        &typescript_interface::<RunGalgameAssetGenerationInput>("RunGalgameAssetGenerationInput"),
        &typescript_interface::<GalgameAssetGenerationItem>("GalgameAssetGenerationItem"),
        &typescript_interface::<GalgameAssetGenerationSummary>("GalgameAssetGenerationSummary"),
        &typescript_interface::<BindGalgameMediaInput>("BindGalgameMediaInput"),
        &typescript_interface::<UnbindGalgameMediaInput>("UnbindGalgameMediaInput"),
        &typescript_interface::<ImportGalgameAudioInput>("ImportGalgameAudioInput"),
        &typescript_enum::<GalgameAudioRole>("GalgameAudioRole"),
        &typescript_interface::<UpdateGalgameLineInput>("UpdateGalgameLineInput"),
        &typescript_interface::<UpdateGalgameSceneInput>("UpdateGalgameSceneInput"),
        &typescript_interface::<InsertGalgameLineInput>("InsertGalgameLineInput"),
        &typescript_interface::<DeleteGalgameLineInput>("DeleteGalgameLineInput"),
        &typescript_interface::<ReorderGalgameLinesInput>("ReorderGalgameLinesInput"),
        &typescript_interface::<UpdateGalgameChoicesInput>("UpdateGalgameChoicesInput"),
        &typescript_interface::<UpdateGalgameChoiceOptionInput>("UpdateGalgameChoiceOptionInput"),
        &typescript_interface::<RagSearchResult>("RagSearchResult"),
        &typescript_interface::<EnhancedSearchInput>("EnhancedSearchInput"),
        &typescript_interface::<RagEnhancedQuerySummary>("RagEnhancedQuerySummary"),
        &typescript_interface::<RagEnhancedSearchOutput>("RagEnhancedSearchOutput"),
        &typescript_interface::<RagPreparedEnhancedSearchOutput>("RagPreparedEnhancedSearchOutput"),
        &typescript_interface::<RagEnhancedSearchWithVectorsInput>(
            "RagEnhancedSearchWithVectorsInput",
        ),
        &typescript_interface::<ExportProjectFamilyDataInput>("ExportProjectFamilyDataInput"),
        &typescript_interface::<ExportDerivedProjectDataInput>("ExportDerivedProjectDataInput"),
        &typescript_interface::<ExportVideoStudioProjectDataInput>(
            "ExportVideoStudioProjectDataInput",
        ),
        &typescript_interface::<ExportProjectPackageData>("ExportProjectPackageData"),
        &typescript_interface::<ImportProjectPackageUploadInput>("ImportProjectPackageUploadInput"),
        &typescript_enum::<ImportReviewStatus>("ImportReviewStatus"),
        &typescript_interface::<ImportReviewSummary>("ImportReviewSummary"),
        &typescript_interface::<ImportProjectReviewsInput>("ImportProjectReviewsInput"),
        &typescript_interface::<ImportReviewIdInput>("ImportReviewIdInput"),
        &typescript_interface::<AcceptImportReviewInput>("AcceptImportReviewInput"),
        &typescript_interface::<BatchAcceptImportReviewsInput>("BatchAcceptImportReviewsInput"),
        &typescript_interface::<DedupResult>("DedupResult"),
        &typescript_interface::<CheckImportDedupInput>("CheckImportDedupInput"),
        &typescript_interface::<CanvasPortContract>("CanvasPortContract"),
        &typescript_interface::<CanvasNodeContract>("CanvasNodeContract"),
        &typescript_interface::<CanvasGraphSummary>("CanvasGraphSummary"),
        &typescript_interface::<CanvasNodeSummary>("CanvasNodeSummary"),
        &typescript_interface::<CanvasEdgeSummary>("CanvasEdgeSummary"),
        &typescript_interface::<CanvasGraphDocument>("CanvasGraphDocument"),
        &typescript_interface::<CanvasNodeInput>("CanvasNodeInput"),
        &typescript_interface::<CanvasEdgeInput>("CanvasEdgeInput"),
        &typescript_interface::<CanvasGraphInput>("CanvasGraphInput"),
        &typescript_interface::<CanvasTextAssetInput>("CanvasTextAssetInput"),
        &typescript_interface::<CanvasTextAssetSummary>("CanvasTextAssetSummary"),
        &typescript_interface::<GetCanvasGraphInput>("GetCanvasGraphInput"),
        &typescript_interface::<GetCanvasNodeContractsInput>("GetCanvasNodeContractsInput"),
        &typescript_interface::<SaveCanvasGraphInput>("SaveCanvasGraphInput"),
        &typescript_interface::<SaveCanvasTextAssetInput>("SaveCanvasTextAssetInput"),
        &typescript_interface::<PreviewCanvasNodeRequestInput>("PreviewCanvasNodeRequestInput"),
        &typescript_interface::<RunCanvasGraphInput>("RunCanvasGraphInput"),
        &typescript_interface::<RunCanvasNodeInput>("RunCanvasNodeInput"),
        &typescript_interface::<ImportCanvasMediaInput>("ImportCanvasMediaInput"),
        &typescript_interface::<ImportedCanvasMedia>("ImportedCanvasMedia"),
        &typescript_interface::<CanvasRunSummary>("CanvasRunSummary"),
        &typescript_interface::<CanvasNodeRequestPreview>("CanvasNodeRequestPreview"),
        &typescript_interface::<OptimizationAgentPermissions>("OptimizationAgentPermissions"),
        &typescript_interface::<OptimizationPatch>("OptimizationPatch"),
        &typescript_interface::<OptimizationSessionSummary>("OptimizationSessionSummary"),
        &typescript_interface::<OptimizationTurnEventSummary>("OptimizationTurnEventSummary"),
        &typescript_interface::<OptimizationTurnSummary>("OptimizationTurnSummary"),
        &typescript_interface::<OptimizationPatchSetSummary>("OptimizationPatchSetSummary"),
        &typescript_interface::<AnalyzeChapterInput>("AnalyzeChapterInput"),
        &typescript_interface::<CreateOptimizationChatSessionInput>(
            "CreateOptimizationChatSessionInput",
        ),
        &typescript_interface::<PrepareOptimizationAnalysisSessionInput>(
            "PrepareOptimizationAnalysisSessionInput",
        ),
        &typescript_interface::<ProposeOptimizationPatchesInput>("ProposeOptimizationPatchesInput"),
        &typescript_interface::<ProposeOptimizationPatchesResponse>(
            "ProposeOptimizationPatchesResponse",
        ),
        &typescript_interface::<ApplyOptimizationPatchesInput>("ApplyOptimizationPatchesInput"),
        &typescript_interface::<ApplyOptimizationPatchesResponse>(
            "ApplyOptimizationPatchesResponse",
        ),
        &typescript_interface::<SendOptimizationMessageInput>("SendOptimizationMessageInput"),
        &typescript_interface::<ProjectProgressSummary>("ProjectProgressSummary"),
        &typescript_enum::<ProjectKind>("ProjectKind"),
        &typescript_project_status(),
        &typescript_interface::<ProjectStatusHistorySummary>("ProjectStatusHistorySummary"),
        &typescript_interface::<ProjectStatusMachineSummary>("ProjectStatusMachineSummary"),
        &typescript_interface::<ProjectRollbackImpactSummary>("ProjectRollbackImpactSummary"),
        &typescript_interface::<ProjectRollbackPreview>("ProjectRollbackPreview"),
        &typescript_interface::<AssetGraphNode>("AssetGraphNode"),
        &typescript_interface::<AssetGraphEdge>("AssetGraphEdge"),
        &typescript_interface::<AssetGraphLayer>("AssetGraphLayer"),
        &typescript_interface::<AssetDependencyGraph>("AssetDependencyGraph"),
        &typescript_interface::<AssetLineageRef>("AssetLineageRef"),
        &typescript_interface::<PromptSource>("PromptSource"),
        &typescript_interface::<GenerationRunSnapshot>("GenerationRunSnapshot"),
        &typescript_interface::<GenerationInputSource>("GenerationInputSource"),
        &typescript_interface::<AssetLineageRecord>("AssetLineageRecord"),
        &typescript_interface::<AssetGraphNodeDetail>("AssetGraphNodeDetail"),
        &typescript_interface::<ProjectSummary>("ProjectSummary"),
        &typescript_interface::<PaginatedProjects>("PaginatedProjects"),
        &typescript_interface::<CreateProjectInput>("CreateProjectInput"),
        &typescript_interface::<CreateFreeformVideoProjectInput>("CreateFreeformVideoProjectInput"),
        &typescript_interface::<CreateDerivedProjectInput>("CreateDerivedProjectInput"),
        &typescript_interface::<DuplicateProjectInput>("DuplicateProjectInput"),
        &typescript_interface::<UpdateProjectInput>("UpdateProjectInput"),
        &typescript_interface::<ProjectGeneratedDataResetSummary>(
            "ProjectGeneratedDataResetSummary",
        ),
        &typescript_enum::<ProjectGeneratedDataResetStatus>("ProjectGeneratedDataResetStatus"),
        &typescript_interface::<ProjectGeneratedDataResetResponse>(
            "ProjectGeneratedDataResetResponse",
        ),
        &typescript_interface::<PreviewBlueprintGenerationInput>("PreviewBlueprintGenerationInput"),
        &typescript_interface::<PreviewBlueprintRefinementInput>("PreviewBlueprintRefinementInput"),
        &typescript_interface::<ListBlueprintVersionsInput>("ListBlueprintVersionsInput"),
        &typescript_interface::<DiffBlueprintVersionsInput>("DiffBlueprintVersionsInput"),
        &typescript_interface::<RestoreBlueprintVersionInput>("RestoreBlueprintVersionInput"),
        &typescript_interface::<SaveBlueprintInput>("SaveBlueprintInput"),
        &typescript_interface::<UpdateWorldSettingInput>("UpdateWorldSettingInput"),
        &typescript_interface::<BlueprintSummary>("BlueprintSummary"),
        &typescript_interface::<BlueprintVersionSummary>("BlueprintVersionSummary"),
        &typescript_interface::<BlueprintDiffSummary>("BlueprintDiffSummary"),
        &typescript_interface::<BlueprintMutationResult>("BlueprintMutationResult"),
        &typescript_interface::<ChapterOutlineSummary>("ChapterOutlineSummary"),
        &typescript_interface::<PartOutlineSummary>("PartOutlineSummary"),
        &typescript_interface::<SaveChapterOutlineInput>("SaveChapterOutlineInput"),
        &typescript_interface::<SaveChapterOutlineResult>("SaveChapterOutlineResult"),
        &typescript_interface::<SavePartOutlineInput>("SavePartOutlineInput"),
        &typescript_interface::<SavePartOutlineResult>("SavePartOutlineResult"),
        &typescript_interface::<OutlineDeleteTarget>("OutlineDeleteTarget"),
        &typescript_interface::<DeleteLatestOutlineRecordsInput>("DeleteLatestOutlineRecordsInput"),
        &typescript_interface::<GeneratePartsInput>("GeneratePartsInput"),
        &typescript_interface::<RegeneratePartsInput>("RegeneratePartsInput"),
        &typescript_interface::<RegenerateLastPartInput>("RegenerateLastPartInput"),
        &typescript_interface::<ContinuePartOutlinesInput>("ContinuePartOutlinesInput"),
        &typescript_interface::<RegenerateChapterOutlineInput>("RegenerateChapterOutlineInput"),
        &typescript_interface::<RegeneratePartOutlineInput>("RegeneratePartOutlineInput"),
        &typescript_interface::<GeneratePartChapterOutlinesInput>(
            "GeneratePartChapterOutlinesInput",
        ),
        &typescript_interface::<BatchGeneratePartOutlinesInput>("BatchGeneratePartOutlinesInput"),
        &typescript_interface::<ApplyBeatMappingInput>("ApplyBeatMappingInput"),
        &typescript_interface::<GenerateChapterOutlinesByCountInput>(
            "GenerateChapterOutlinesByCountInput",
        ),
        &typescript_interface::<UpdateChapterOutlineInput>("UpdateChapterOutlineInput"),
        &typescript_interface::<UpdateChapterOutlineResult>("UpdateChapterOutlineResult"),
        &typescript_interface::<OutlineMutationResult>("OutlineMutationResult"),
        &typescript_interface::<OutlineGenerationProgressSummary>(
            "OutlineGenerationProgressSummary",
        ),
        &typescript_interface::<ChapterOutlinePacingAnalysis>("ChapterOutlinePacingAnalysis"),
        &typescript_interface::<ChapterOutlinePacing>("ChapterOutlinePacing"),
        &typescript_enum::<ProjectAccessRole>("ProjectAccessRole"),
        &typescript_enum::<ProjectAccessScope>("ProjectAccessScope"),
        &typescript_enum::<ProjectMemberRole>("ProjectMemberRole"),
        &typescript_interface::<ProjectMemberSummary>("ProjectMemberSummary"),
        &typescript_interface::<ProjectInviteSummary>("ProjectInviteSummary"),
        &typescript_interface::<InviteProjectMemberInput>("InviteProjectMemberInput"),
        &typescript_interface::<UpdateProjectMemberRoleInput>("UpdateProjectMemberRoleInput"),
        &typescript_enum::<ProgressPhaseKind>("ProgressPhaseKind"),
        &typescript_enum::<ProgressSeverity>("ProgressSeverity"),
        &typescript_interface::<ProgressMeta>("ProgressMeta"),
        &typescript_interface::<TaskProgressSummary>("TaskProgressSummary"),
        &typescript_interface::<TaskInfoSummary>("TaskInfoSummary"),
        &typescript_interface::<CancelTaskInput>("CancelTaskInput"),
        &typescript_interface::<GetTaskHistoryInput>("GetTaskHistoryInput"),
        &typescript_interface::<QueueSlotStatsSummary>("QueueSlotStatsSummary"),
        &typescript_interface::<QueueStatsSummary>("QueueStatsSummary"),
        &typescript_interface::<WorkflowProgressEvent>("WorkflowProgressEvent"),
        &typescript_interface::<WorkflowProgressEventsResponse>("WorkflowProgressEventsResponse"),
        &typescript_interface::<WorkflowStateSummary>("WorkflowStateSummary"),
        &typescript_interface::<WorkflowRunProgressSummary>("WorkflowRunProgressSummary"),
        &typescript_interface::<WorkflowRunSummary>("WorkflowRunSummary"),
        &typescript_interface::<WorkflowCheckpointSummary>("WorkflowCheckpointSummary"),
        &typescript_interface::<GenerationRunSummary>("GenerationRunSummary"),
        &typescript_interface::<WorkflowRunDetailSummary>("WorkflowRunDetailSummary"),
        &typescript_interface::<CancelWorkflowInput>("CancelWorkflowInput"),
        &typescript_interface::<WorkflowProgressEventsInput>("WorkflowProgressEventsInput"),
        &typescript_interface::<ListWorkflowRunsInput>("ListWorkflowRunsInput"),
        &typescript_interface::<GetWorkflowRunInput>("GetWorkflowRunInput"),
        &typescript_interface::<ResumeWorkflowRunInput>("ResumeWorkflowRunInput"),
        &typescript_interface::<CreateProfileInput>("CreateProfileInput"),
        &typescript_interface::<UpdateCharacterProfileInput>("UpdateCharacterProfileInput"),
        &typescript_interface::<ListProfilesInput>("ListProfilesInput"),
        &typescript_character_constants(),
        &typescript_enum::<CharacterAttributeCategory>("CharacterAttributeCategory"),
        &typescript_enum::<CharacterAttributeOperation>("CharacterAttributeOperation"),
        &typescript_enum::<CharacterTimelineSourceKind>("CharacterTimelineSourceKind"),
        &typescript_enum::<DeletionMarkStatus>("DeletionMarkStatus"),
        &typescript_enum::<ImplicitUpdateDecision>("ImplicitUpdateDecision"),
        &typescript_enum::<AppliedImplicitUpdateAction>("AppliedImplicitUpdateAction"),
        &typescript_interface::<AttributeCountsSummary>("AttributeCountsSummary"),
        &typescript_interface::<CharacterProfileSummary>("CharacterProfileSummary"),
        &typescript_interface::<CharacterProfileCreateResult>("CharacterProfileCreateResult"),
        &typescript_interface::<CharacterProfileMutationResult>("CharacterProfileMutationResult"),
        &typescript_interface::<CharacterProfileDeleteResult>("CharacterProfileDeleteResult"),
        &typescript_interface::<SyncCharacterProfileInput>("SyncCharacterProfileInput"),
        &typescript_interface::<SyncAllCharacterProfilesInput>("SyncAllCharacterProfilesInput"),
        &typescript_enum::<CharacterProfileSyncSkipReason>("CharacterProfileSyncSkipReason"),
        &typescript_interface::<CharacterProfileSyncRange>("CharacterProfileSyncRange"),
        &typescript_interface::<CharacterProfileProtectedAttribute>(
            "CharacterProfileProtectedAttribute",
        ),
        &typescript_interface::<CharacterProfileSyncedResult>("CharacterProfileSyncedResult"),
        &typescript_interface::<CharacterProfileSkippedResult>("CharacterProfileSkippedResult"),
        &typescript_type::<SyncCharacterProfileResult>("SyncCharacterProfileResult"),
        &typescript_interface::<CharacterProfileSyncFailure>("CharacterProfileSyncFailure"),
        &typescript_enum::<CharacterProfileSyncBatchStatus>("CharacterProfileSyncBatchStatus"),
        &typescript_interface::<SyncAllCharacterProfilesResult>("SyncAllCharacterProfilesResult"),
        &typescript_interface::<AddProfileAttributeInput>("AddProfileAttributeInput"),
        &typescript_interface::<ModifyProfileAttributeInput>("ModifyProfileAttributeInput"),
        &typescript_interface::<AttributeHistoryInput>("AttributeHistoryInput"),
        &typescript_interface::<AttributeEventSummary>("AttributeEventSummary"),
        &typescript_interface::<ListBehaviorsInput>("ListBehaviorsInput"),
        &typescript_interface::<BehaviorRecordSummary>("BehaviorRecordSummary"),
        &typescript_interface::<GetImplicitReviewInput>("GetImplicitReviewInput"),
        &typescript_interface::<CheckImplicitUpdateInput>("CheckImplicitUpdateInput"),
        &typescript_interface::<ImplicitEvidenceSummary>("ImplicitEvidenceSummary"),
        &typescript_interface::<ImplicitAttributeReviewSummary>("ImplicitAttributeReviewSummary"),
        &typescript_interface::<ImplicitReviewSummary>("ImplicitReviewSummary"),
        &typescript_interface::<ImplicitUpdateDecisionSummary>("ImplicitUpdateDecisionSummary"),
        &typescript_interface::<ApplyImplicitUpdateDecisionInput>(
            "ApplyImplicitUpdateDecisionInput",
        ),
        &typescript_interface::<ApplyImplicitUpdateDecisionResultSummary>(
            "ApplyImplicitUpdateDecisionResultSummary",
        ),
        &typescript_interface::<ListSnapshotsInput>("ListSnapshotsInput"),
        &typescript_interface::<SnapshotReferenceSummary>("SnapshotReferenceSummary"),
        &typescript_interface::<GetProfileStateAtChapterInput>("GetProfileStateAtChapterInput"),
        &typescript_interface::<ProfileChapterStateSummary>("ProfileChapterStateSummary"),
        &typescript_interface::<GetSnapshotInput>("GetSnapshotInput"),
        &typescript_interface::<SnapshotSummary>("SnapshotSummary"),
        &typescript_interface::<DiffProfileChaptersInput>("DiffProfileChaptersInput"),
        &typescript_interface::<SnapshotDeltaSummary>("SnapshotDeltaSummary"),
        &typescript_interface::<CheckProfileSyncConflictInput>("CheckProfileSyncConflictInput"),
        &typescript_interface::<ProfileSyncConflictSummary>("ProfileSyncConflictSummary"),
        &typescript_interface::<RollbackToChapterInput>("RollbackToChapterInput"),
        &typescript_interface::<RollbackResultSummary>("RollbackResultSummary"),
        &typescript_interface::<ListDeletionMarksInput>("ListDeletionMarksInput"),
        &typescript_interface::<DeletionMarkSummary>("DeletionMarkSummary"),
        &typescript_interface::<RequestAttributeDeletionInput>("RequestAttributeDeletionInput"),
        &typescript_interface::<ExecuteAttributeDeletionInput>("ExecuteAttributeDeletionInput"),
        &typescript_interface::<ResetDeletionMarksInput>("ResetDeletionMarksInput"),
        &typescript_interface::<ListObjectProfilesInput>("ListObjectProfilesInput"),
        &typescript_interface::<ObjectProfileInput>("ObjectProfileInput"),
        &typescript_interface::<CreateObjectProfileInput>("CreateObjectProfileInput"),
        &typescript_interface::<UpdateObjectProfileInput>("UpdateObjectProfileInput"),
        &typescript_interface::<DeleteObjectProfileInput>("DeleteObjectProfileInput"),
        &typescript_interface::<ObjectProfileSummary>("ObjectProfileSummary"),
        &typescript_interface::<ObjectProfileMutationResult>("ObjectProfileMutationResult"),
        &typescript_interface::<ObjectProfileDeleteResult>("ObjectProfileDeleteResult"),
        &typescript_interface::<ListSceneProfilesInput>("ListSceneProfilesInput"),
        &typescript_interface::<SyncSceneProfilesInput>("SyncSceneProfilesInput"),
        &typescript_interface::<SceneProfileInput>("SceneProfileInput"),
        &typescript_interface::<CreateSceneProfileInput>("CreateSceneProfileInput"),
        &typescript_interface::<UpdateSceneProfileInput>("UpdateSceneProfileInput"),
        &typescript_interface::<DeleteSceneProfileInput>("DeleteSceneProfileInput"),
        &typescript_interface::<SceneProfileSummary>("SceneProfileSummary"),
        &typescript_interface::<SceneProfileMutationResult>("SceneProfileMutationResult"),
        &typescript_interface::<SceneProfileDeleteResult>("SceneProfileDeleteResult"),
        &typescript_interface::<ObjectProfileImageInput>("ObjectProfileImageInput"),
        &typescript_interface::<ObjectProfileImagePromptOutput>("ObjectProfileImagePromptOutput"),
        &typescript_interface::<SceneProfileImageInput>("SceneProfileImageInput"),
        &typescript_interface::<SceneProfileImagePromptOutput>("SceneProfileImagePromptOutput"),
        &typescript_enum::<ProfileMaterialKind>("ProfileMaterialKind"),
        &typescript_enum::<ProfileMaterialKindFilter>("ProfileMaterialKindFilter"),
        &typescript_enum::<ProfileMaterialState>("ProfileMaterialState"),
        &typescript_enum::<ProfileMaterialImageStaleness>("ProfileMaterialImageStaleness"),
        &typescript_enum::<ProfileMaterialFilter>("ProfileMaterialFilter"),
        &typescript_enum::<ProfileMaterialSort>("ProfileMaterialSort"),
        &typescript_interface::<ListProfileMaterialsInput>("ListProfileMaterialsInput"),
        &typescript_interface::<SetProfilePrimaryMaterialInput>("SetProfilePrimaryMaterialInput"),
        &typescript_interface::<UploadProfileMaterialImageInput>("UploadProfileMaterialImageInput"),
        &typescript_interface::<ProfileMaterialProfileSummary>("ProfileMaterialProfileSummary"),
        &typescript_interface::<ProfileMaterialImageSummary>("ProfileMaterialImageSummary"),
        &typescript_interface::<ProfileMaterialGroup>("ProfileMaterialGroup"),
        &typescript_interface::<ListForeshadowingInput>("ListForeshadowingInput"),
        &typescript_enum::<ForeshadowingStatus>("ForeshadowingStatus"),
        &typescript_interface::<ForeshadowingRecordInput>("ForeshadowingRecordInput"),
        &typescript_interface::<CreateForeshadowingInput>("CreateForeshadowingInput"),
        &typescript_interface::<UpdateForeshadowingInput>("UpdateForeshadowingInput"),
        &typescript_interface::<DeleteForeshadowingInput>("DeleteForeshadowingInput"),
        &typescript_interface::<ForeshadowingSummary>("ForeshadowingSummary"),
        &typescript_interface::<ForeshadowingWriteResult>("ForeshadowingWriteResult"),
        &typescript_enum::<ChapterShootingScriptSchemaVersion>(
            "ChapterShootingScriptSchemaVersion",
        ),
        &typescript_enum::<ShootingEditingModeV1>("ShootingEditingModeV1"),
        &typescript_enum::<ShootingViewpointModeV1>("ShootingViewpointModeV1"),
        &typescript_enum::<ShootingContinuityIntentV1>("ShootingContinuityIntentV1"),
        &typescript_enum::<ShootingCoverageV1>("ShootingCoverageV1"),
        &typescript_enum::<ShootingShotSizeV1>("ShootingShotSizeV1"),
        &typescript_enum::<ShootingCameraAngleV1>("ShootingCameraAngleV1"),
        &typescript_enum::<ShootingCameraHeightV1>("ShootingCameraHeightV1"),
        &typescript_enum::<ShootingCameraMovementV1>("ShootingCameraMovementV1"),
        &typescript_interface::<ShootingDirectingStatementV1>("ShootingDirectingStatementV1"),
        &typescript_interface::<ShootingTransitionContractV1>("ShootingTransitionContractV1"),
        &typescript_interface::<ShootingViewpointV1>("ShootingViewpointV1"),
        &typescript_interface::<ShootingSpatialContinuityV1>("ShootingSpatialContinuityV1"),
        &typescript_interface::<ShootingBlockingV1>("ShootingBlockingV1"),
        &typescript_interface::<ShootingPerformanceBeatV1>("ShootingPerformanceBeatV1"),
        &typescript_interface::<ShootingDialogueCueV1>("ShootingDialogueCueV1"),
        &typescript_interface::<ShootingSoundDesignV1>("ShootingSoundDesignV1"),
        &typescript_interface::<ShootingShotV1>("ShootingShotV1"),
        &typescript_interface::<ShootingSceneV1>("ShootingSceneV1"),
        &typescript_interface::<ChapterShootingScriptV1>("ChapterShootingScriptV1"),
        &typescript_interface::<ChapterShootingScriptSourceInput>(
            "ChapterShootingScriptSourceInput",
        ),
        &typescript_interface::<EditChapterShootingScriptInput>("EditChapterShootingScriptInput"),
        &typescript_interface::<ChapterShootingScriptRecordSummary>(
            "ChapterShootingScriptRecordSummary",
        ),
        &typescript_interface::<ChapterShootingScriptView>("ChapterShootingScriptView"),
        &typescript_interface::<ChapterShootingScriptUnitProgressView>(
            "ChapterShootingScriptUnitProgressView",
        ),
        &typescript_interface::<EditedChapterShootingScriptView>("EditedChapterShootingScriptView"),
        &typescript_interface::<SourceSyncDecisionInput>("SourceSyncDecisionInput"),
        &typescript_interface::<ApplySourceSyncInput>("ApplySourceSyncInput"),
        &typescript_interface::<SourceSyncRunSummary>("SourceSyncRunSummary"),
        &typescript_interface::<SourceSyncConflictSummary>("SourceSyncConflictSummary"),
        &typescript_interface::<SourceSyncPlanSummary>("SourceSyncPlanSummary"),
        &typescript_interface::<GeneratedVideoSummary>("GeneratedVideoSummary"),
        &typescript_interface::<UpdateGeneratedVideoInput>("UpdateGeneratedVideoInput"),
        &typescript_enum::<DownstreamConflictStrategy>("DownstreamConflictStrategy"),
        &typescript_interface::<ChapterVideoPromptSummary>("ChapterVideoPrompt"),
        &typescript_interface::<UpdateChapterVideoPromptInput>("UpdateChapterVideoPromptInput"),
        &typescript_interface::<OptimizeChapterVideoPromptInput>("OptimizeChapterVideoPromptInput"),
        &typescript_interface::<DeleteChapterVideoPromptInput>("DeleteChapterVideoPromptInput"),
        &typescript_interface::<PreviewChapterVideoPromptOverrideInput>(
            "PreviewChapterVideoPromptOverrideInput",
        ),
        &typescript_interface::<GenerateChapterVideosBatchResult>(
            "GenerateChapterVideosBatchResult",
        ),
        &typescript_interface::<VideoPlaybackData>("VideoPlaybackData"),
        &typescript_interface::<VideoScrubFramesData>("VideoScrubFramesData"),
        &typescript_interface::<GetVideoTimelineInput>("GetVideoTimelineInput"),
        &typescript_interface::<CommitInsertedSegmentDraftInput>("CommitInsertedSegmentDraftInput"),
        &typescript_interface::<MoveInsertedSegmentInput>("MoveInsertedSegmentInput"),
        &typescript_interface::<DeleteInsertedSegmentInput>("DeleteInsertedSegmentInput"),
        &typescript_enum::<VideoTimelineItemKind>("VideoTimelineItemKind"),
        &typescript_enum::<VideoTimelineItemStatus>("VideoTimelineItemStatus"),
        &typescript_interface::<VideoTimelineItemSummary>("VideoTimelineItemSummary"),
        &typescript_interface::<VideoTimelineSummary>("VideoTimelineSummary"),
        &typescript_interface::<AssembledChapterVideoClipSummary>(
            "AssembledChapterVideoClipSummary",
        ),
        &typescript_interface::<AssembledChapterVideoPreview>("AssembledChapterVideoPreview"),
        &typescript_interface::<AssemblyPushResult>("AssemblyPushResult"),
        &typescript_interface::<PushAssembledChapterVideoInput>("PushAssembledChapterVideoInput"),
        &typescript_enum::<AssemblyPushAction>("AssemblyPushAction"),
        &typescript_interface::<UpdateAssemblyPushStatusInput>("UpdateAssemblyPushStatusInput"),
        &typescript_enum::<AssemblyPostprocessOperation>("AssemblyPostprocessOperation"),
        &typescript_interface::<ApplyAssemblyPostprocessEditInput>(
            "ApplyAssemblyPostprocessEditInput",
        ),
        &typescript_interface::<UploadChapterVideoInput>("UploadChapterVideoInput"),
        &typescript_interface::<VideoSegmentTarget>("VideoSegmentTarget"),
        &typescript_interface::<UploadReferenceMediaVideoInput>("UploadReferenceMediaVideoInput"),
        &typescript_interface::<UploadAssemblyInsertVideoInput>("UploadAssemblyInsertVideoInput"),
        &typescript_interface::<ImportCanvasVideoInput>("ImportCanvasVideoInput"),
    ]
    .concat();
    let host_output = format!("{}\n", host_output.trim_end());

    match env::args().nth(1).as_deref() {
        Some("--write") => {
            write_generated("theme.ts", &theme_output);
            write_generated("auth.ts", &auth_output);
            write_generated("config.ts", &config_output);
            write_generated("host.ts", &host_output);
        }
        Some("--check") => {
            check_generated("theme.ts", &theme_output);
            check_generated("auth.ts", &auth_output);
            check_generated("config.ts", &config_output);
            check_generated("host.ts", &host_output);
        }
        _ => print!("{theme_output}{auth_output}{config_output}{host_output}"),
    }
}

fn generated_path(file_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../frontend/src/api/generated")
        .join(file_name)
}

fn write_generated(file_name: &str, output: &str) {
    let path = generated_path(file_name);
    fs::write(&path, output).unwrap_or_else(|error| {
        panic!("write {}: {error}", path.display());
    });
}

fn check_generated(file_name: &str, output: &str) {
    let path = generated_path(file_name);
    let existing = fs::read_to_string(&path).unwrap_or_else(|error| {
        panic!("read {}: {error}", path.display());
    });
    assert_eq!(
        existing, output,
        "generated TypeScript is stale; run --write"
    );
}

fn typescript_interface<T: JsonSchema>(name: &str) -> String {
    let root = serde_json::to_value(schemars::schema_for!(T)).expect("serialize JSON schema");
    let schema = resolve_root(&root, &root);
    let properties = schema.get("properties").and_then(Value::as_object);
    let required = schema
        .get("required")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .collect::<std::collections::BTreeSet<_>>()
        })
        .unwrap_or_default();
    let force_nullable_fields_required = !name.ends_with("Input");

    let mut output = format!("export interface {name} {{\n");
    if let Some(properties) = properties {
        for (field, property) in properties {
            let optional = !force_nullable_fields_required
                && !required.contains(field.as_str())
                && property.get("x-a4n-required").and_then(Value::as_bool) != Some(true);
            output.push_str(&format!(
                "  {field}{}: {};\n",
                if optional { "?" } else { "" },
                ts_type(property, &root)
            ));
        }
    }
    output.push_str("}\n\n");
    output
}

fn typescript_type<T: JsonSchema>(name: &str) -> String {
    let root = serde_json::to_value(schemars::schema_for!(T)).expect("serialize JSON schema");
    format!("export type {name} = {};\n\n", ts_type(&root, &root))
}

fn typescript_enum<T: JsonSchema>(name: &str) -> String {
    let root = serde_json::to_value(schemars::schema_for!(T)).expect("serialize JSON schema");
    let schema = resolve_root(&root, &root);
    let values = schema
        .get("enum")
        .and_then(Value::as_array)
        .expect("contract enum schema must contain enum values");
    let variants = values
        .iter()
        .map(|value| match value {
            Value::String(value) => format!("{value:?}"),
            other => other.to_string(),
        })
        .collect::<Vec<_>>();
    format!("export type {name} = {};\n\n", variants.join(" | "))
}

fn typescript_project_status() -> String {
    let values = ProjectStatus::ALL
        .iter()
        .map(|status| format!("{:?}", status.as_str()))
        .collect::<Vec<_>>()
        .join(", ");
    let labels = ProjectStatus::ALL
        .iter()
        .map(|status| format!("  {:?}: {:?},", status.as_str(), status.to_string()))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "export const PROJECT_STATUSES = [{values}] as const;\nexport type ProjectStatus = (typeof PROJECT_STATUSES)[number];\nexport const PROJECT_STATUS_LABELS: Record<ProjectStatus, string> = {{\n{labels}\n}};\n\n"
    )
}

fn typescript_llm_constants() -> String {
    let defaults = LlmMaxTokensConfig::default()
        .profile_values()
        .into_iter()
        .map(|(field, value)| format!("  {field}: {value},"))
        .collect::<Vec<_>>()
        .join("\n");
    let reserved = LLM_RESERVED_REQUEST_OPTION_KEYS
        .iter()
        .map(|key| format!("{key:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "export const DEFAULT_LLM_MAX_CONTEXT_TOKENS = {DEFAULT_LLM_MAX_CONTEXT_TOKENS};\n\
         export const DEFAULT_LLM_MAX_OUTPUT_TOKENS = {DEFAULT_LLM_MAX_OUTPUT_TOKENS};\n\
         export const LLM_MAX_TOKEN_LIMIT = {LLM_MAX_TOKEN_LIMIT};\n\
         export const DEFAULT_LLM_MAX_TOKENS_CONFIG: LlmMaxTokensConfig = {{\n{defaults}\n}};\n\
         export const LLM_RESERVED_REQUEST_OPTION_KEYS: readonly string[] = [{reserved}];\n\n"
    )
}

fn typescript_project_package_suffixes() -> String {
    format!(
        "export const PROJECT_FAMILY_PACKAGE_SUFFIX = {PROJECT_FAMILY_PACKAGE_SUFFIX:?};\n\
         export const VIDEO_STUDIO_PROJECT_PACKAGE_SUFFIX = {VIDEO_STUDIO_PROJECT_PACKAGE_SUFFIX:?};\n\n"
    )
}

fn typescript_import_constants() -> String {
    format!("export const MAX_PROJECT_TEXT_UPLOAD_BYTES = {MAX_PROJECT_TEXT_UPLOAD_BYTES};\n\n")
}

fn typescript_chapter_constants() -> String {
    format!(
        "export const CHAPTER_ANALYSIS_PROGRESS_TOTAL = {CHAPTER_ANALYSIS_PROGRESS_TOTAL};\n\
         export const CHAPTER_SCENE_PROGRESS_TOTAL = {CHAPTER_SCENE_PROGRESS_TOTAL};\n\n"
    )
}

fn typescript_character_constants() -> String {
    format!("export const CHARACTER_DELETION_THRESHOLD = {CHARACTER_DELETION_THRESHOLD};\n\n")
}

fn typescript_runtime_config_policies() -> String {
    let mut output = String::from(
        "export type RuntimeConfigPolicy = \"full\" | \"without_secrets\";\n\n\
         export const RUNTIME_CONFIG_POLICIES: Readonly<Record<string, RuntimeConfigPolicy>> = {\n",
    );
    for command in API_COMMANDS {
        let policy = match command_runtime_config_policy(command) {
            RuntimeConfigPolicy::None => continue,
            RuntimeConfigPolicy::WithoutSecrets => "without_secrets",
            RuntimeConfigPolicy::Full => "full",
        };
        output.push_str(&format!("  {command:?}: {policy:?},\n"));
    }
    output.push_str("};\n\n");
    output
}

fn typescript_api_commands() -> String {
    let commands = API_COMMANDS
        .iter()
        .map(|command| format!("  {command:?},"))
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "export const HOST_API_COMMANDS = [\n{commands}\n] as const;\n\n\
         export type HostApiCommand = (typeof HOST_API_COMMANDS)[number];\n\n"
    )
}

fn typescript_generation_start_paths() -> String {
    let mut output = String::from(
        "export const GENERATION_START_PATHS: ReadonlyMap<string, string> = new Map<string, string>([\n",
    );
    for (command, path) in GENERATION_START_ROUTES {
        output.push_str(&format!("  [{command:?}, {path:?}],\n"));
    }
    output.push_str("]);\n\n");
    output
}

fn resolve_root<'a>(schema: &'a Value, root: &'a Value) -> &'a Value {
    let Some(reference) = schema.get("$ref").and_then(Value::as_str) else {
        return schema;
    };
    let Some(name) = reference.strip_prefix("#/$defs/") else {
        return schema;
    };
    root.get("$defs")
        .and_then(Value::as_object)
        .and_then(|definitions| definitions.get(name))
        .unwrap_or(schema)
}

fn ts_type(schema: &Value, root: &Value) -> String {
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str)
        && let Some(name) = reference.strip_prefix("#/$defs/")
    {
        let name = if name == "GeneratedImageSummary" {
            "GeneratedImage"
        } else {
            name
        };
        return if schema.get("properties").is_some() {
            format!("{name} & {}", ts_object_type(schema, root))
        } else {
            name.to_string()
        };
    }
    let schema = resolve_root(schema, root);
    if let Some(value) = schema.get("const") {
        return value.to_string();
    }
    if let Some(types) = schema.get("type").and_then(Value::as_array) {
        return types
            .iter()
            .filter_map(Value::as_str)
            .map(|kind| match kind {
                "array" => ts_array_type(schema, root),
                "object" => ts_object_type(schema, root),
                _ => primitive_type(kind),
            })
            .collect::<Vec<_>>()
            .join(" | ");
    }
    if let Some(any_of) = schema.get("anyOf").and_then(Value::as_array) {
        return any_of
            .iter()
            .map(|item| ts_type(item, root))
            .collect::<Vec<_>>()
            .into_iter()
            .fold(Vec::<String>::new(), |mut types, item| {
                if !types.contains(&item) {
                    types.push(item);
                }
                types
            })
            .join(" | ");
    }
    if let Some(one_of) = schema.get("oneOf").and_then(Value::as_array) {
        return one_of
            .iter()
            .map(|item| ts_type(item, root))
            .collect::<Vec<_>>()
            .join(" | ");
    }
    if let Some(all_of) = schema.get("allOf").and_then(Value::as_array) {
        return all_of
            .iter()
            .map(|item| ts_type(item, root))
            .collect::<Vec<_>>()
            .join(" & ");
    }
    if let Some(values) = schema.get("enum").and_then(Value::as_array) {
        return values
            .iter()
            .map(|value| match value {
                Value::String(value) => format!("{value:?}"),
                other => other.to_string(),
            })
            .collect::<Vec<_>>()
            .join(" | ");
    }
    if schema
        .get("additionalProperties")
        .is_some_and(Value::is_object)
    {
        return ts_object_type(schema, root);
    }
    match schema.get("type").and_then(Value::as_str) {
        Some("array") => ts_array_type(schema, root),
        Some("object") => ts_object_type(schema, root),
        Some(kind) => primitive_type(kind),
        None => "unknown".to_string(),
    }
}

fn ts_array_type(schema: &Value, root: &Value) -> String {
    if let Some(items) = schema.get("prefixItems").and_then(Value::as_array) {
        return format!(
            "[{}]",
            items
                .iter()
                .map(|item| ts_type(item, root))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    schema
        .get("items")
        .map(|items| format!("{}[]", ts_type(items, root)))
        .unwrap_or_else(|| "unknown[]".to_string())
}

fn ts_object_type(schema: &Value, root: &Value) -> String {
    if let Some(properties) = schema.get("properties").and_then(Value::as_object) {
        let required = schema
            .get("required")
            .and_then(Value::as_array)
            .map(|values| {
                values
                    .iter()
                    .filter_map(Value::as_str)
                    .collect::<std::collections::BTreeSet<_>>()
            })
            .unwrap_or_default();
        let fields = properties
            .iter()
            .map(|(field, property)| {
                format!(
                    "{field}{}: {}",
                    if required.contains(field.as_str()) {
                        ""
                    } else {
                        "?"
                    },
                    ts_type(property, root)
                )
            })
            .collect::<Vec<_>>()
            .join("; ");
        return format!("{{ {fields} }}");
    }
    schema
        .get("additionalProperties")
        .filter(|value| value.is_object())
        .map(|value| format!("Record<string, {}>", ts_type(value, root)))
        .unwrap_or_else(|| "Record<string, unknown>".to_string())
}

fn primitive_type(kind: &str) -> String {
    match kind {
        "string" => "string".to_string(),
        "integer" | "number" => "number".to_string(),
        "boolean" => "boolean".to_string(),
        "null" => "null".to_string(),
        _ => "unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use a4n_api_contract::canvas::GetCanvasNodeContractsInput;
    use a4n_api_contract::chapter::ReorderChapterScenesInput;
    use a4n_api_contract::character::SyncCharacterProfileResult;
    use a4n_api_contract::profile::ProfileMaterialImageSummary;
    use a4n_api_contract::rag::EnhancedSearchInput;

    use super::{typescript_interface, typescript_type};

    #[test]
    fn nullable_array_keeps_its_item_type() {
        let output = typescript_interface::<EnhancedSearchInput>("EnhancedSearchInput");

        assert!(output.contains("data_types?: string[] | null;"), "{output}");
    }

    #[test]
    fn empty_input_generates_an_empty_interface() {
        assert_eq!(
            typescript_interface::<GetCanvasNodeContractsInput>("GetCanvasNodeContractsInput"),
            "export interface GetCanvasNodeContractsInput {\n}\n\n"
        );
    }

    #[test]
    fn tuple_array_keeps_its_item_positions() {
        let output = typescript_interface::<ReorderChapterScenesInput>("ReorderChapterScenesInput");

        assert!(output.contains("order: [number, number][];"), "{output}");
    }

    #[test]
    fn tagged_result_keeps_its_status_discriminator() {
        let output = typescript_type::<SyncCharacterProfileResult>("SyncCharacterProfileResult");

        assert!(
            output.contains("CharacterProfileSyncedResult & { status: \"synced\" }")
                && output.contains("CharacterProfileSkippedResult & { status: \"skipped\" }"),
            "{output}"
        );
    }

    #[test]
    fn generated_image_references_use_the_frontend_canonical_name() {
        let output =
            typescript_interface::<ProfileMaterialImageSummary>("ProfileMaterialImageSummary");

        assert!(output.contains("image: GeneratedImage;"), "{output}");
        assert!(!output.contains("GeneratedImageSummary"), "{output}");
    }
}
