use log::debug;
use std::{env, path::PathBuf};

pub fn resource_path(resource: &str) -> crate::error::Result<PathBuf> {
    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        let p = PathBuf::from(manifest_dir).join("assets").join(resource);
        if p.exists() {
            debug!("Found {resource} at: {}", p.to_string_lossy());
            return Ok(p);
        }
    }

    if let Ok(mut exe) = env::current_exe() {
        exe.pop();
        let p = exe.join("assets").join(resource);
        if p.exists() {
            debug!("Found {resource} at: {}", p.to_string_lossy());
            return Ok(p);
        }
    }

    Err(crate::error::Error::MissingResource(resource.to_owned()))
}
