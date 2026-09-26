use a4n_api_contract::command_manifest::{
    API_COMMANDS, AgentCommandPolicy, CommandDescriptor, RuntimeConfigPolicy,
    command_runtime_config_policy,
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
    agent::{
        AGENT_API_PROTOCOL_VERSION, AGENT_EVENTS_DEFAULT_LIMIT, AGENT_EVENTS_MAX_LIMIT,
        AGENT_SKILLS_DEFAULT_LIMIT, AGENT_SKILLS_MAX_LIMIT, AGENT_THREADS_DEFAULT_LIMIT,
        AGENT_THREADS_MAX_LIMIT, AGENT_TOOLS_DEFAULT_LIMIT, AGENT_TOOLS_MAX_LIMIT,
        AgentApprovalDecision, AgentApprovalStatus, AgentApprovalView, AgentBudget,
        AgentControlAction, AgentDelegationReviewDecision, AgentDelegationStatus,
        AgentDelegationView, AgentDeliveryMode, AgentEditorDraftView, AgentEditorOperation,
        AgentEditorReply, AgentEditorRequest, AgentEditorResponse, AgentEditorSavedView,
        AgentEditorSource, AgentEditorState, AgentEditorTarget, AgentEvent, AgentEventsInput,
        AgentExecutionKind, AgentExecutionMode, AgentExecutionResultPage, AgentExecutionStatus,
        AgentExecutionView, AgentInboxItemView, AgentInboxKind, AgentInboxStatus, AgentMemoryInput,
        AgentMemoryScopeKind, AgentMemorySearchResult, AgentMemoryStatus, AgentMemoryView,
        AgentMessageAccepted, AgentMessageRole, AgentMessageStatus, AgentMessageView,
        AgentResourceRef, AgentRunInput, AgentRunStatus, AgentRunView, AgentScope,
        AgentSkillContent, AgentSkillDescriptor, AgentSkillProfileView, AgentSkillSearchResult,
        AgentSkillSettingsView, AgentThreadKind, AgentThreadSnapshot, AgentThreadStatus,
        AgentThreadView, AgentToolCostClass, AgentToolDescriptor, AgentToolExposure,
        AgentToolReplayPolicy, AgentToolReversibility, AgentToolRisk, AgentToolSearchResult,
        AgentToolSummary, AgentTurnStatus, AgentTurnTrigger, AgentTurnView, AgentWaitingReason,
        ArchiveAgentThreadInput, ControlAgentRunInput, ControlChildAgentInput,
        CreateAgentThreadInput, DescribeAgentToolInput, EnsureMainAgentThreadInput,
        ForkAgentThreadInput, ListAgentMessagesInput, ListAgentThreadsInput, LoadAgentSkillInput,
        LocateWritingTargetInput, ReadAgentExecutionResultInput, ReadAgentHistoryInput,
        ReadAgentMemoryInput, ReadEditorDraftInput, RequestAgentInput, ResolveAgentApprovalInput,
        RespondAgentEditorInput, RetractAgentMemoryInput, ReviewAgentDelegationInput,
        SaveEditorDraftInput, SearchAgentSkillsInput, SearchAgentToolsInput, SendAgentMessageInput,
        SendAgentMessageToChildInput, SpawnAgentInput, UpdateAgentSkillConfigurationInput,
        UpdateAgentThreadInput, WritingTargetView,
    },
    auth::{
        AuthResult, ChangePasswordInput, LoginInput, OAuth2AuthorizeInput, OAuth2BrowserInput,
        OAuth2CallbackInput, RefreshTokenInput, RegisterInput, RevokeSessionInput, SessionSummary,
        TokenPairSummary, UpdateProfileInput, UserSummary,
    },
    blueprint::{
        BlueprintDiffSummary, BlueprintMutationResult, BlueprintSummary, BlueprintVersionSummary,
        DiffBlueprintVersionsInput, ListBlueprintVersionsInput, PreviewBlueprintGenerationInput,
        PreviewBlueprintRefinementInput, RestoreBlueprintVersionInput, SaveBlueprintInput,
        UpdateWorldSettingInput,
    },
    canvas::{
        CanvasEdgeInput, CanvasEdgeSummary, CanvasGraphDocument, CanvasGraphEditOperation,
        CanvasGraphEditResult, CanvasGraphInput, CanvasGraphProjection, CanvasGraphReadResult,
        CanvasGraphSaveResult, CanvasGraphSummary, CanvasMediaAssetSummary, CanvasNodeContract,
        CanvasNodeIndex, CanvasNodeInput, CanvasNodeSummary, CanvasPortContract, CanvasRunSummary,
        CanvasTextAssetInput, CanvasTextAssetSummary, EditCanvasGraphInput, GetCanvasGraphInput,
        GetCanvasNodeContractsInput, ImportCanvasMediaInput, ImportedCanvasMedia,
        PreviewCanvasNodeRequestInput, ReadCanvasGraphInput, RunCanvasGraphInput,
        RunCanvasNodeInput, SaveCanvasGraphInput, SaveCanvasTextAssetInput,
    },
    chapter::{
        CHAPTER_ANALYSIS_PROGRESS_TOTAL, CHAPTER_SCENE_PROGRESS_TOTAL, ChapterAnalysisInput,
        ChapterAnalysisMutationResult, ChapterMaterialSyncState, ChapterMaterialSyncStatus,
        ChapterMaterialUpdate, ChapterMaterialUpdateAction, ChapterMaterialUpdateKind,
        ChapterSceneSummary, ChapterSummary, GenerateChapterSceneInput, ImportChapterInput,
        PreparedGenerationRagEmbeddingRequest, ReorderChapterScenesInput,
        UpdateChapterContentInput, UpdateChapterSceneInput,
    },
    chapter_visual_plan::{
        ChapterVisualPlanAuthoringMode, ChapterVisualPlanEditResult, ChapterVisualPlanReadResult,
        ChapterVisualPlanRecordInput, ChapterVisualPlanRecordSummary,
        ChapterVisualPlanSchemaVersion, ChapterVisualPlanSetView, ChapterVisualPlanSourceInput,
        ChapterVisualPlanSourceView, ChapterVisualPlanStatus, ChapterVisualPlanV3,
        ChapterVisualPlanValidationResult, ChapterVisualPlanView, ConfirmedChapterVisualPlanView,
        EditChapterVisualPlanDraftInput, PreviewVisualPlanPointInput, ReadChapterVisualPlanInput,
        ReadVisualPlanPromptCandidateInput, SaveChapterVisualPlanDraftInput, SceneBindingV3,
        StartVisualPlanPromptGenerationInput, ValidateChapterVisualPlanInput, VisualFrameV3,
        VisualPlanDiagnostic, VisualPlanEditOperation, VisualPlanFieldChange,
        VisualPlanOutlineItem, VisualPlanPointGenerationOptions, VisualPlanPointGenerationPreview,
        VisualPlanProjection, VisualPlanPromptCandidate, VisualPlanReadView,
        VisualPlanValidationMode, VisualPlanValidationResult, VisualPointV3,
        VisualSegmentProjectionV3, VisualSegmentV3, VisualUserAdditionKindV1, VisualUserAdditionV1,
    },
    character::{
        AddProfileAttributeInput, ApplyImplicitUpdateDecisionInput,
        ApplyImplicitUpdateDecisionResultSummary, AttributeCountsSummary, AttributeEventSummary,
        AttributeHistoryInput, BehaviorRecordSummary, CharacterConflictCheckInput,
        CharacterConflictSeverity, CharacterConflictSummary, CharacterProfileCreateResult,
        CharacterProfileDeleteResult, CharacterProfileMutationResult,
        CharacterProfileProtectedAttribute, CharacterProfileSkippedResult, CharacterProfileSummary,
        CharacterProfileSyncBatchStatus, CharacterProfileSyncFailure, CharacterProfileSyncRange,
        CharacterProfileSyncSkipReason, CharacterProfileSyncedResult, CheckImplicitUpdateInput,
        CheckProfileSyncConflictInput, CreateProfileInput, DeletionMarkSummary,
        DiffProfileChaptersInput, DiscoverCharacterProfilesInput, DiscoverCharacterProfilesResult,
        DiscoveredCharacterProfileSummary, ExecuteAttributeDeletionInput,
        ExistingCharacterProfileDiscovery, GetImplicitReviewInput, GetProfileStateAtChapterInput,
        GetSnapshotInput, ImplicitAttributeReviewSummary, ImplicitEvidenceSummary,
        ImplicitReviewSummary, ImplicitUpdateDecisionSummary, ListBehaviorsInput,
        ListDeletionMarksInput, ListProfilesInput, ListSnapshotsInput, ModifyProfileAttributeInput,
        ProfileChapterStateSummary, ProfileSyncConflictSummary, RequestAttributeDeletionInput,
        ResetDeletionMarksInput, RollbackResultSummary, RollbackToChapterInput,
        SnapshotDeltaSummary, SnapshotReferenceSummary, SnapshotSummary,
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
        ModelDiscoveryInput, ModelDiscoveryResult, PricingPlan, UpdateEmbeddingInput,
        UpdateImageConfigInput, UpdateLlmInput, UpdateVideoConfigInput, VideoConfigInput,
        VideoConfigSummary, VideoDurationMode,
    },
    cost::{
        ModelUsageCapabilityTotal, ModelUsageCurrencyTotal, ModelUsageEntry, ModelUsageReport,
        ModelUsageReportInput,
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
        ListImagesByProjectInput, ListImagesByProjectsInput, ListVideoReferenceImagesInput,
        UpdateGeneratedImageInput, UploadImageInput,
    },
    image_generation::{
        GenerateBatchImagesInput, GeneratePageImageInput, GenerateSceneImageInput,
        ObjectProfileImageInput, ObjectProfileImagePromptOutput, SceneProfileImageInput,
        SceneProfileImagePromptOutput,
    },
    image_read::{ReadImageInput, ReadImageResult},
    import::{
        ImportAnalysisStageKey, ImportAnalysisStageProgress, ImportAnalysisStageStatus,
        ImportAnalysisStages, ImportAnalysisStatus, ImportProjectAnalysisSummary,
        ImportProjectTextUploadInput, MAX_PROJECT_TEXT_UPLOAD_BYTES, ProjectImportAnalysisProgress,
    },
    import_reverse::{
        ConfirmReverseEngineeringCandidateInput, GetReverseEngineeringCandidateInput,
        REVERSE_ENGINEERING_PROGRESS_TOTAL, ReverseEngineerInput, ReverseEngineeringCandidate,
        ReverseEngineeringCandidateState, ReverseEngineeringTarget,
    },
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
        MangaPipelineStatusesInput, MangaReadingFlowV2, MangaReferenceAssetKind,
        MangaReferenceBindingSummary, MangaShootingBeatRefV3, MangaShotScaleV2,
        MangaTextLayerKindV2, MangaTextLayerV2, MangaTextPlacementV2, OptimizeMangaPromptInput,
        UpdateMangaPromptInput,
    },
    optimize::{
        AnalyzeChapterInput, ApplyOptimizationPatchesInput, ApplyOptimizationPatchesResponse,
        CreateOptimizationChatSessionInput, DiscardOptimizationPatchSetInput,
        ListOptimizeSessionsInput, OptimizationAgentPermissions, OptimizationPatch,
        OptimizationPatchOperation, OptimizationPatchSetSummary, OptimizationSessionSummary,
        OptimizationTurnEventSummary, OptimizationTurnSummary, OptimizeSessionInput,
        PrepareOptimizationAnalysisSessionInput, ProposeOptimizationPatchesInput,
        RecordOptimizeAgentEventInput, SendOptimizationMessageInput,
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
    package::{
        GetPackageOperationInput, ListPackageCommandAvailabilityInput, PackageCatalog,
        PackageCommandAvailability, PackageCommandCategory, PackageDefinition, PackageHealth,
        PackageHostState, PackageId, PackageInstallationState, PackageLifecycleAction,
        PackageLifecycleState, PackageLifecycleSummary, PackageOverview, PackagePreferencesSummary,
        PackageReasonCode, StartPackageLifecycleInput, UpdatePackagePreferencesInput,
    },
    page::{
        PageAttachResponse, PageAuthorization, PageCommandAvailability, PageCommandManifest,
        PageContext, PageNavigationInput, PageOperationInput, PageOperationStatus,
        PageOperationUpdate, PageOperationView, PageSessionCreateResponse, PageSessionEvent,
        PageSessionStatus, PageSessionView,
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
        GenerationInputSource, PaginatedProjects, ProjectGeneratedDataResetResponse,
        ProjectGeneratedDataResetStatus, ProjectGeneratedDataResetSummary, ProjectKind,
        ProjectProgressSummary, ProjectRollbackImpactSummary, ProjectRollbackPreview,
        ProjectSourceMode, ProjectStatus, ProjectStatusHistorySummary, ProjectStatusMachineSummary,
        ProjectSummary, PromptSource, SourceSnapshotChapterSummary, UpdateProjectInput,
    },
    project_transfer::{
        ExportDerivedProjectDataInput, ExportProjectBundleDataInput, ExportProjectFamilyDataInput,
        ExportProjectPackageData, ExportVideoStudioProjectDataInput,
        ImportProjectPackageUploadInput, PROJECT_BUNDLE_PACKAGE_SUFFIX,
        PROJECT_FAMILY_PACKAGE_SUFFIX, ProjectBundleImportReport, ProjectBundleProviderSlotReport,
        VIDEO_STUDIO_PROJECT_PACKAGE_SUFFIX,
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
        ClientRagIngestPrepareOutput, ClientRagIngestUploadInput, EnhancedSearchInput,
        PreparedClientRagIngestRecord, RagEnhancedQuerySummary, RagEnhancedSearchOutput,
        RagEnhancedSearchWithVectorsInput, RagIngestOptions, RagIngestResult,
        RagPreparedEnhancedSearchOutput, RagSearchResult, UploadedRagEmbedding,
    },
    request_preview::{ModelRequestPreview, ModelRequestPreviewStep},
    runtime_config::{
        RuntimeConfigSnapshot, RuntimeEmbeddingConfig, RuntimeImageConfig, RuntimeLlmConfig,
        RuntimeVideoConfig, RuntimeWebFetchConfig, RuntimeWebSearchConfig,
    },
    shooting_script::{
        ChapterShootingScriptRecordSummary, ChapterShootingScriptSchemaVersion,
        ChapterShootingScriptSourceInput, ChapterShootingScriptUnitProgressView,
        ChapterShootingScriptV1, ChapterShootingScriptView, DeleteChapterShootingScriptInput,
        EditChapterShootingScriptInput, EditChapterShootingScriptProgressInput,
        EditedChapterShootingScriptProgressView, EditedChapterShootingScriptView,
        GenerateChapterShootingScriptInput, ShootingBlockingV1, ShootingCameraAngleV1,
        ShootingCameraHeightV1, ShootingCameraMovementV1, ShootingContinuityIntentV1,
        ShootingCoverageV1, ShootingDialogueCueV1, ShootingDirectingStatementV1,
        ShootingEditingModeV1, ShootingPerformanceBeatV1, ShootingSceneV1,
        ShootingScriptPlanSyncV1, ShootingShotSizeV1, ShootingShotV1, ShootingSoundDesignV1,
        ShootingSpatialContinuityV1, ShootingTransitionContractV1, ShootingTransitionTypeV1,
        ShootingViewpointModeV1, ShootingViewpointV1, ShootingVisualConstraintsV3,
        ShootingVisualPlanSourceV1,
    },
    source_sync::{
        ApplySourceSyncInput, SourceSyncChangeKind, SourceSyncConflictSeverity,
        SourceSyncConflictStatus, SourceSyncConflictSummary, SourceSyncDecisionInput,
        SourceSyncLocalStateKind, SourceSyncPlanSummary, SourceSyncProjectInput,
        SourceSyncResolutionStrategy, SourceSyncRunMode, SourceSyncRunStatus, SourceSyncRunSummary,
    },
    storage::{OrphanProjectDataDirSummary, OrphanProjectDataDirsSummary, StoragePathsSummary},
    task::{
        CancelTaskInput, GetTaskHistoryInput, QueueSlotStatsSummary, QueueStatsSummary,
        TaskInfoSummary, TaskProgressSummary,
    },
    theme::{
        CreateThemeInput, DeleteThemeResourceInput, ThemeAppearance, ThemeBackgroundSettings,
        ThemeDocument, ThemeMode, ThemePackageExport, ThemePackageUploadInput,
        ThemePreferenceSummary, ThemeResourceSummary, ThemeResourceUploadInput,
        ThemeStreamDisplayMode, ThemeStreamDisplaySettings, ThemeSummary, UpdateThemeInput,
        UpdateThemePreferenceInput,
    },
    video::{
        ApplyAssemblyPostprocessEditInput, AssembledChapterVideoClipSummary,
        AssembledChapterVideoPreview, AssemblyPostprocessOperation, AssemblyPushAction,
        AssemblyPushResult, ChapterVideoPromptSummary, CommitInsertedSegmentDraftInput,
        DeleteChapterVideoPromptInput, DeleteInsertedSegmentInput, DownstreamConflictStrategy,
        GenerateChapterVideosBatchResult, GeneratedVideoSummary, GetVideoTimelineInput,
        MoveInsertedSegmentInput, OptimizeChapterVideoPromptInput,
        PreviewChapterVideoPromptOverrideInput, PushAssembledChapterVideoInput,
        UpdateAssemblyPushStatusInput, UpdateChapterVideoPromptInput,
        UpdateChapterVideoSegmentPlanInput, UpdateGeneratedVideoInput,
        UploadAssemblyInsertVideoInput, UploadChapterVideoInput, UploadReferenceMediaVideoInput,
        VideoPlaybackData, VideoScrubFramesData, VideoSegmentTarget, VideoTimelineItemKind,
        VideoTimelineItemStatus, VideoTimelineItemSummary, VideoTimelineSummary,
    },
    web_fetch::{WebFetchInput, WebFetchResult},
    web_search::{WebSearchInput, WebSearchResult, WebSearchSource},
    workflow::{
        CancelWorkflowInput, GenerationRunSummary, GetWorkflowRunInput, ListWorkflowRunsInput,
        ResumeWorkflowRunInput, WorkflowCheckpointSummary, WorkflowProgressEvent,
        WorkflowProgressEventsInput, WorkflowProgressEventsResponse, WorkflowRunDetailSummary,
        WorkflowRunProgressSummary, WorkflowRunSummary, WorkflowStateSummary,
    },
};
use a4n_domain::agent::{AGENT_PROTOCOL_JSON_COMPAT, AGENT_PROTOCOL_NATIVE_TOOL_CALLING};
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
use a4n_domain::{
    DEFAULT_IMAGE_SIZES, DEFAULT_LLM_MAX_CONTEXT_TOKENS, DEFAULT_LLM_MAX_OUTPUT_TOKENS, ImageSize,
    LLM_RESERVED_REQUEST_OPTION_KEYS, THEME_DOCUMENT_VERSION, THEME_GLOBAL_ROUTE_FAMILY,
    THEME_ROUTE_FAMILIES, THEME_TOKEN_NAMES,
};
use a4n_progress::{ProgressMeta, ProgressPhaseKind, ProgressSeverity};
use a4n_video_contract::{
    VideoEmotionActionGoalV1, VideoEmotionalArcV1, VideoIntentV1, VideoMaterialCoverageV1,
    VideoPerformanceActionV1, VideoPerformancePlanV2, VideoPerformanceTimeBlockV1,
    VideoSegmentPlanV2, VideoSegmentTransitionV1, VideoSpatialAnchorV1,
};
use schemars::JsonSchema;
use serde_json::Value;
use std::{env, fs, path::PathBuf};

