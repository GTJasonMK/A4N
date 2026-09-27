mod flow;
mod provider;

pub use a4n_api_contract::auth::{
    OAuth2AuthorizeInput, OAuth2AvailableProvidersInput, OAuth2CallbackInput,
};
pub(crate) use flow::oauth2_login_user;
pub use flow::{oauth2_authorize_url, oauth2_available_providers};

#[cfg(test)]
mod tests;
