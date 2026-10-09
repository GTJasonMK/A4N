use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub(super) fn unauthorized() -> Response {
    StatusCode::UNAUTHORIZED.into_response()
}
