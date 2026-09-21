use super::*;

impl AppState {
    pub(crate) fn agent_thread_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentThreadMutationApplicationPort {
        self.agent_thread_mutation_application.as_ref()
    }

    pub(crate) fn agent_thread_read_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentThreadReadApplicationPort {
        self.agent_thread_read_application.as_ref()
    }

    pub(crate) fn agent_run_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentRunMutationApplicationPort {
        self.agent_run_mutation_application.as_ref()
    }

    pub(crate) fn agent_run_read_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentRunReadApplicationPort {
        self.agent_run_read_application.as_ref()
    }

    pub(crate) fn agent_delegation_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentDelegationApplicationPort {
        self.agent_delegation_application.as_ref()
    }

    pub(crate) fn agent_tool_read_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentToolReadApplicationPort {
        self.agent_tool_read_application.as_ref()
    }

    pub(crate) fn agent_skill_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentSkillApplicationPort {
        self.agent_skill_application.as_ref()
    }

    pub(crate) fn agent_event_read_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentEventReadApplicationPort {
        self.agent_event_read_application.as_ref()
    }

    pub(crate) fn agent_memory_read_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentMemoryReadApplicationPort {
        self.agent_memory_read_application.as_ref()
    }

    pub(crate) fn agent_memory_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentMemoryMutationApplicationPort {
        self.agent_memory_mutation_application.as_ref()
    }

    pub(crate) fn agent_approval_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentApprovalApplicationPort {
        self.agent_approval_application.as_ref()
    }

    pub(crate) fn agent_resource_access_application(
        &self,
    ) -> &dyn a4n_api_contract::agent::AgentResourceAccessApplicationPort {
        self.agent_resource_access_application.as_ref()
    }

    pub(crate) fn agent_external_task_wait_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentExternalTaskWaitApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_editor_wait_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentEditorWaitApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_editor_claim_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentEditorClaimApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_generation_cancel_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentGenerationCancelApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_workflow_cancel_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkflowCancelApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_budget_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerBudgetApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_approval_expiry_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentApprovalExpiryApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_run_lease_recovery_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentRunLeaseRecoveryApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_lease_application(
        &self,
    ) -> Arc<dyn a4n_agent_runtime_contract::AgentWorkerLeaseApplicationPort> {
        self.agent_worker_application.clone()
    }

    pub(crate) fn agent_worker_claim_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerClaimApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_failure_application(
        &self,
    ) -> Arc<dyn a4n_agent_runtime_contract::AgentWorkerFailureApplicationPort> {
        self.agent_worker_application.clone()
    }

    pub(crate) fn agent_worker_wait_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerWaitApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_read_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerReadApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_context_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerContextApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_execution_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerExecutionApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_approval_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerApprovalApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_completion_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerCompletionApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_delegation_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerDelegationApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_history_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerHistoryApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_inbox_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerInboxApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_message_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerMessageApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_usage_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerUsageApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_editor_response_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentEditorResponseApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_recovery_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerRecoveryApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_worker_activation_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkerActivationApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn agent_workflow_binding_application(
        &self,
    ) -> &dyn a4n_agent_runtime_contract::AgentWorkflowBindingApplicationPort {
        self.agent_worker_application.as_ref()
    }

    pub(crate) fn canvas_read_application(
        &self,
    ) -> &dyn a4n_api_contract::canvas::CanvasReadApplicationPort {
        self.canvas_read_application.as_ref()
    }

    pub(crate) fn canvas_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::canvas::CanvasMutationApplicationPort {
        self.canvas_mutation_application.as_ref()
    }

    pub(crate) fn canvas_render_text_application(
        &self,
    ) -> &dyn a4n_canvas_contract::CanvasRenderTextNodeApplicationPort {
        self.canvas_render_text_application.as_ref()
    }

    pub(crate) fn canvas_llm_text_application(
        &self,
    ) -> &dyn a4n_canvas_host_contract::CanvasLlmTextNodeApplicationPort {
        self.canvas_llm_text_application.as_ref()
    }

    pub(crate) fn canvas_image_application(
        &self,
    ) -> &dyn a4n_canvas_host_contract::CanvasImageNodeApplicationPort {
        self.canvas_image_application.as_ref()
    }

    pub(crate) fn canvas_video_application(
        &self,
    ) -> &dyn a4n_canvas_host_contract::CanvasVideoNodeApplicationPort {
        self.canvas_video_application.as_ref()
    }

    pub(crate) fn canvas_preview_application(
        &self,
    ) -> &dyn a4n_canvas_host_contract::CanvasPreviewApplicationPort {
        self.canvas_preview_application.as_ref()
    }

    pub(crate) fn canvas_editing_application(
        &self,
    ) -> &dyn a4n_canvas_contract::CanvasEditingNodeApplicationPort {
        self.canvas_editing_application.as_ref()
    }

