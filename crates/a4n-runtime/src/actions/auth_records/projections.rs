use super::contracts::{SessionSummary, UserSummary};
use a4n_core::models::user::{user, user_session};

pub(crate) fn to_user_summary(user: &user::Model) -> UserSummary {
    UserSummary {
        id: user.id,
        username: user.username.clone(),
        email: user.email.clone(),
        role: user.role.clone(),
        oauth_provider: user.oauth_provider.clone(),
        oauth_subject: user.oauth_subject.clone(),
        avatar_url: user.avatar_url.clone(),
        is_active: user.is_active,
        created_at: Some(user.created_at.to_string()),
        updated_at: Some(user.updated_at.to_string()),
        last_login_at: user.last_login_at.map(|date| date.to_string()),
    }
}

pub(super) fn to_session_summary(session: &user_session::Model) -> SessionSummary {
    SessionSummary {
        id: session.id.clone(),
        user_agent: session.user_agent.clone(),
        created_at: session.created_at.to_string(),
        expires_at: session.expires_at.to_string(),
        last_used_at: session.last_used_at.map(|date| date.to_string()),
    }
}
