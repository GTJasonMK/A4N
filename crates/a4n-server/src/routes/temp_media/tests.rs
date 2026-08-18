use super::*;

#[test]
fn validate_temp_media_file_name_accepts_uuid_artifacts() {
    let name = "0123456789abcdef0123456789abcdef.png";
    assert_eq!(validate_temp_media_file_name(name), Ok(name.to_string()));
}

#[test]
fn validate_temp_media_file_name_rejects_paths_and_non_uuid_names() {
    for value in [
        "../x.png",
        "nested/x.png",
        "not-a-uuid.png",
        "0123456789abcdef0123456789abcdef.jpeg",
        "0123456789abcdef0123456789abcdef.exe",
    ] {
        assert!(
            validate_temp_media_file_name(value).is_err(),
            "{value} should be rejected"
        );
    }
}

#[test]
fn validate_temp_media_artifact_id_accepts_only_opaque_uuid_hex() {
    assert_eq!(
        validate_temp_media_artifact_id("ABCDEF0123456789ABCDEF0123456789"),
        Ok("abcdef0123456789abcdef0123456789".to_string())
    );
    for value in [
        "../0123456789abcdef0123456789abcdef",
        "0123456789abcdef0123456789abcdef.png",
        "not-a-uuid",
    ] {
        assert!(validate_temp_media_artifact_id(value).is_err(), "{value}");
    }
}

#[test]
fn validate_media_magic_rejects_mime_mismatch() {
    assert!(validate_media_magic("image/png", b"\x89PNG\r\n\x1a\nabc").is_ok());
    assert!(validate_media_magic("image/jpeg", b"\xFF\xD8\xFFabc").is_ok());
    assert!(validate_media_magic("image/webp", b"RIFFxxxxWEBPabc").is_ok());
    assert!(validate_media_magic("image/png", b"\xFF\xD8\xFFabc").is_err());
    assert!(validate_media_magic("video/mp4", b"\0\0\0\x18ftypisom").is_ok());
    assert!(validate_media_magic("video/webm", b"\x1A\x45\xDF\xA3abc").is_ok());
    assert!(validate_media_magic("audio/mpeg", b"ID3abc").is_ok());
    assert!(validate_media_magic("audio/wav", b"RIFFxxxxWAVEabc").is_ok());
    assert!(validate_media_magic("audio/ogg", b"OggSabc").is_ok());
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
fn temp_media_numeric_config_requires_positive_values() {
    assert_eq!(parse_positive_i64_config("TTL", "360").unwrap(), 360);
    assert!(parse_positive_i64_config("TTL", "0").is_err());
    assert!(parse_positive_i64_config("TTL", "invalid").is_err());
    assert_eq!(parse_positive_usize_config("MAX", "1024").unwrap(), 1024);
    assert!(parse_positive_usize_config("MAX", "-1").is_err());
}
