use super::*;
use std::path::Path;

#[test]
fn video_mime_from_path_rejects_unknown_video_extensions() {
    assert_eq!(
        media::video_mime_from_path(Path::new("clip.mp4")).expect("mp4 MIME"),
        "video/mp4"
    );
    assert_eq!(
        media::video_mime_from_path(Path::new("clip.M4V")).expect("m4v MIME"),
        "video/mp4"
    );
    assert_eq!(
        media::video_mime_from_path(Path::new("clip.webm")).expect("webm MIME"),
        "video/webm"
    );
    assert_eq!(
        media::video_mime_from_path(Path::new("clip.mov")).expect("mov MIME"),
        "video/quicktime"
    );

    let err = media::video_mime_from_path(Path::new("clip.avi"))
        .expect_err("unsupported extension should fail");
    assert!(err.contains("视频文件扩展名不支持播放：avi"));

    let err =
        media::video_mime_from_path(Path::new("clip")).expect_err("missing extension should fail");
    assert_eq!(err, "视频文件缺少扩展名，无法确定 MIME 类型");
}
