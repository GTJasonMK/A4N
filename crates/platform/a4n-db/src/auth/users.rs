mod account;
mod admin;
mod read;
mod registration;

pub use account::{change_password, login_user, update_profile};
#[cfg(test)]
pub use admin::disable_user;
pub(crate) use admin::lock_administrator_membership;
pub use admin::update_user_role;
pub use read::{
    current_user, current_user_summary, list_users, user_authorization_by_id, user_by_id,
    user_summary_by_id,
};
#[cfg(test)]
pub use registration::{bootstrap_first_admin, register_user};
pub use registration::{create_user, require_existing_user_or_bootstrap_admin};

pub(in crate::auth) use read::{active_user, find_by_email_on, find_by_username_on};
pub(in crate::auth) use registration::create_user_on;
