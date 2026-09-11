use std::{
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub fn create_unique_timestamped_file(
    directory: &Path,
    prefix: &str,
    extension: &str,
) -> Result<(PathBuf, std::fs::File), std::io::Error> {
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?
        .as_nanos();
    for attempt in 0..20u32 {
        let suffix = if attempt == 0 {
            String::new()
        } else {
            format!("_{attempt}")
        };
        let path = directory.join(format!("{prefix}_{timestamp}{suffix}.{extension}"));
        match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&path)
        {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AlreadyExists,
        "文件名冲突过多",
    ))
}
