use a4n_media::{MediaFileResponseError, read_media_file_response, relative_media_uri_path};
use a4n_runtime::AppState;
use a4n_storage::canonicalize_path_in_data_dir;
use std::path::Path;
use tauri::http::{Request, Response, StatusCode, header};
use tauri::{Manager, Runtime, UriSchemeContext};

const MEDIA_LABEL: &str = "媒体文件";

pub fn handle_request<R: Runtime>(
    ctx: UriSchemeContext<'_, R>,
    request: Request<Vec<u8>>,
) -> Response<Vec<u8>> {
    let Some(state) = ctx.app_handle().try_state::<AppState>() else {
        return text_response(
            StatusCode::SERVICE_UNAVAILABLE,
            "应用状态尚未初始化，无法读取媒体文件",
        );
    };

    let path = match relative_media_uri_path(request.uri().path()) {
        Ok(path) => path,
        Err(err) => return text_response(StatusCode::BAD_REQUEST, &err),
    };
    let data_dir = state.inner().data_dir().to_path_buf();
    let path = data_dir.join(path);
    let path = match canonicalize_path_in_data_dir(&data_dir, &path, MEDIA_LABEL) {
        Ok(path) => path,
        Err(err) if err.is_outside_data_directory() => {
            return text_response(StatusCode::FORBIDDEN, &err.to_string());
        }
        Err(err) => return text_response(StatusCode::NOT_FOUND, &err.to_string()),
    };

    match file_response(&path, request.headers().get(header::RANGE)) {
        Ok(response) => response,
        Err((status, message)) => text_response(status, &message),
    }
}

fn file_response(
    path: &Path,
    range_header: Option<&header::HeaderValue>,
) -> Result<Response<Vec<u8>>, (StatusCode, String)> {
    let media = read_media_file_response(
        path,
        range_header.map(header::HeaderValue::to_str),
        MEDIA_LABEL,
    )
    .map_err(|error| {
        let status = match &error {
            MediaFileResponseError::UnsupportedMediaType(_) => StatusCode::BAD_REQUEST,
            MediaFileResponseError::Read(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (status, error.to_string())
    })?;
    let mut response = Response::builder()
        .status(
            StatusCode::from_u16(media.status.as_u16())
                .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))?,
        )
        .header(header::CONTENT_TYPE, media.content_type)
        .header(header::ACCEPT_RANGES, "bytes");
    if let Some(content_length) = media.content_length {
        response = response.header(header::CONTENT_LENGTH, content_length.to_string());
    }
    if let Some(content_range) = media.content_range {
        response = response.header(header::CONTENT_RANGE, content_range);
    }
    response
        .body(media.body)
        .map_err(|err| (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()))
}

fn text_response(status: StatusCode, message: &str) -> Response<Vec<u8>> {
    Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "text/plain; charset=utf-8")
        .body(message.as_bytes().to_vec())
        .expect("text response should be valid")
}
