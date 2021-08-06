use async_std::fs as async_fs;
use std::ffi::OsStr;
use std::fs::{self, DirEntry};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

pub mod file_lock;

pub fn slurp(path: &dyn AsRef<Path>) -> Result<Vec<u8>, String> { Ok(gstuff::slurp(path)) }

pub fn safe_slurp(path: &dyn AsRef<Path>) -> Result<Vec<u8>, String> {
    let mut file = match fs::File::open(path) {
        Ok(f) => f,
        Err(ref err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(err) => return ERR!("Can't open {:?}: {}", path.as_ref(), err),
    };
    let mut buf = Vec::new();
    try_s!(file.read_to_end(&mut buf));
    Ok(buf)
}

pub fn remove_file(path: &dyn AsRef<Path>) -> Result<(), String> {
    try_s!(fs::remove_file(path));
    Ok(())
}

pub fn write(path: &dyn AsRef<Path>, contents: &dyn AsRef<[u8]>) -> Result<(), String> {
    try_s!(fs::write(path, contents));
    Ok(())
}

/// Read a folder asynchronously and return a list of files.
pub async fn read_dir_async<P: AsRef<Path>>(dir: P) -> Result<Vec<PathBuf>, String> {
    use futures::StreamExt;

    let mut result = Vec::new();
    let mut entries = try_s!(async_fs::read_dir(dir.as_ref()).await);

    while let Some(entry) = entries.next().await {
        let entry = match entry {
            Ok(entry) => entry,
            Err(e) => {
                log::error!("Error '{}' reading from dir {}", e, dir.as_ref().display());
                continue;
            },
        };
        result.push(entry.path().into());
    }
    Ok(result)
}

/// Read a folder and return a list of files with their last-modified ms timestamps.
pub fn read_dir(dir: &dyn AsRef<Path>) -> Result<Vec<(u64, PathBuf)>, String> {
    let entries = try_s!(dir.as_ref().read_dir())
        .filter_map(|dir_entry| {
            let entry = match dir_entry {
                Ok(ent) => ent,
                Err(e) => {
                    log!("Error " (e) " reading from dir " (dir.as_ref().display()));
                    return None;
                },
            };

            let metadata = match entry.metadata() {
                Ok(m) => m,
                Err(e) => {
                    log!("Error " (e) " getting file " (entry.path().display()) " meta");
                    return None;
                },
            };

            let m_time = match metadata.modified() {
                Ok(time) => time,
                Err(e) => {
                    log!("Error " (e) " getting file " (entry.path().display()) " m_time");
                    return None;
                },
            };

            let lm = m_time.duration_since(UNIX_EPOCH).expect("!duration_since").as_millis();
            assert!(lm < u64::MAX as u128);
            let lm = lm as u64;

            let path = entry.path();
            if path.extension() == Some(OsStr::new("json")) {
                Some((lm, path))
            } else {
                None
            }
        })
        .collect();

    Ok(entries)
}

pub fn json_dir_entries(path: &dyn AsRef<Path>) -> Result<Vec<DirEntry>, String> {
    Ok(try_s!(path.as_ref().read_dir())
        .filter_map(|dir_entry| {
            let entry = match dir_entry {
                Ok(ent) => ent,
                Err(e) => {
                    log!("Error " (e) " reading from dir " (path.as_ref().display()));
                    return None;
                },
            };

            if entry.path().extension() == Some(OsStr::new("json")) {
                Some(entry)
            } else {
                None
            }
        })
        .collect())
}
