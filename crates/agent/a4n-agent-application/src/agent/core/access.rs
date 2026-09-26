//! Agent scope 与资源访问校验。

use super::*;

impl AgentApplication {
    pub(crate) async fn owned_thread(
        &self,
        user_id: i64,
        thread_id: i64,
    ) -> Result<thread::Model, AppError> {
        validate_user_id(user_id)?;
        let row = agent::thread_by_id(self.connection(), thread_id)
            .await
            .map_err(agent_db_error_to_app)?
            .ok_or_else(|| AppError::NotFound(format!("Agent thread 不存在: {thread_id}")))?;
        if row.owner_user_id != user_id {
            return Err(AppError::Forbidden("无权访问此 Agent thread".into()));
        }
        Ok(row)
    }

    pub(crate) async fn verify_thread_scope(
        &self,
        user_id: i64,
        scope: &AgentScope,
        bound_resource: Option<&AgentResourceRef>,
    ) -> Result<(Option<i64>, Option<i64>, Option<i32>), AppError> {
        let mut project_id = scope.project_id;
        let mut chapter_id = scope.chapter_id;
        if let (Some(kind), Some(id)) = (&scope.resource_kind, &scope.resource_id) {
            collect_scoped_resource(kind, id, &mut project_id, &mut chapter_id)?;
        }
        if let Some(resource) = bound_resource {
            collect_scoped_resource(
                &resource.kind,
                &resource.id,
                &mut project_id,
                &mut chapter_id,
            )?;
        }
        let chapter_number = if let Some(chapter_id) = chapter_id {
            let chapter = a4n_db::project::chapter_by_id(self.connection(), chapter_id)
                .await
                .map_err(AppError::database)?
                .ok_or_else(|| AppError::NotFound(format!("章节不存在: {chapter_id}")))?;
            merge_scope_id(&mut project_id, chapter.project_id, "project_id")?;
            Some(chapter.chapter_number)
        } else {
            None
        };
        if (scope.resource_kind.is_some() || bound_resource.is_some()) && project_id.is_none() {
            return Err(AppError::InvalidArgument(
                "非项目/章节资源绑定必须在 scope.project_id 中声明所属项目".into(),
            ));
        }
        if let Some(project_id) = project_id {
            a4n_db::collaboration::verify_project_permission(
                self.connection(),
                project_id,
                user_id,
                ProjectAccessRole::Viewer,
            )
            .await
            .map_err(collaboration_db_error_to_app)?;
        }
        Ok((project_id, chapter_id, chapter_number))
    }

    pub(crate) async fn hydrate_scope_chapter_number(
        &self,
        scope: &mut AgentScope,
    ) -> Result<(), AppError> {
        let Some(chapter_id) = scope.chapter_id else {
            scope.chapter_number = None;
            return Ok(());
        };
        let chapter = a4n_db::project::chapter_by_id(self.connection(), chapter_id)
            .await
            .map_err(AppError::database)?
            .ok_or_else(|| AppError::NotFound(format!("章节不存在: {chapter_id}")))?;
        if scope
            .project_id
            .is_some_and(|project_id| project_id != chapter.project_id)
        {
            return Err(AppError::Forbidden(format!(
                "章节 {chapter_id} 不属于 Agent scope 项目 {}",
                scope.project_id.expect("checked Some")
            )));
        }
        scope.chapter_number = Some(chapter.chapter_number);
        Ok(())
    }

    /// Validates chapter identifiers found inside a dynamically discovered
    /// Host command against a project-bound Agent scope. Argument names alone
    /// cannot prove this relationship, so the durable chapter row is the
    /// single source of truth at the runtime boundary.
    pub(crate) async fn verify_chapters_in_project(
        &self,
        user_id: i64,
        project_id: i64,
        chapter_ids: &[i64],
    ) -> Result<(), AppError> {
        validate_user_id(user_id)?;
        if project_id <= 0 {
            return Err(AppError::InvalidArgument("projectId 无效".into()));
        }
        a4n_db::collaboration::verify_project_permission(
            self.connection(),
            project_id,
            user_id,
            ProjectAccessRole::Viewer,
        )
        .await
        .map_err(collaboration_db_error_to_app)?;
        let mut seen = HashSet::new();
        for chapter_id in chapter_ids {
            if !seen.insert(*chapter_id) {
                continue;
            }
            if *chapter_id <= 0 {
                return Err(AppError::InvalidArgument("chapterId 无效".into()));
            }
            let chapter = a4n_db::project::chapter_by_id(self.connection(), *chapter_id)
                .await
                .map_err(AppError::database)?
                .ok_or_else(|| AppError::NotFound(format!("章节不存在: {chapter_id}")))?;
            if chapter.project_id != project_id {
                return Err(AppError::Forbidden(format!(
                    "章节 {chapter_id} 不属于 Agent scope 项目 {project_id}"
                )));
            }
        }
        Ok(())
    }

    pub(super) async fn append_user_event(
        &self,
        user_id: i64,
        fields: UserEventFields,
        kind: &str,
        status: &str,
        public_text: &str,
    ) -> Result<(), AppError> {
        agent::append_event(
            self.connection(),
            AgentEventInput {
                thread_id: fields.thread_id,
                turn_id: fields.turn_id,
                run_id: fields.run_id,
                execution_id: None,
                delegation_id: None,
                message_id: fields.message_id,
                actor_kind: "user".into(),
                actor_id: Some(user_id.to_string()),
                kind: kind.into(),
                status: status.into(),
                public_text: public_text.into(),
                payload_json: None,
            },
        )
        .await
        .map_err(agent_db_error_to_app)?;
        Ok(())
    }
}
