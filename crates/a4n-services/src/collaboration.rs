//! Project collaboration service.

use chrono::{Duration, Utc};
use sea_orm::sea_query::Expr;
use sea_orm::sea_query::OnConflict;
use sea_orm::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

use a4n_core::error::AppError;
use a4n_core::models::{
    project::{novel_project, project_invite, project_member},
    user::{role::ROLE_READ_ONLY, user},
};

use crate::project::{
    PROJECT_KIND_TEXT, SOURCE_MODE_FREEFORM, SOURCE_MODE_SELF, normalize_project_kind,
};

const MEMBER_ROLE_MANAGER: &str = "manager";
const MEMBER_ROLE_EDITOR: &str = "editor";
const MEMBER_ROLE_VIEWER: &str = "viewer";

const INVITE_STATUS_PENDING: &str = "pending";
const INVITE_STATUS_ACCEPTED: &str = "accepted";
const INVITE_STATUS_DECLINED: &str = "declined";
const INVITE_STATUS_REVOKED: &str = "revoked";
const INVITE_STATUS_EXPIRED: &str = "expired";

const INVITE_TTL_DAYS: i64 = 7;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectMemberRole {
    Manager,
    Editor,
    Viewer,
}

impl ProjectMemberRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manager => MEMBER_ROLE_MANAGER,
            Self::Editor => MEMBER_ROLE_EDITOR,
            Self::Viewer => MEMBER_ROLE_VIEWER,
        }
    }
}

impl std::fmt::Display for ProjectMemberRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

pub fn parse_project_member_role(value: &str) -> Result<ProjectMemberRole, AppError> {
    match value.trim() {
        MEMBER_ROLE_MANAGER => Ok(ProjectMemberRole::Manager),
        MEMBER_ROLE_EDITOR => Ok(ProjectMemberRole::Editor),
        MEMBER_ROLE_VIEWER => Ok(ProjectMemberRole::Viewer),
        other => Err(AppError::InvalidArgument(format!(
            "未知项目协作角色: {other}"
        ))),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectAccessRole {
    Owner,
    Manager,
    Editor,
    Viewer,
}

impl ProjectAccessRole {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Owner => "owner",
            Self::Manager => MEMBER_ROLE_MANAGER,
            Self::Editor => MEMBER_ROLE_EDITOR,
            Self::Viewer => MEMBER_ROLE_VIEWER,
        }
    }

    fn rank(self) -> u8 {
        match self {
            Self::Viewer => 1,
            Self::Editor => 2,
            Self::Manager => 3,
            Self::Owner => 4,
        }
    }

    fn satisfies(self, required: ProjectAccessRole) -> bool {
        self.rank() >= required.rank()
    }
}

