use std::path::Path;

use anyhow::Result;

use crate::versions::{
    backends::{
        PathVersionBackend, ZipVersionBackend, SEVEN_ZIP_INSTALLED, SUPPORTED_FILE_EXTENSIONS,
    },
    types::VersionBackend,
};

pub mod backends;
pub mod types;
pub fn create_backend_constructor<'a, P>(
    path: P,
) -> Option<Box<dyn FnOnce() -> Result<Box<dyn VersionBackend + Send + Sync + 'a>>>>
where
    P: AsRef<Path>,
{
    let path = path.as_ref();
    if !path.exists() {
        return None;
    }

    let is_directory = path.is_dir();
    if is_directory {
        let base_dir = path.to_path_buf();
        return Some(Box::new(move || {
            Ok(Box::new(PathVersionBackend { base_dir }))
        }));
    };

    if *SEVEN_ZIP_INSTALLED {
        /*
        Slow 7zip integrity test
        let mut test = Command::new("7z");
        test.args(vec!["t", path.to_str().expect("invalid utf path")]);
        let status = test.status().ok()?;
        if status.code().unwrap_or(1) == 0 {
            let buf = path.to_path_buf();
            return Some(Box::new(move || Ok(Box::new(ZipVersionBackend::new(buf)?))));
        }
         */
        // Fast filename-based test
        if let Some(extension) = path.extension().and_then(|v| v.to_str()) {
            let supported = SUPPORTED_FILE_EXTENSIONS.iter().any(|v| **v == *extension);
            if supported {
                let buf = path.to_path_buf();
                return Some(Box::new(move || Ok(Box::new(ZipVersionBackend::new(buf)?))));
            }
        }
    }

    None
}
