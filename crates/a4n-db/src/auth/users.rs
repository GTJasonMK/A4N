mod account;
mod admin;
mod read;
mod registration;

pub use account::{change_password, login_user, update_profile};
pub use admin::{disable_user, update_user_role};
pub use read::{count_users, current_user, list_users, user_by_id};
pub use registration::{
    bootstrap_first_admin, create_user, register_user, require_existing_user_or_bootstrap_admin,
};

pub(in crate::auth) use read::{active_user, find_by_email_on, find_by_username_on};
pub(in crate::auth) use registration::create_user_on;
