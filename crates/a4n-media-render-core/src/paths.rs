use std::path::{Path, PathBuf};

use a4n_media_contract::{MediaFileRef, MediaRoot};

#[derive(Debug, Clone)]
pub struct MediaRoots {
    data_dir: PathBuf,
    temp_dir: PathBuf,
}

impl MediaRoots {
    pub fn new(data_dir: &Path, temp_dir: &Path) -> Result<Self, String> {
        Ok(Self {
            data_dir: canonical_root(data_dir, "数据目录")?,
            temp_dir: canonical_root(temp_dir, "临时目录")?,
        })
    }

    pub fn input(&self, file: &MediaFileRef) -> Result<PathBuf, String> {
        file.validate()?;
        let root = self.root(file.root);
        let path = root
            .join(&file.relative_path)
            .canonicalize()
            .map_err(|error| format!("无法读取剪辑源媒体：{error}"))?;
        if !path.starts_with(root) {
            return Err("剪辑源媒体不在允许的本地目录内".to_string());
        }
        if !path.is_file() {
            return Err("剪辑源媒体不是文件".to_string());
        }
        Ok(path)
    }

    pub fn output(&self, file: &MediaFileRef) -> Result<PathBuf, String> {
        file.validate()?;
        let root = self.root(file.root);
        let path = root.join(&file.relative_path);
        let parent = path
            .parent()
            .ok_or_else(|| "剪辑输出媒体缺少父目录".to_string())?
            .canonicalize()
            .map_err(|error| format!("无法解析剪辑输出目录：{error}"))?;
        if !parent.starts_with(root) {
            return Err("剪辑输出媒体不在允许的本地目录内".to_string());
        }
        match std::fs::symlink_metadata(&path) {
            Ok(metadata) if metadata.file_type().is_symlink() => {
                Err("剪辑输出媒体不能是符号链接".to_string())
            }
            Ok(metadata) if !metadata.is_file() => Err("剪辑输出媒体不是文件".to_string()),
            Ok(_) => Ok(path),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(path),
            Err(error) => Err(format!("无法检查剪辑输出媒体：{error}")),
        }
    }

    fn root(&self, root: MediaRoot) -> &Path {
        match root {
            MediaRoot::Data => &self.data_dir,
            MediaRoot::Temp => &self.temp_dir,
        }
    }
}

fn canonical_root(path: &Path, label: &str) -> Result<PathBuf, String> {
    let path = path
        .canonicalize()
        .map_err(|error| format!("无法解析媒体 worker {label}：{error}"))?;
    if !path.is_dir() {
        return Err(format!("媒体 worker {label}不是目录"));
    }
    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::MediaRoots;
    use a4n_media_contract::{MediaFileRef, MediaRoot};

    #[test]
    fn media_roots_reject_traversal_and_external_symlinks() {
        let root = std::env::temp_dir().join(format!("a4n-media-roots-{}", std::process::id()));
        let outside = root.with_extension("outside");
        std::fs::create_dir_all(&root).expect("create media root");
        std::fs::create_dir_all(&outside).expect("create outside dir");
        std::fs::write(outside.join("secret"), b"secret").expect("write outside file");
        #[cfg(unix)]
        std::os::unix::fs::symlink(outside.join("secret"), root.join("escape"))
            .expect("create escape symlink");
        let roots = MediaRoots::new(&root, &root).expect("resolve media roots");

        let traversal = MediaFileRef {
            root: MediaRoot::Data,
            relative_path: "../secret".into(),
        };
        assert!(roots.input(&traversal).is_err());
        #[cfg(unix)]
        {
            let symlink = MediaFileRef {
                root: MediaRoot::Data,
                relative_path: "escape".into(),
            };
            assert!(roots.input(&symlink).is_err());
        }

        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&outside);
    }
}