    pub(crate) fn canvas_probe_application(
        &self,
    ) -> &dyn a4n_canvas_contract::CanvasProbeNodeApplicationPort {
        self.canvas_probe_application.as_ref()
    }

    pub(crate) fn canvas_output_application(
        &self,
    ) -> &dyn a4n_canvas_contract::CanvasOutputNodeApplicationPort {
        self.canvas_output_application.as_ref()
    }

    pub(crate) fn canvas_image_import_application(
        &self,
    ) -> &dyn a4n_canvas_contract::CanvasImageImportApplicationPort {
        self.canvas_image_import_application.as_ref()
    }

    pub(crate) fn canvas_video_import_application(
        &self,
    ) -> &dyn a4n_canvas_contract::CanvasVideoImportApplicationPort {
        self.canvas_video_import_application.as_ref()
    }

    pub(crate) fn canvas_audio_import_application(
        &self,
    ) -> &dyn a4n_canvas_contract::CanvasAudioImportApplicationPort {
        self.canvas_audio_import_application.as_ref()
    }

    pub(crate) fn canvas_graph_preparation_application(
        &self,
    ) -> &dyn a4n_canvas_contract::CanvasGraphPreparationApplicationPort {
        self.canvas_graph_preparation_application.as_ref()
    }

    pub(crate) fn canvas_graph_execution_application(
        &self,
    ) -> &dyn a4n_canvas_contract::CanvasGraphExecutionApplicationPort {
        self.canvas_graph_execution_application.as_ref()
    }

    pub(crate) fn canvas_graph_failure_application(
        &self,
    ) -> &dyn a4n_canvas_contract::CanvasGraphFailureApplicationPort {
        self.canvas_graph_failure_application.as_ref()
    }

    pub(crate) fn package_read_application(
        &self,
    ) -> &dyn a4n_api_contract::package::PackageReadApplicationPort {
        self.package_read_application.as_ref()
    }

    pub(crate) fn package_mutation_application(
        &self,
    ) -> &dyn a4n_api_contract::package::PackageMutationApplicationPort {
        self.package_mutation_application.as_ref()
    }

    pub(crate) fn import_analysis_generation_application(
        &self,
    ) -> &dyn a4n_import_analysis_host_contract::ImportAnalysisGenerationApplicationPort {
        self.import_analysis_generation_application.as_ref()
    }

    pub(crate) fn import_analysis_cancellation_application(
        &self,
    ) -> &dyn a4n_api_contract::import::ImportAnalysisCancellationApplicationPort {
        self.import_analysis_cancellation_application.as_ref()
    }

    pub(crate) fn import_analysis_progress_application(
        &self,
    ) -> &dyn a4n_api_contract::import::ImportAnalysisProgressApplicationPort {
        self.import_analysis_progress_application.as_ref()
    }

    pub(crate) fn import_analysis_preview_application(
        &self,
    ) -> &dyn a4n_import_analysis_host_contract::ImportAnalysisPreviewApplicationPort {
        self.import_analysis_preview_application.as_ref()
    }

    pub(crate) fn import_reverse_preview_application(
        &self,
    ) -> &dyn a4n_novel_host_contract::ImportReversePreviewApplicationPort {
        self.import_reverse_preview_application.as_ref()
    }

    pub(crate) fn import_reverse_generation_application(
        &self,
    ) -> &dyn a4n_novel_host_contract::ImportReverseGenerationApplicationPort {
        self.import_reverse_generation_application.as_ref()
    }

    pub(crate) fn import_reverse_confirmation_application(
        &self,
    ) -> &dyn a4n_api_contract::import_reverse::ImportReverseConfirmationApplicationPort {
        self.import_reverse_confirmation_application.as_ref()
    }

    pub(crate) fn import_reverse_read_application(
        &self,
    ) -> &dyn a4n_api_contract::import_reverse::ImportReverseReadApplicationPort {
        self.import_reverse_read_application.as_ref()
    }

    pub(crate) fn chapter_analysis_generation_application(
        &self,
    ) -> &dyn a4n_chapter_analysis_host_contract::ChapterAnalysisGenerationApplicationPort {
        self.chapter_analysis_generation_application.as_ref()
    }

    pub(crate) fn chapter_analysis_resume_application(
        &self,
    ) -> &dyn a4n_chapter_analysis_host_contract::ChapterAnalysisResumeApplicationPort {
        self.chapter_analysis_resume_application.as_ref()
    }

    pub(crate) fn chapter_analysis_generation_preview_application(
        &self,
    ) -> &dyn a4n_chapter_analysis_host_contract::ChapterAnalysisGenerationPreviewApplicationPort
    {
        self.chapter_analysis_generation_preview_application
            .as_ref()
    }
}
