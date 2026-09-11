use crate::{AppState, cleanup_outbox::enqueue_file_artifact_cleanup_by_project};
use a4n_storage::data_dir_relative_file_path;
use std::{
    io::ErrorKind,
    path::Path,
    time::{Duration, SystemTime},
};

const ASSEMBLY_CACHE_TTL: Duration = Duration::from_secs(7 * 24 * 60 * 60);

pub(in crate::actions::video_records) fn assembly_cache_is_fresh(
    manifest_path: &Path,
) -> Result<bool, String> {
    let modified = std::fs::metadata(manifest_path)
        .and_then(|metadata| metadata.modified())
        .map_err(|error| format!("读取成片预览缓存时间失败：{error}"))?;
    let age = SystemTime::now()
        .duration_since(modified)
        .map_err(|_| "成片预览缓存修改时间晚于当前系统时间".to_string())?;
    Ok(age <= ASSEMBLY_CACHE_TTL)
}

pub(crate) async fn recover_expired_assembly_caches(state: &AppState) {
    let Some(cutoff) = SystemTime::now().checked_sub(ASSEMBLY_CACHE_TTL) else {
        tracing::error!("当前系统时间无法计算成片缓存过期边界");
        return;
    };
    let caches = match expired_assembly_cache_files(state.data_dir(), cutoff) {
        Ok(caches) => caches,
        Err(error) => {
            tracing::warn!(%error, "扫描过期成片缓存失败");
            return;
        }
    };
    for cache in caches {
        if let Err(error) = enqueue_file_artifact_cleanup_by_project(
            state,
            &format!(
                "assembly-cache-expired:{}:{}:{}",
                cache.project_id, cache.version, cache.paths[0]
            ),
            &cache.paths,
        )
        .await
        {
            tracing::warn!(
                %error,
                project_id = cache.project_id,
                "写入过期成片缓存 cleanup intent 失败"
            );
        }
    }
}

pub(in crate::actions::video_records) struct ExpiredAssemblyCache {
    pub(in crate::actions::video_records) project_id: i64,
    version: u128,
    pub(in crate::actions::video_records) paths: Vec<String>,
}

pub(in crate::actions::video_records) fn expired_assembly_cache_files(
    data_dir: &Path,
    cutoff: SystemTime,
) -> Result<Vec<ExpiredAssemblyCache>, String> {
    let projects = match std::fs::read_dir(data_dir.join("projects")) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("读取项目目录失败：{error}")),
    };
    let mut caches = Vec::new();
    for project in projects {
        let project = project.map_err(|error| format!("读取项目目录项失败：{error}"))?;
        if !project
            .file_type()
            .map_err(|error| format!("读取项目目录类型失败：{error}"))?
            .is_dir()
        {
            continue;
        }
        let Some(project_id) = project
            .file_name()
            .to_str()
            .and_then(|value| value.parse::<i64>().ok())
            .filter(|value| *value > 0)
        else {
            continue;
        };
        collect_expired_manifests(
            data_dir,
            project_id,
            &project.path().join("videos"),
            cutoff,
            &mut caches,
        )?;
    }
    Ok(caches)
}

fn collect_expired_manifests(
    data_dir: &Path,
    project_id: i64,
    videos_dir: &Path,
    cutoff: SystemTime,
    caches: &mut Vec<ExpiredAssemblyCache>,
) -> Result<(), String> {
    let chapters = match std::fs::read_dir(videos_dir) {
        Ok(entries) => entries,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("读取项目视频目录失败：{error}")),
    };
    for chapter in chapters {
        let chapter = chapter.map_err(|error| format!("读取章节视频目录项失败：{error}"))?;
        if !chapter
            .file_type()
            .map_err(|error| format!("读取章节视频目录类型失败：{error}"))?
            .is_dir()
        {
            continue;
        }
        let entries = std::fs::read_dir(chapter.path())
            .map_err(|error| format!("读取章节视频目录失败：{error}"))?;
        for entry in entries {
            let entry = entry.map_err(|error| format!("读取成片缓存目录项失败：{error}"))?;
            if !entry
                .file_type()
                .map_err(|error| format!("读取成片缓存文件类型失败：{error}"))?
                .is_file()
            {
                continue;
            }
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|value| value.to_str()) else {
                continue;
            };
            if !name.starts_with("assembled_") || !name.ends_with(".mp4.cache.json") {
                continue;
            }
            let modified = entry
                .metadata()
                .and_then(|metadata| metadata.modified())
                .map_err(|error| format!("读取成片缓存时间失败：{error}"))?;
            if modified > cutoff {
                continue;
            }
            let output = path.with_file_name(name.trim_end_matches(".cache.json"));
            let mut paths = vec![data_dir_relative_file_path(
                data_dir,
                &path,
                "成片缓存清单",
            )?];
            if output.is_file() {
                paths.push(data_dir_relative_file_path(
                    data_dir,
                    &output,
                    "成片缓存文件",
                )?);
            }
            caches.push(ExpiredAssemblyCache {
                project_id,
                version: modified
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .map(|value| value.as_nanos())
                    .map_err(|_| "成片缓存修改时间早于 UNIX_EPOCH".to_string())?,
                paths,
            });
        }
    }
    Ok(())
}
