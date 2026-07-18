use super::*;

#[test]
fn validate_temp_media_file_name_accepts_uuid_images() {
    let name = "0123456789abcdef0123456789abcdef.png";
    assert_eq!(validate_temp_media_file_name(name), Ok(name.to_string()));
}

#[test]
fn validate_temp_media_file_name_rejects_paths_and_non_uuid_names() {
    for value in [
        "../x.png",
        "nested/x.png",
        "not-a-uuid.png",
        "0123456789abcdef0123456789abcdef.exe",
    ] {
        assert!(
            validate_temp_media_file_name(value).is_err(),
            "{value} should be rejected"
        );
    }
}

#[test]
fn validate_image_magic_rejects_mime_mismatch() {
    assert!(validate_image_magic("image/png", b"\x89PNG\r\n\x1a\nabc").is_ok());
    assert!(validate_image_magic("image/jpeg", b"\xFF\xD8\xFFabc").is_ok());
    assert!(validate_image_magic("image/webp", b"RIFFxxxxWEBPabc").is_ok());
    assert!(validate_image_magic("image/png", b"\xFF\xD8\xFFabc").is_err());
}

#[test]
fn validate_upload_file_name_hint_rejects_paths_and_mime_mismatch() {
    assert!(validate_upload_file_name_hint("local.png", "png").is_ok());
    assert!(validate_upload_file_name_hint("local.jpeg", "jpg").is_ok());
    assert!(validate_upload_file_name_hint("../local.png", "png").is_err());
    assert!(validate_upload_file_name_hint("nested/local.png", "png").is_err());
    assert!(validate_upload_file_name_hint("nested\\local.png", "png").is_err());
    assert!(validate_upload_file_name_hint("local.webp", "png").is_err());
}

#[test]
fn temp_media_numeric_config_rejects_invalid_values_instead_of_defaulting() {
    assert_eq!(parse_positive_i64_config("TTL", None, 360).unwrap(), 360);
    assert!(parse_positive_i64_config("TTL", Some("0"), 360).is_err());
    assert!(parse_positive_i64_config("TTL", Some("invalid"), 360).is_err());
    assert_eq!(
        parse_positive_usize_config("MAX", Some("1024"), 20).unwrap(),
        1024
    );
    assert!(parse_positive_usize_config("MAX", Some("-1"), 20).is_err());
}

#[test]
fn temp_media_public_url_requires_http_without_credentials_or_query() {
    assert_eq!(
        normalize_temp_media_public_url("https://example.com/a4n/api/temp-media/").unwrap(),
        "https://example.com/a4n/api/temp-media"
    );
    assert!(normalize_temp_media_public_url("ftp://example.com/media").is_err());
    assert!(normalize_temp_media_public_url("https://user:secret@example.com/media").is_err());
    assert!(normalize_temp_media_public_url("https://example.com/media?token=x").is_err());
}
