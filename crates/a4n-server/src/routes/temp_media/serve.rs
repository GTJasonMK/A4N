use axum::extract::{Path, State};
use axum::http::{HeaderMap, header};
use axum::response::Response;

use super::super::error::ApiError;
use super::super::media::file_response;
use super::rules::validate_temp_media_file_name;
use super::storage::{temp_media_dir, temp_media_file_expired};
use crate::state::ServerState;

pub(in crate::routes) async fn serve(
    State(state): State<ServerState>,
    headers: HeaderMap,
    Path(file_name): Path<String>,
) -> Result<Response, ApiError> {
    let file_name = validate_temp_media_file_name(&file_name).map_err(ApiError::bad_request)?;
    let path = temp_media_dir(&state).join(file_name);
    match temp_media_file_expired(&path).map_err(ApiError::bad_request)? {
        None => return Err(ApiError::not_found("临时媒体不存在或已过期")),
        Some(true) => {
            match std::fs::remove_file(&path) {
                Ok(()) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(ApiError::bad_request(format!(
                        "临时媒体已过期，但删除失败：{}：{error}",
                        path.display()
                    )));
                }
            }
            return Err(ApiError::not_found("临时媒体不存在或已过期"));
        }
        Some(false) => {}
    }
    file_response(&path, headers.get(header::RANGE), "临时媒体")
}
