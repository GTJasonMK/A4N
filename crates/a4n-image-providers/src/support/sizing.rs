use a4n_image::ImageGenerationRequest;

pub(super) fn grok_image_sizing_fields_required(model: &str) -> bool {
    model.trim().starts_with("grok-imagine-image")
}

pub(crate) fn image_resolution(width: i32, height: i32) -> &'static str {
    if width.max(height) >= 1536 {
        "2k"
    } else {
        "1k"
    }
}

pub(crate) fn image_aspect_ratio(width: i32, height: i32) -> Option<&'static str> {
    if width <= 0 || height <= 0 {
        return None;
    }
    if width == height {
        return Some("1:1");
    }
    let width = i64::from(width);
    let height = i64::from(height);
    if width * 9 == height * 16 {
        return Some("16:9");
    }
    if width * 16 == height * 9 {
        return Some("9:16");
    }
    if width * 2 == height * 3 {
        return Some("3:2");
    }
    if width * 3 == height * 2 {
        return Some("2:3");
    }
    None
}

pub(crate) fn grok_image_sizing_fields(
    request: &ImageGenerationRequest,
) -> Option<(&'static str, &'static str)> {
    if !grok_image_sizing_fields_required(&request.model) {
        return None;
    }
    image_aspect_ratio(request.width, request.height).map(|aspect_ratio| {
        (
            image_resolution(request.width, request.height),
            aspect_ratio,
        )
    })
}
