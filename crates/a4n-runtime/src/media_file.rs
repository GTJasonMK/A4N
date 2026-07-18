mod path;
mod range;
mod response;

pub use path::{relative_media_path, relative_media_uri_path};
pub use range::{requested_byte_range, requested_byte_range_header};
pub use response::{
    MediaFileResponse, MediaResponseStatus, content_type_for_path, read_media_file_response,
};

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    #[test]
    fn relative_media_uri_path_decodes_convert_file_src_path() {
        let path = relative_media_uri_path("/projects%2F1%2Fimages%2Fchapter_7%2Fimage.png")
            .expect("path should decode");

        assert_eq!(path, PathBuf::from("projects/1/images/chapter_7/image.png"));
    }

    #[test]
    fn relative_media_uri_path_rejects_absolute_path() {
        let err = relative_media_uri_path("/%2Fhome%2Ffufu%2FCode%2FA4N%2Fdata%2Fimage.png")
            .expect_err("absolute media path should fail");

        assert!(err.contains("相对路径"));
    }

    #[test]
    fn relative_media_uri_path_rejects_blank_path() {
        let err = relative_media_uri_path("/").expect_err("blank path should fail");

        assert!(err.contains("不能为空"));
    }

    #[test]
    fn requested_byte_range_supports_open_and_suffix_ranges() {
        assert_eq!(
            requested_byte_range("bytes=10-", 100).expect("range"),
            Ok((10, 99))
        );
        assert_eq!(
            requested_byte_range("bytes=-20", 100).expect("range"),
            Ok((80, 99))
        );
    }

    #[test]
    fn requested_byte_range_rejects_invalid_ranges() {
        assert_eq!(requested_byte_range("bytes=10-5", 100), Some(Err(())));
        assert_eq!(requested_byte_range("bytes=100-120", 100), Some(Err(())));
        assert_eq!(requested_byte_range("bytes=0-1,2-3", 100), Some(Err(())));
        assert_eq!(requested_byte_range("bytes=10", 100), Some(Err(())));
        assert_eq!(requested_byte_range("bytes=abc-20", 100), Some(Err(())));
        assert_eq!(requested_byte_range("bytes=-abc", 100), Some(Err(())));
        assert_eq!(requested_byte_range("bytes=", 100), Some(Err(())));
        assert_eq!(requested_byte_range("items=0-1", 100), None);
    }

    #[test]
    fn requested_byte_range_header_maps_invalid_header_to_invalid_range() {
        let valid: Option<Result<&str, ()>> = Some(Ok("bytes=10-"));
        let invalid: Option<Result<&str, ()>> = Some(Err(()));
        let absent: Option<Result<&str, ()>> = None;

        assert_eq!(
            requested_byte_range_header(valid, 100).expect("range"),
            Ok((10, 99))
        );
        assert_eq!(requested_byte_range_header(invalid, 100), Some(Err(())));
        assert_eq!(requested_byte_range_header(absent, 100), None);
    }

    #[test]
    fn content_type_for_known_media_extensions() {
        assert_eq!(content_type_for_path(Path::new("image.PNG")), "image/png");
        assert_eq!(content_type_for_path(Path::new("clip.mp4")), "video/mp4");
        assert_eq!(content_type_for_path(Path::new("voice.ogg")), "audio/ogg");
        assert_eq!(
            content_type_for_path(Path::new("archive.bin")),
            "application/octet-stream"
        );
    }
}