impl std::fmt::Display for ProjectAccessRole {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectAccessScope {
    All,
    Owned,
    Collaborating,
}

impl ProjectAccessScope {
    pub fn from_optional(value: Option<&str>) -> Result<Self, AppError> {
        match value.map(str::trim).filter(|v| !v.is_empty()) {
            None | Some("all") => Ok(Self::All),
            Some("owned") => Ok(Self::Owned),
            Some("collaborating") => Ok(Self::Collaborating),
            Some(other) => Err(AppError::InvalidArgument(format!(
                "项目访问范围无效: {other}"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct AccessibleProjectListFilter<'a> {
    pub project_kind: Option<&'a str>,
    pub root_project_id: Option<i64>,
    pub access_scope: ProjectAccessScope,
}

#[derive(Debug, Clone)]
pub struct ProjectAccessInfo {
    pub collaboration_root_project_id: i64,
    pub owner_user_id: i64,
    pub owner_username: Option<String>,
    pub my_role: Option<ProjectAccessRole>,
    pub member_count: u64,
    pub pending_invite_count: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectMemberSummary {
    pub project_id: i64,
    pub user_id: i64,
    pub username: String,
    pub email: String,
    pub role: String,
    pub is_owner: bool,
    pub invited_by_user_id: Option<i64>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectInviteSummary {
    pub id: i64,
    pub project_id: i64,
    pub project_title: Option<String>,
    pub inviter_user_id: i64,
    pub inviter_username: Option<String>,
    pub invitee_user_id: i64,
    pub invitee_username: Option<String>,
    pub invitee_email: Option<String>,
    pub role: String,
    pub status: String,
    pub expires_at: String,
    pub accepted_at: Option<String>,
    pub created_at: Option<String>,
    pub updated_at: Option<String>,
}

#[derive(Clone)]
pub struct ProjectCollaborationService {
    db: Arc<DatabaseConnection>,
}

impl ProjectCollaborationService {
    pub fn new(db: Arc<DatabaseConnection>) -> Self {
        Self { db }
    }

    pub async fn resolve_root_project(
        &self,
        project_id: i64,
    ) -> Result<novel_project::Model, AppError> {
        let project = find_project(self.db.as_ref(), project_id).await?;
        let root_id = project.root_project_id.unwrap_or(project.id);
        let root = if root_id == project.id {
            project
        } else {
            find_project(self.db.as_ref(), root_id).await?
        };
        if root.project_kind != PROJECT_KIND_TEXT
            && !(root.project_kind == "video" && root.source_mode == SOURCE_MODE_FREEFORM)
        {
            return Err(AppError::InvalidArgument(
                "协作权限只能绑定到文本根项目或自由视频项目".to_string(),
            ));
        }
        if root.project_kind == PROJECT_KIND_TEXT && root.source_mode != SOURCE_MODE_SELF {
            return Err(AppError::InvalidArgument(
                "文本协作根项目 source_mode 必须为 self".to_string(),
            ));
        }
        Ok(root)
    }

    pub async fn access_info(
        &self,
        project_id: i64,
        user_id: i64,
    ) -> Result<ProjectAccessInfo, AppError> {
        let root = self.resolve_root_project(project_id).await?;
        let role = self.effective_role_for_root(&root, user_id).await?;
        let owner = find_user(self.db.as_ref(), root.user_id).await.ok();
        let member_count = project_member::Entity::find()
            .filter(project_member::Column::ProjectId.eq(root.id))
            .count(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let pending_invite_count = if role
            .map(|role| role.satisfies(ProjectAccessRole::Manager))
            .unwrap_or(false)
        {
            Some(
                project_invite::Entity::find()
                    .filter(project_invite::Column::ProjectId.eq(root.id))
                    .filter(project_invite::Column::Status.eq(INVITE_STATUS_PENDING))
                    .filter(project_invite::Column::ExpiresAt.gt(Utc::now().naive_utc()))
                    .count(self.db.as_ref())
                    .await
                    .map_err(AppError::Database)?,
            )
        } else {
            None
        };

        Ok(ProjectAccessInfo {
            collaboration_root_project_id: root.id,
            owner_user_id: root.user_id,
            owner_username: owner.map(|user| user.username),
            my_role: role,
            member_count,
            pending_invite_count,
        })
    }

    pub async fn access_info_for_projects(
        &self,
        projects: &[novel_project::Model],
        user_id: i64,
    ) -> Result<HashMap<i64, ProjectAccessInfo>, AppError> {
        if projects.is_empty() {
            return Ok(HashMap::new());
        }

        let user = find_user(self.db.as_ref(), user_id).await?;
        if !user.is_active {
            return Err(AppError::Forbidden("账户已被禁用".to_string()));
        }

        let mut root_ids = Vec::with_capacity(projects.len());
        for project in projects {
            root_ids.push(project.root_project_id.unwrap_or(project.id));
        }
        root_ids.sort_unstable();
        root_ids.dedup();

        let roots_by_id = projects_by_id(self.db.as_ref(), &root_ids).await?;
        for root_id in &root_ids {
            let root = roots_by_id
                .get(root_id)
                .ok_or_else(|| AppError::NotFound(format!("项目不存在: {root_id}")))?;
            validate_collaboration_root(root)?;
        }

        let members = project_member::Entity::find()
            .filter(project_member::Column::ProjectId.is_in(root_ids.iter().copied()))
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let mut member_counts: HashMap<i64, u64> = HashMap::new();
        let mut member_roles: HashMap<i64, ProjectAccessRole> = HashMap::new();
        for member in members {
            *member_counts.entry(member.project_id).or_insert(0) += 1;
            if member.user_id == user_id {
                member_roles.insert(
                    member.project_id,
                    match parse_project_member_role(&member.role)? {
                        ProjectMemberRole::Manager => ProjectAccessRole::Manager,
                        ProjectMemberRole::Editor => ProjectAccessRole::Editor,
                        ProjectMemberRole::Viewer => ProjectAccessRole::Viewer,
                    },
                );
            }
        }

        let owner_ids = roots_by_id
            .values()
            .map(|root| root.user_id)
            .collect::<Vec<_>>();
        let owners_by_id = users_by_id(self.db.as_ref(), &owner_ids).await?;

        let mut role_by_root = HashMap::with_capacity(root_ids.len());
        let mut manager_root_ids = Vec::new();
        for root_id in &root_ids {
            let root = roots_by_id.get(root_id).expect("root was validated above");
            let mut role = if root.user_id == user_id {
                Some(ProjectAccessRole::Owner)
            } else {
                member_roles.get(root_id).copied()
            };
            if user.role == ROLE_READ_ONLY && role.is_some() {
                role = Some(ProjectAccessRole::Viewer);
            }
            if role
                .map(|role| role.satisfies(ProjectAccessRole::Manager))
                .unwrap_or(false)
            {
                manager_root_ids.push(*root_id);
            }
            role_by_root.insert(*root_id, role);
        }

        let pending_invite_counts =
            pending_invite_counts_by_root(self.db.as_ref(), &manager_root_ids).await?;
        let mut infos = HashMap::with_capacity(projects.len());
        for project in projects {
            let root_id = project.root_project_id.unwrap_or(project.id);
            let root = roots_by_id.get(&root_id).expect("root was validated above");
            let role = role_by_root.get(&root_id).copied().flatten();
            infos.insert(
                project.id,
                ProjectAccessInfo {
                    collaboration_root_project_id: root.id,
                    owner_user_id: root.user_id,
                    owner_username: owners_by_id
                        .get(&root.user_id)
                        .map(|owner| owner.username.clone()),
                    my_role: role,
                    member_count: member_counts.get(&root.id).copied().unwrap_or(0),
                    pending_invite_count: if role
                        .map(|role| role.satisfies(ProjectAccessRole::Manager))
                        .unwrap_or(false)
                    {
                        Some(pending_invite_counts.get(&root.id).copied().unwrap_or(0))
                    } else {
                        None
                    },
                },
            );
        }
        Ok(infos)
    }

    pub async fn verify_permission(
        &self,
        project_id: i64,
        user_id: i64,
        required: ProjectAccessRole,
    ) -> Result<ProjectAccessRole, AppError> {
        let root = self.resolve_root_project(project_id).await?;
        let Some(role) = self.effective_role_for_root(&root, user_id).await? else {
            return Err(AppError::Forbidden("无权访问该项目".to_string()));
        };
        if !role.satisfies(required) {
            return Err(AppError::Forbidden(format!(
                "项目权限不足，需要 {} 权限",
                required.as_str()
            )));
        }
        Ok(role)
    }

    pub async fn verify_permissions_for_projects(
        &self,
        project_ids: &[i64],
        user_id: i64,
        required: ProjectAccessRole,
    ) -> Result<(), AppError> {
        if project_ids.is_empty() {
            return Ok(());
        }
        let mut normalized_project_ids = Vec::with_capacity(project_ids.len());
        for project_id in project_ids {
            if *project_id <= 0 {
                return Err(AppError::InvalidArgument(
                    "project_id 必须为正整数".to_string(),
                ));
            }
            normalized_project_ids.push(*project_id);
        }
        normalized_project_ids.sort_unstable();
        normalized_project_ids.dedup();

        let projects_by_id_map = projects_by_id(self.db.as_ref(), &normalized_project_ids).await?;
        for project_id in &normalized_project_ids {
            if !projects_by_id_map.contains_key(project_id) {
                return Err(AppError::NotFound(format!("项目不存在: {project_id}")));
            }
        }

        let mut root_ids = Vec::with_capacity(normalized_project_ids.len());
        for project_id in &normalized_project_ids {
            let project = projects_by_id_map
                .get(project_id)
                .expect("project existence checked above");
            root_ids.push(project.root_project_id.unwrap_or(project.id));
        }
        root_ids.sort_unstable();
        root_ids.dedup();

        let roots_by_id = projects_by_id(self.db.as_ref(), &root_ids).await?;
        for root_id in &root_ids {
            let root = roots_by_id
                .get(root_id)
                .ok_or_else(|| AppError::NotFound(format!("项目不存在: {root_id}")))?;
            validate_collaboration_root(root)?;
        }

        let user = find_user(self.db.as_ref(), user_id).await?;
        if !user.is_active {
            return Err(AppError::Forbidden("账户已被禁用".to_string()));
        }

        let members = project_member::Entity::find()
            .filter(project_member::Column::ProjectId.is_in(root_ids.iter().copied()))
            .filter(project_member::Column::UserId.eq(user_id))
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let mut member_roles = HashMap::with_capacity(members.len());
        for member in members {
            member_roles.insert(
                member.project_id,
                match parse_project_member_role(&member.role)? {
                    ProjectMemberRole::Manager => ProjectAccessRole::Manager,
                    ProjectMemberRole::Editor => ProjectAccessRole::Editor,
                    ProjectMemberRole::Viewer => ProjectAccessRole::Viewer,
                },
            );
        }

        for project_id in &normalized_project_ids {
            let project = projects_by_id_map
                .get(project_id)
                .expect("project existence checked above");
            let root_id = project.root_project_id.unwrap_or(project.id);
            let root = roots_by_id
                .get(&root_id)
                .expect("root existence checked above");
            let mut role = if root.user_id == user_id {
                Some(ProjectAccessRole::Owner)
            } else {
                member_roles.get(&root.id).copied()
            };
            if user.role == ROLE_READ_ONLY && role.is_some() {
                role = Some(ProjectAccessRole::Viewer);
            }
            let Some(role) = role else {
                return Err(AppError::Forbidden("无权访问该项目".to_string()));
            };
            if !role.satisfies(required) {
                return Err(AppError::Forbidden(format!(
                    "项目权限不足，需要 {} 权限",
                    required.as_str()
                )));
            }
        }
        Ok(())
    }

    pub async fn list_accessible_projects(
        &self,
        user_id: i64,
        offset: u64,
        limit: u64,
        filter: AccessibleProjectListFilter<'_>,
    ) -> Result<(Vec<novel_project::Model>, u64), AppError> {
        let member_root_ids = self.member_root_ids(user_id).await?;
        let mut condition = match filter.access_scope {
            ProjectAccessScope::Owned => {
                Condition::all().add(novel_project::Column::UserId.eq(user_id))
            }
            ProjectAccessScope::Collaborating => accessible_member_condition(&member_root_ids),
            ProjectAccessScope::All => {
                let mut condition = Condition::any().add(novel_project::Column::UserId.eq(user_id));
                if !member_root_ids.is_empty() {
                    condition = condition
                        .add(novel_project::Column::Id.is_in(member_root_ids.clone()))
                        .add(novel_project::Column::RootProjectId.is_in(member_root_ids.clone()));
                }
                Condition::all().add(condition)
            }
        };
        if let Some(kind) = filter.project_kind {
            condition =
                condition.add(novel_project::Column::ProjectKind.eq(normalize_project_kind(kind)?));
        }
        if let Some(root_project_id) = filter.root_project_id {
            if root_project_id <= 0 {
                return Err(AppError::InvalidArgument(
                    "root_project_id 必须为正整数".to_string(),
                ));
            }
            condition = condition.add(novel_project::Column::RootProjectId.eq(root_project_id));
        }

        let paginator = novel_project::Entity::find()
            .filter(condition)
            .order_by_desc(novel_project::Column::UpdatedAt)
            .paginate(self.db.as_ref(), limit);
        let total = paginator.num_items().await.map_err(AppError::Database)?;
        let items = paginator
            .fetch_page(offset / limit)
            .await
            .map_err(AppError::Database)?;
        Ok((items, total))
    }

    pub async fn list_project_members(
        &self,
        actor_user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ProjectMemberSummary>, AppError> {
        self.verify_permission(project_id, actor_user_id, ProjectAccessRole::Viewer)
            .await?;
        let root = self.resolve_root_project(project_id).await?;
        let rows = project_member::Entity::find()
            .filter(project_member::Column::ProjectId.eq(root.id))
            .order_by_asc(project_member::Column::CreatedAt)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        let user_ids = unique_member_user_ids(root.user_id, &rows);
        let users_by_id = users_by_id(self.db.as_ref(), &user_ids).await?;
        let owner = users_by_id
            .get(&root.user_id)
            .ok_or_else(|| AppError::NotFound(format!("用户不存在: {}", root.user_id)))?;
        let mut members = vec![ProjectMemberSummary {
            project_id: root.id,
            user_id: owner.id,
            username: owner.username.clone(),
            email: owner.email.clone(),
            role: ProjectAccessRole::Owner.as_str().to_string(),
            is_owner: true,
            invited_by_user_id: None,
            created_at: Some(root.created_at.to_string()),
            updated_at: Some(root.updated_at.to_string()),
        }];

        for row in rows {
            if let Some(user) = users_by_id.get(&row.user_id) {
                members.push(ProjectMemberSummary {
                    project_id: root.id,
                    user_id: row.user_id,
                    username: user.username.clone(),
                    email: user.email.clone(),
                    role: row.role,
                    is_owner: false,
                    invited_by_user_id: Some(row.invited_by_user_id),
                    created_at: Some(row.created_at.to_string()),
                    updated_at: Some(row.updated_at.to_string()),
                });
            }
        }
        Ok(members)
    }

    pub async fn invite_project_member(
        &self,
        inviter_user_id: i64,
        project_id: i64,
        username_or_email: &str,
        role: ProjectMemberRole,
    ) -> Result<ProjectInviteSummary, AppError> {
        let inviter_role = self
            .verify_permission(project_id, inviter_user_id, ProjectAccessRole::Manager)
            .await?;
        if role == ProjectMemberRole::Manager && inviter_role != ProjectAccessRole::Owner {
            return Err(AppError::Forbidden(
                "只有项目所有者可以邀请 manager".to_string(),
            ));
        }
        let root = self.resolve_root_project(project_id).await?;
        let invitee = find_user_by_identity(self.db.as_ref(), username_or_email).await?;
        if !invitee.is_active {
            return Err(AppError::Forbidden("被邀请用户已被禁用".to_string()));
        }
        if invitee.id == inviter_user_id {
            return Err(AppError::InvalidArgument("不能邀请自己".to_string()));
        }
        if invitee.id == root.user_id {
            return Err(AppError::InvalidArgument(
                "项目所有者不需要邀请".to_string(),
            ));
        }
        if project_member::Entity::find()
            .filter(project_member::Column::ProjectId.eq(root.id))
            .filter(project_member::Column::UserId.eq(invitee.id))
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .is_some()
        {
            return Err(AppError::InvalidArgument(
                "该用户已是项目协作者".to_string(),
            ));
        }

        self.expire_project_invites(root.id).await?;
        if project_invite::Entity::find()
            .filter(project_invite::Column::ProjectId.eq(root.id))
            .filter(project_invite::Column::InviteeUserId.eq(invitee.id))
            .filter(project_invite::Column::Status.eq(INVITE_STATUS_PENDING))
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .is_some()
        {
            return Err(AppError::InvalidArgument(
                "该用户已有待处理邀请".to_string(),
            ));
        }

        let now = Utc::now().naive_utc();
        let invite = project_invite::ActiveModel {
            project_id: Set(root.id),
            inviter_user_id: Set(inviter_user_id),
            invitee_user_id: Set(invitee.id),
            role: Set(role.as_str().to_string()),
            status: Set(INVITE_STATUS_PENDING.to_string()),
            expires_at: Set(now + Duration::days(INVITE_TTL_DAYS)),
            accepted_at: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        }
        .insert(self.db.as_ref())
        .await
        .map_err(AppError::Database)?;
        self.invite_summary(invite).await
    }

    pub async fn list_project_invites(
        &self,
        actor_user_id: i64,
        project_id: i64,
    ) -> Result<Vec<ProjectInviteSummary>, AppError> {
        self.verify_permission(project_id, actor_user_id, ProjectAccessRole::Manager)
            .await?;
        let root = self.resolve_root_project(project_id).await?;
        self.expire_project_invites(root.id).await?;
        let invites = project_invite::Entity::find()
            .filter(project_invite::Column::ProjectId.eq(root.id))
            .order_by_desc(project_invite::Column::CreatedAt)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        self.invite_summaries(invites).await
    }

    pub async fn list_my_project_invites(
        &self,
        user_id: i64,
    ) -> Result<Vec<ProjectInviteSummary>, AppError> {
        self.expire_user_invites(user_id).await?;
        let invites = project_invite::Entity::find()
            .filter(project_invite::Column::InviteeUserId.eq(user_id))
            .filter(project_invite::Column::Status.eq(INVITE_STATUS_PENDING))
            .filter(project_invite::Column::ExpiresAt.gt(Utc::now().naive_utc()))
            .order_by_desc(project_invite::Column::CreatedAt)
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        self.invite_summaries(invites).await
    }

    pub async fn accept_project_invite(
        &self,
        user_id: i64,
        invite_id: i64,
    ) -> Result<ProjectMemberSummary, AppError> {
        let tx = self.db.begin().await.map_err(AppError::Database)?;
        let invite = find_invite(&tx, invite_id).await?;
        if invite.invitee_user_id != user_id {
            return Err(AppError::Forbidden("只能处理自己的项目邀请".to_string()));
        }
        if invite.status != INVITE_STATUS_PENDING {
            return Err(AppError::InvalidArgument("邀请已不可接受".to_string()));
        }
        let now = Utc::now().naive_utc();
        if invite.expires_at <= now {
            project_invite::Entity::update_many()
                .col_expr(
                    project_invite::Column::Status,
                    Expr::value(INVITE_STATUS_EXPIRED),
                )
                .col_expr(project_invite::Column::UpdatedAt, Expr::value(now))
                .filter(project_invite::Column::Id.eq(invite.id))
                .exec(&tx)
                .await
                .map_err(AppError::Database)?;
            tx.commit().await.map_err(AppError::Database)?;
            return Err(AppError::InvalidArgument("邀请已过期".to_string()));
        }
        let root = find_project(&tx, invite.project_id).await?;
        if root.user_id == user_id {
            return Err(AppError::InvalidArgument(
                "项目所有者不需要接受邀请".to_string(),
            ));
        }

        let member_active = project_member::ActiveModel {
            project_id: Set(invite.project_id),
            user_id: Set(user_id),
            role: Set(invite.role.clone()),
            invited_by_user_id: Set(invite.inviter_user_id),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };
        project_member::Entity::insert(member_active)
            .on_conflict(
                OnConflict::columns([
                    project_member::Column::ProjectId,
                    project_member::Column::UserId,
                ])
                .update_columns([
                    project_member::Column::Role,
                    project_member::Column::InvitedByUserId,
                    project_member::Column::UpdatedAt,
                ])
                .to_owned(),
            )
            .exec(&tx)
            .await
            .map_err(AppError::Database)?;
        project_invite::Entity::update_many()
            .col_expr(
                project_invite::Column::Status,
                Expr::value(INVITE_STATUS_ACCEPTED),
            )
            .col_expr(project_invite::Column::AcceptedAt, Expr::value(Some(now)))
            .col_expr(project_invite::Column::UpdatedAt, Expr::value(now))
            .filter(project_invite::Column::Id.eq(invite.id))
            .filter(project_invite::Column::Status.eq(INVITE_STATUS_PENDING))
            .exec(&tx)
            .await
            .map_err(AppError::Database)?;
        tx.commit().await.map_err(AppError::Database)?;

        let member = project_member::Entity::find()
            .filter(project_member::Column::ProjectId.eq(invite.project_id))
            .filter(project_member::Column::UserId.eq(user_id))
            .one(self.db.as_ref())
            .await
            .map_err(AppError::Database)?
            .ok_or_else(|| AppError::Internal(anyhow::anyhow!("协作成员写入后未找到")))?;
        self.member_summary(member).await
    }

    pub async fn decline_project_invite(
        &self,
        user_id: i64,
        invite_id: i64,
    ) -> Result<ProjectInviteSummary, AppError> {
        let invite = find_invite(self.db.as_ref(), invite_id).await?;
        if invite.invitee_user_id != user_id {
            return Err(AppError::Forbidden("只能处理自己的项目邀请".to_string()));
        }
        let updated = self
            .update_invite_status(invite_id, INVITE_STATUS_DECLINED, None)
            .await?;
        self.invite_summary(updated).await
    }

    pub async fn revoke_project_invite(
        &self,
        actor_user_id: i64,
        project_id: i64,
        invite_id: i64,
    ) -> Result<ProjectInviteSummary, AppError> {
        self.verify_permission(project_id, actor_user_id, ProjectAccessRole::Manager)
            .await?;
        let root = self.resolve_root_project(project_id).await?;
        let invite = find_invite(self.db.as_ref(), invite_id).await?;
        if invite.project_id != root.id {
            return Err(AppError::InvalidArgument("邀请不属于该项目".to_string()));
        }
        let updated = self
            .update_invite_status(invite_id, INVITE_STATUS_REVOKED, None)
            .await?;
        self.invite_summary(updated).await
    }

    pub async fn update_project_member_role(
        &self,
        actor_user_id: i64,
        project_id: i64,
        member_user_id: i64,
        role: ProjectMemberRole,
    ) -> Result<ProjectMemberSummary, AppError> {
        let actor_role = self
            .verify_permission(project_id, actor_user_id, ProjectAccessRole::Manager)
            .await?;
        let root = self.resolve_root_project(project_id).await?;
        if member_user_id == root.user_id {
            return Err(AppError::InvalidArgument(
                "不能修改项目所有者角色".to_string(),
            ));
        }
        let member = find_member(self.db.as_ref(), root.id, member_user_id).await?;
        let current_role = parse_project_member_role(&member.role)?;
        if actor_role != ProjectAccessRole::Owner
            && (current_role == ProjectMemberRole::Manager || role == ProjectMemberRole::Manager)
        {
            return Err(AppError::Forbidden(
                "manager 不能管理其他 manager 或提升 manager".to_string(),
            ));
        }
        let now = Utc::now().naive_utc();
        let mut active: project_member::ActiveModel = member.into();
        active.role = Set(role.as_str().to_string());
        active.updated_at = Set(now);
        let updated = active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        self.member_summary(updated).await
    }

    pub async fn remove_project_member(
        &self,
        actor_user_id: i64,
        project_id: i64,
        member_user_id: i64,
    ) -> Result<(), AppError> {
        let actor_role = self
            .verify_permission(project_id, actor_user_id, ProjectAccessRole::Manager)
            .await?;
        let root = self.resolve_root_project(project_id).await?;
        if member_user_id == root.user_id {
            return Err(AppError::InvalidArgument("不能移除项目所有者".to_string()));
        }
        let member = find_member(self.db.as_ref(), root.id, member_user_id).await?;
        let current_role = parse_project_member_role(&member.role)?;
        if actor_role != ProjectAccessRole::Owner && current_role == ProjectMemberRole::Manager {
            return Err(AppError::Forbidden(
                "manager 不能移除其他 manager".to_string(),
            ));
        }
        project_member::Entity::delete_by_id(member.id)
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }

    pub async fn leave_project(&self, user_id: i64, project_id: i64) -> Result<(), AppError> {
        let root = self.resolve_root_project(project_id).await?;
        if root.user_id == user_id {
            return Err(AppError::InvalidArgument(
                "项目所有者不能离开自己的项目".to_string(),
            ));
        }
        project_member::Entity::delete_many()
            .filter(project_member::Column::ProjectId.eq(root.id))
            .filter(project_member::Column::UserId.eq(user_id))
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }

    async fn effective_role_for_root(
        &self,
        root: &novel_project::Model,
        user_id: i64,
    ) -> Result<Option<ProjectAccessRole>, AppError> {
        let user = find_user(self.db.as_ref(), user_id).await?;
        if !user.is_active {
            return Err(AppError::Forbidden("账户已被禁用".to_string()));
        }

        let mut role = if root.user_id == user_id {
            Some(ProjectAccessRole::Owner)
        } else {
            let member = project_member::Entity::find()
                .filter(project_member::Column::ProjectId.eq(root.id))
                .filter(project_member::Column::UserId.eq(user_id))
                .one(self.db.as_ref())
                .await
                .map_err(AppError::Database)?;
            match member {
                Some(member) => Some(match parse_project_member_role(&member.role)? {
                    ProjectMemberRole::Manager => ProjectAccessRole::Manager,
                    ProjectMemberRole::Editor => ProjectAccessRole::Editor,
                    ProjectMemberRole::Viewer => ProjectAccessRole::Viewer,
                }),
                None => None,
            }
        };

        if user.role == ROLE_READ_ONLY && role.is_some() {
            role = Some(ProjectAccessRole::Viewer);
        }
        Ok(role)
    }

    async fn member_root_ids(&self, user_id: i64) -> Result<Vec<i64>, AppError> {
        let members = project_member::Entity::find()
            .filter(project_member::Column::UserId.eq(user_id))
            .all(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        Ok(members
            .into_iter()
            .map(|member| member.project_id)
            .collect())
    }

    async fn expire_project_invites(&self, project_id: i64) -> Result<(), AppError> {
        project_invite::Entity::update_many()
            .col_expr(
                project_invite::Column::Status,
                Expr::value(INVITE_STATUS_EXPIRED),
            )
            .col_expr(
                project_invite::Column::UpdatedAt,
                Expr::value(Utc::now().naive_utc()),
            )
            .filter(project_invite::Column::ProjectId.eq(project_id))
            .filter(project_invite::Column::Status.eq(INVITE_STATUS_PENDING))
            .filter(project_invite::Column::ExpiresAt.lte(Utc::now().naive_utc()))
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }

    async fn expire_user_invites(&self, user_id: i64) -> Result<(), AppError> {
        project_invite::Entity::update_many()
            .col_expr(
                project_invite::Column::Status,
                Expr::value(INVITE_STATUS_EXPIRED),
            )
            .col_expr(
                project_invite::Column::UpdatedAt,
                Expr::value(Utc::now().naive_utc()),
            )
            .filter(project_invite::Column::InviteeUserId.eq(user_id))
            .filter(project_invite::Column::Status.eq(INVITE_STATUS_PENDING))
            .filter(project_invite::Column::ExpiresAt.lte(Utc::now().naive_utc()))
            .exec(self.db.as_ref())
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }

    async fn update_invite_status(
        &self,
        invite_id: i64,
        status: &str,
        accepted_at: Option<chrono::NaiveDateTime>,
    ) -> Result<project_invite::Model, AppError> {
        let invite = find_invite(self.db.as_ref(), invite_id).await?;
        if invite.status != INVITE_STATUS_PENDING {
            return Err(AppError::InvalidArgument("邀请已不可处理".to_string()));
        }
        let now = Utc::now().naive_utc();
        let mut active: project_invite::ActiveModel = invite.into();
        active.status = Set(status.to_string());
        active.accepted_at = Set(accepted_at);
        active.updated_at = Set(now);
        active
            .update(self.db.as_ref())
            .await
            .map_err(AppError::Database)
    }

    async fn member_summary(
        &self,
        member: project_member::Model,
    ) -> Result<ProjectMemberSummary, AppError> {
        let user = find_user(self.db.as_ref(), member.user_id).await?;
        Ok(ProjectMemberSummary {
            project_id: member.project_id,
            user_id: member.user_id,
            username: user.username,
            email: user.email,
            role: member.role,
            is_owner: false,
            invited_by_user_id: Some(member.invited_by_user_id),
            created_at: Some(member.created_at.to_string()),
            updated_at: Some(member.updated_at.to_string()),
        })
    }

    async fn invite_summaries(
        &self,
        invites: Vec<project_invite::Model>,
    ) -> Result<Vec<ProjectInviteSummary>, AppError> {
        if invites.is_empty() {
            return Ok(Vec::new());
        }
        let user_ids = unique_invite_user_ids(&invites);
        let project_ids = unique_invite_project_ids(&invites);
        let users_by_id = users_by_id(self.db.as_ref(), &user_ids).await?;
        let projects_by_id = projects_by_id(self.db.as_ref(), &project_ids).await?;
        let mut summaries = Vec::with_capacity(invites.len());
        for invite in invites {
            summaries.push(invite_summary_from_indexes(
                invite,
                &users_by_id,
                &projects_by_id,
            ));
        }
        Ok(summaries)
    }

    async fn invite_summary(
        &self,
        invite: project_invite::Model,
    ) -> Result<ProjectInviteSummary, AppError> {
        let inviter = find_user(self.db.as_ref(), invite.inviter_user_id)
            .await
            .ok();
        let invitee = find_user(self.db.as_ref(), invite.invitee_user_id)
            .await
            .ok();
        let project = find_project(self.db.as_ref(), invite.project_id).await.ok();
        Ok(ProjectInviteSummary {
            id: invite.id,
            project_id: invite.project_id,
            project_title: project.map(|project| project.title),
            inviter_user_id: invite.inviter_user_id,
            inviter_username: inviter.map(|user| user.username),
            invitee_user_id: invite.invitee_user_id,
            invitee_username: invitee.as_ref().map(|user| user.username.clone()),
            invitee_email: invitee.map(|user| user.email),
            role: invite.role,
            status: invite.status,
            expires_at: invite.expires_at.to_string(),
            accepted_at: invite.accepted_at.map(|time| time.to_string()),
            created_at: Some(invite.created_at.to_string()),
            updated_at: Some(invite.updated_at.to_string()),
        })
    }
}

fn accessible_member_condition(root_ids: &[i64]) -> Condition {
    if root_ids.is_empty() {
        Condition::all().add(novel_project::Column::Id.eq(-1))
    } else {
        Condition::any()
            .add(novel_project::Column::Id.is_in(root_ids.to_vec()))
            .add(novel_project::Column::RootProjectId.is_in(root_ids.to_vec()))
    }
}

fn unique_member_user_ids(owner_user_id: i64, members: &[project_member::Model]) -> Vec<i64> {
    let mut ids = Vec::with_capacity(members.len() + 1);
    ids.push(owner_user_id);
    for member in members {
        ids.push(member.user_id);
    }
    ids.sort_unstable();
    ids.dedup();
    ids
}

fn unique_invite_user_ids(invites: &[project_invite::Model]) -> Vec<i64> {
    let mut ids = Vec::with_capacity(invites.len() * 2);
    for invite in invites {
        ids.push(invite.inviter_user_id);
        ids.push(invite.invitee_user_id);
    }
    ids.sort_unstable();
    ids.dedup();
    ids
}

fn unique_invite_project_ids(invites: &[project_invite::Model]) -> Vec<i64> {
    let mut ids = Vec::with_capacity(invites.len());
    for invite in invites {
        ids.push(invite.project_id);
    }
    ids.sort_unstable();
    ids.dedup();
    ids
}

async fn users_by_id<C>(db: &C, user_ids: &[i64]) -> Result<HashMap<i64, user::Model>, AppError>
where
    C: ConnectionTrait,
{
    if user_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = user::Entity::find()
        .filter(user::Column::Id.is_in(user_ids.iter().copied()))
        .all(db)
        .await
        .map_err(AppError::Database)?;
    Ok(rows.into_iter().map(|row| (row.id, row)).collect())
}

async fn projects_by_id<C>(
    db: &C,
    project_ids: &[i64],
) -> Result<HashMap<i64, novel_project::Model>, AppError>
where
    C: ConnectionTrait,
{
    if project_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let rows = novel_project::Entity::find()
        .filter(novel_project::Column::Id.is_in(project_ids.iter().copied()))
        .all(db)
        .await
        .map_err(AppError::Database)?;
    Ok(rows.into_iter().map(|row| (row.id, row)).collect())
}

async fn pending_invite_counts_by_root<C>(
    db: &C,
    root_ids: &[i64],
) -> Result<HashMap<i64, u64>, AppError>
where
    C: ConnectionTrait,
{
    if root_ids.is_empty() {
        return Ok(HashMap::new());
    }
    let invites = project_invite::Entity::find()
        .filter(project_invite::Column::ProjectId.is_in(root_ids.iter().copied()))
        .filter(project_invite::Column::Status.eq(INVITE_STATUS_PENDING))
        .filter(project_invite::Column::ExpiresAt.gt(Utc::now().naive_utc()))
        .all(db)
        .await
        .map_err(AppError::Database)?;
    let mut counts = HashMap::new();
    for invite in invites {
        *counts.entry(invite.project_id).or_insert(0) += 1;
    }
    Ok(counts)
}

fn validate_collaboration_root(root: &novel_project::Model) -> Result<(), AppError> {
    if root.project_kind != PROJECT_KIND_TEXT
        && !(root.project_kind == "video" && root.source_mode == SOURCE_MODE_FREEFORM)
    {
        return Err(AppError::InvalidArgument(
            "协作权限只能绑定到文本根项目或自由视频项目".to_string(),
        ));
    }
    if root.project_kind == PROJECT_KIND_TEXT && root.source_mode != SOURCE_MODE_SELF {
        return Err(AppError::InvalidArgument(
            "文本协作根项目 source_mode 必须为 self".to_string(),
        ));
    }
    Ok(())
}

fn invite_summary_from_indexes(
    invite: project_invite::Model,
    users_by_id: &HashMap<i64, user::Model>,
    projects_by_id: &HashMap<i64, novel_project::Model>,
) -> ProjectInviteSummary {
    let inviter = users_by_id.get(&invite.inviter_user_id);
    let invitee = users_by_id.get(&invite.invitee_user_id);
    let project = projects_by_id.get(&invite.project_id);
    ProjectInviteSummary {
        id: invite.id,
        project_id: invite.project_id,
        project_title: project.map(|project| project.title.clone()),
        inviter_user_id: invite.inviter_user_id,
        inviter_username: inviter.map(|user| user.username.clone()),
        invitee_user_id: invite.invitee_user_id,
        invitee_username: invitee.map(|user| user.username.clone()),
        invitee_email: invitee.map(|user| user.email.clone()),
        role: invite.role,
        status: invite.status,
        expires_at: invite.expires_at.to_string(),
        accepted_at: invite.accepted_at.map(|time| time.to_string()),
        created_at: Some(invite.created_at.to_string()),
        updated_at: Some(invite.updated_at.to_string()),
    }
}

async fn find_project<C>(db: &C, project_id: i64) -> Result<novel_project::Model, AppError>
where
    C: ConnectionTrait,
{
    novel_project::Entity::find_by_id(project_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("项目不存在: {project_id}")))
}

async fn find_user<C>(db: &C, user_id: i64) -> Result<user::Model, AppError>
where
    C: ConnectionTrait,
{
    user::Entity::find_by_id(user_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("用户不存在: {user_id}")))
}

async fn find_user_by_identity<C>(db: &C, value: &str) -> Result<user::Model, AppError>
where
    C: ConnectionTrait,
{
    let value = value.trim();
    if value.is_empty() {
        return Err(AppError::InvalidArgument("邀请对象不能为空".to_string()));
    }
    user::Entity::find()
        .filter(
            Condition::any()
                .add(user::Column::Username.eq(value))
                .add(user::Column::Email.eq(value)),
        )
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("用户不存在: {value}")))
}

async fn find_invite<C>(db: &C, invite_id: i64) -> Result<project_invite::Model, AppError>
where
    C: ConnectionTrait,
{
    project_invite::Entity::find_by_id(invite_id)
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound(format!("项目邀请不存在: {invite_id}")))
}

async fn find_member<C>(
    db: &C,
    project_id: i64,
    user_id: i64,
) -> Result<project_member::Model, AppError>
where
    C: ConnectionTrait,
{
    project_member::Entity::find()
        .filter(project_member::Column::ProjectId.eq(project_id))
        .filter(project_member::Column::UserId.eq(user_id))
        .one(db)
        .await
        .map_err(AppError::Database)?
        .ok_or_else(|| AppError::NotFound("项目协作者不存在".to_string()))
}
