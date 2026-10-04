use a4n_api_contract::galgame::{
    AdvanceGalgameStoryRuntimeInput, GalgameStoryGraphMutationApplicationPort,
    GalgameStoryRuntimeView, GetGalgameStoryRuntimeInput, SaveGalgameStoryGraphInput,
    StartGalgameStoryRuntimeInput,
};
use a4n_domain::project::ProjectKind;
use a4n_domain::{
    collaboration::ProjectAccessRole,
    galgame::{StoryGraph, StoryRuntimeState},
};
use a4n_error::AppError;

use crate::OrchestratorContext;

use super::errors::{galgame_story_graph_record_error, galgame_story_runtime_record_error};

impl OrchestratorContext {
    pub(crate) async fn ensure_story_graph_regeneration_safe(
        &self,
        project_id: i64,
    ) -> Result<(), AppError> {
        let current =
            a4n_db::galgame::current_galgame_story_graph(self.database.executor(), project_id)
                .await
                .map_err(galgame_story_graph_record_error)?;
        if current.is_some_and(|record| record.is_user_edited()) {
            return Err(AppError::InvalidState {
                current: "story_graph_manual_edits".into(),
                operation: "重新生成会覆盖手工节点修改，请先重置 Galgame 生成数据".into(),
            });
        }
        Ok(())
    }

    async fn save_story_graph_for_user_validated(
        &self,
        user_id: i64,
        project_id: i64,
        input: SaveGalgameStoryGraphInput,
    ) -> Result<StoryGraph, AppError> {
        self.verify_galgame_project_access(user_id, project_id, ProjectAccessRole::Editor)
            .await?;
        let record = a4n_db::galgame::save_galgame_story_graph_revision(
            self.database.executor(),
            project_id,
            input.expected_revision,
            "edited",
            input.graph,
        )
        .await
        .map_err(galgame_story_graph_record_error)?;
        Ok(record.graph)
    }

    async fn current_graph_for_runtime(&self, project_id: i64) -> Result<StoryGraph, AppError> {
        let project = self
            .project_by_id(project_id)
            .await?
            .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))?;
        if project.project_kind != ProjectKind::Galgame.as_str() {
            return Err(AppError::InvalidArgument("项目不是 Galgame 项目".into()));
        }
        let record =
            a4n_db::galgame::current_galgame_story_graph(self.database.executor(), project_id)
                .await
                .map_err(galgame_story_graph_record_error)?
                .ok_or_else(|| AppError::NotFound("Galgame 项目尚无节点图".into()))?;
        if record.status == "stale" || project.source_snapshot_id != Some(record.source_snapshot_id)
        {
            return Err(AppError::InvalidState {
                current: "story_graph_stale".into(),
                operation: "运行来源已变化的 Galgame 节点图".into(),
            });
        }
        Ok(record.graph)
    }
}

#[async_trait::async_trait]
impl GalgameStoryGraphMutationApplicationPort for OrchestratorContext {
    async fn save(
        &self,
        user_id: i64,
        project_id: i64,
        input: SaveGalgameStoryGraphInput,
    ) -> Result<StoryGraph, AppError> {
        self.save_story_graph_for_user_validated(user_id, project_id, input)
            .await
    }

    async fn runtime_state(
        &self,
        user_id: i64,
        project_id: i64,
        input: GetGalgameStoryRuntimeInput,
    ) -> Result<Option<GalgameStoryRuntimeView>, AppError> {
        self.verify_galgame_project_access(user_id, project_id, ProjectAccessRole::Viewer)
            .await?;
        let graph = self.current_graph_for_runtime(project_id).await?;
        let record = a4n_db::galgame::galgame_story_runtime_state(
            self.database.executor(),
            user_id,
            project_id,
            &input.save_slot,
        )
        .await
        .map_err(galgame_story_runtime_record_error)?;
        record
            .map(|record| galgame_story_runtime_view(&graph, record.state))
            .transpose()
    }

    async fn start_runtime(
        &self,
        user_id: i64,
        project_id: i64,
        input: StartGalgameStoryRuntimeInput,
    ) -> Result<GalgameStoryRuntimeView, AppError> {
        self.verify_galgame_project_access(user_id, project_id, ProjectAccessRole::Viewer)
            .await?;
        let graph = self.current_graph_for_runtime(project_id).await?;
        let state = StoryRuntimeState::new(&graph).map_err(|message| AppError::InvalidState {
            current: "story_graph_invalid".into(),
            operation: message,
        })?;
        let record = a4n_db::galgame::create_galgame_story_runtime_state(
            self.database.executor(),
            user_id,
            project_id,
            &input.save_slot,
            state,
        )
        .await
        .map_err(galgame_story_runtime_record_error)?;
        galgame_story_runtime_view(&graph, record.state)
    }

    async fn advance_runtime(
        &self,
        user_id: i64,
        project_id: i64,
        input: AdvanceGalgameStoryRuntimeInput,
    ) -> Result<GalgameStoryRuntimeView, AppError> {
        self.verify_galgame_project_access(user_id, project_id, ProjectAccessRole::Viewer)
            .await?;
        let graph = self.current_graph_for_runtime(project_id).await?;
        let record = a4n_db::galgame::galgame_story_runtime_state(
            self.database.executor(),
            user_id,
            project_id,
            &input.save_slot,
        )
        .await
        .map_err(galgame_story_runtime_record_error)?
        .ok_or_else(|| AppError::NotFound(format!("存档槽不存在: {}", input.save_slot)))?;
        let mut state = record.state;
        state
            .advance(&graph, &input.edge_id)
            .map_err(|message| AppError::InvalidState {
                current: "galgame_runtime_transition_rejected".into(),
                operation: message,
            })?;
        let updated = a4n_db::galgame::advance_galgame_story_runtime_state(
            self.database.executor(),
            user_id,
            project_id,
            &input.save_slot,
            input.expected_step_index,
            state,
        )
        .await
        .map_err(galgame_story_runtime_record_error)?;
        galgame_story_runtime_view(&graph, updated.state)
    }
}

fn galgame_story_runtime_view(
    graph: &StoryGraph,
    state: StoryRuntimeState,
) -> Result<GalgameStoryRuntimeView, AppError> {
    let current_node = graph
        .nodes
        .iter()
        .find(|node| node.id == state.current_node_id)
        .cloned()
        .ok_or_else(|| AppError::InvalidState {
            current: "galgame_runtime_node_missing".into(),
            operation: "读取当前叙事节点".into(),
        })?;
    let available_edges = state
        .available_edges(graph)
        .map_err(|message| AppError::InvalidState {
            current: "galgame_runtime_state_invalid".into(),
            operation: message,
        })?
        .into_iter()
        .cloned()
        .collect();
    Ok(GalgameStoryRuntimeView {
        state,
        current_node,
        available_edges,
    })
}
