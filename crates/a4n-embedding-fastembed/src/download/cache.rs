use std::path::{Component, Path, PathBuf};

pub(super) fn cached_file_size(
    cache_repo: &hf_hub::CacheRepo,
    path: &str,
    etag: &str,
    size_bytes: u64,
) -> u64 {
    if let Some(cached_path) = cache_repo.get(path) {
        let cached_size = file_size(&cached_path);
        if cached_size >= size_bytes {
            return size_bytes;
        }
    }
    let blob_size = file_size(&cache_repo.blob_path(etag));
    if blob_size >= size_bytes {
        return size_bytes;
    }

    let mut part_path = cache_repo.blob_path(etag);
    part_path.set_extension("part");
    let part_size = file_size(&part_path);
    if part_size > size_bytes { 0 } else { part_size }
}

pub(super) fn file_size(path: &Path) -> u64 {
    std::fs::metadata(path)
        .map(|metadata| metadata.len())
        .unwrap_or(0)
}

pub(super) fn create_snapshot_pointer(
    cache_repo: &hf_hub::CacheRepo,
    blob_path: &Path,
    commit_hash: &str,
    filename: &str,
) -> Result<(), String> {
    let mut pointer_path = cache_repo.pointer_path(commit_hash);
    pointer_path.push(filename);
    std::fs::create_dir_all(
        pointer_path
            .parent()
            .ok_or_else(|| format!("模型缓存快照路径缺少父目录：{}", pointer_path.display()))?,
    )
    .map_err(|err| format!("创建模型缓存快照目录失败：{err}"))?;
    symlink_or_rename(blob_path, &pointer_path)
        .map_err(|err| format!("创建模型缓存快照指针失败：{err}"))?;
    cache_repo
        .create_ref(commit_hash)
        .map_err(|err| format!("写入模型缓存引用失败：{err}"))?;
    Ok(())
}

fn make_relative(src: &Path, dst: &Path) -> PathBuf {
    let mut src_components = src.components();
    let mut dst_components = dst.components();
    loop {
        match (src_components.next(), dst_components.next()) {
            (Some(src_component), Some(dst_component)) if src_component == dst_component => {}
            (src_component, _) => {
                let mut relative = PathBuf::new();
                for _ in dst_components {
                    relative.push(Component::ParentDir);
                }
                if let Some(src_component) = src_component {
                    relative.push(src_component);
                    for component in src_components {
                        relative.push(component);
                    }
                }
                return relative;
            }
        }
    }
}

fn symlink_or_rename(src: &Path, dst: &Path) -> Result<(), std::io::Error> {
    if dst.exists() {
        return Ok(());
    }
    let rel_src = make_relative(src, dst);

    #[cfg(target_os = "windows")]
    {
        if std::os::windows::fs::symlink_file(rel_src, dst).is_err() {
            std::fs::rename(src, dst)?;
        }
    }

    #[cfg(target_family = "unix")]
    std::os::unix::fs::symlink(rel_src, dst)?;

    Ok(())
}
