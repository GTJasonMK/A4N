mod flow;
mod provider;

pub use a4n_api_contract::auth::{OAuth2AuthorizeInput, OAuth2CallbackInput};
pub use flow::oauth2_authorize_url;
pub(crate) use flow::oauth2_login_user;

#[cfg(test)]
mod tests;
