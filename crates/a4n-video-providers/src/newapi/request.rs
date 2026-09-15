use a4n_video_sidecar_contract::VideoGenerationRequest;

pub(super) fn size_from_request(req: &VideoGenerationRequest) -> String {
    format!("{}x{}", req.width, req.height)
}

pub(super) fn aspect_ratio_from_size(width: i32, height: i32) -> Option<&'static str> {
    if width <= 0 || height <= 0 {
        return None;
    }
    if width == height {
        return Some("1:1");
    }
    if width * 9 == height * 16 {
        return Some("16:9");
    }
    if width * 16 == height * 9 {
        return Some("9:16");
    }
    None
}