#[path = "generate_types/stages.rs"]
mod stages;

fn main() {
    stages::run();
}

fn generated_path(file_name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../frontend/src/api/generated")
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
    // AgentBudget is also the nested partial payload for a budget extension.
    let force_nullable_fields_required = !name.ends_with("Input") && name != "AgentBudget";

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

fn typescript_image_constants() -> String {
    let sizes = DEFAULT_IMAGE_SIZES
        .iter()
        .map(|size| format!("  {{ width: {}, height: {} }},\n", size.width, size.height))
        .collect::<String>();
    format!("export const DEFAULT_IMAGE_SIZES: readonly ImageSize[] = [\n{sizes}];\n\n")
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

fn typescript_theme_constants() -> String {
    let routes = THEME_ROUTE_FAMILIES
        .iter()
        .map(|route| format!("{route:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    let tokens = THEME_TOKEN_NAMES
        .iter()
        .map(|token| format!("{token:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "export const THEME_DOCUMENT_VERSION = {THEME_DOCUMENT_VERSION};\n\
         export const THEME_GLOBAL_ROUTE_FAMILY = {THEME_GLOBAL_ROUTE_FAMILY:?};\n\
         export const THEME_ROUTE_FAMILIES = [{routes}] as const;\n\
         export type ThemeRouteFamily = (typeof THEME_ROUTE_FAMILIES)[number];\n\
         export const THEME_TOKEN_NAMES = [{tokens}] as const;\n\n"
    )
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
    let reserved = LLM_RESERVED_REQUEST_OPTION_KEYS
        .iter()
        .map(|key| format!("{key:?}"))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "export const DEFAULT_LLM_MAX_CONTEXT_TOKENS = {DEFAULT_LLM_MAX_CONTEXT_TOKENS};\n\
         export const DEFAULT_LLM_MAX_OUTPUT_TOKENS = {DEFAULT_LLM_MAX_OUTPUT_TOKENS};\n\
         export const AGENT_PROTOCOL_NATIVE_TOOL_CALLING = {AGENT_PROTOCOL_NATIVE_TOOL_CALLING:?};\n\
         export const AGENT_PROTOCOL_JSON_COMPAT = {AGENT_PROTOCOL_JSON_COMPAT:?};\n\
         export const LLM_RESERVED_REQUEST_OPTION_KEYS: readonly string[] = [{reserved}];\n\n"
    )
}

fn typescript_project_package_suffixes() -> String {
    format!(
        "export const PROJECT_BUNDLE_PACKAGE_SUFFIX = {PROJECT_BUNDLE_PACKAGE_SUFFIX:?};\n\
         export const PROJECT_FAMILY_PACKAGE_SUFFIX = {PROJECT_FAMILY_PACKAGE_SUFFIX:?};\n\
         export const VIDEO_STUDIO_PROJECT_PACKAGE_SUFFIX = {VIDEO_STUDIO_PROJECT_PACKAGE_SUFFIX:?};\n\n"
    )
}

fn typescript_import_constants() -> String {
    format!(
        "export const MAX_PROJECT_TEXT_UPLOAD_BYTES = {MAX_PROJECT_TEXT_UPLOAD_BYTES};\nexport const REVERSE_ENGINEERING_PROGRESS_TOTAL = {REVERSE_ENGINEERING_PROGRESS_TOTAL};\n\n"
    )
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
