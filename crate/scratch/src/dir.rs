//! A scratch directory its owner made: private from creation and marked.
use crate::*;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

/// A scratch directory's mode on Unix: its owner's alone.
#[cfg(unix)]
const DIR_MODE: u32 = 0o700;

/// A scratch directory this process made, marked as its guard's own.
#[derive(Debug)]
pub struct OwnedDir {
    path: PathBuf,
    nonce: String,
}

impl OwnedDir {
    /// A directory made in `base` under `prefix`, private from creation, and marked.
    pub fn create(
        base: &Path,
        prefix: &str,
        owner: &Owner,
    ) -> io::Result<Self> {
        let nonce = owner
            .nonce
            .clone()
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "an owner without a nonce"))?;
        let mut builder = tempfile::Builder::new();
        builder.prefix(prefix);
        #[cfg(unix)]
        builder.permissions(fs::Permissions::from_mode(DIR_MODE));
        let path = builder.tempdir_in(base)?.keep();
        if let Err(e) = owner.write(&path) {
            let _ = fs::remove_dir_all(&path);
            return Err(e);
        }
        Ok(Self { path, nonce })
    }

    /// The directory's path.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Removes the directory while its marker holds this guard's nonce.
    pub fn remove(&self) -> io::Result<()> {
        match fs::symlink_metadata(&self.path) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(()),
            Err(e) => return Err(e),
            Ok(meta) if !meta.is_dir() => {
                return Err(io::Error::other(format!("{} is no directory now", self.path.display())));
            }
            Ok(_) => {}
        }
        match Owner::read(&self.path, current_uid()) {
            Ok(owner) if owner.nonce.as_deref() == Some(self.nonce.as_str()) => {
                match fs::remove_dir_all(&self.path) {
                    Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
                    removed => removed,
                }
            }
            Ok(_) => Err(io::Error::other(format!("{}'s marker is another guard's", self.path.display()))),
            Err(e) => Err(io::Error::other(format!("{}: {e}", self.path.display()))),
        }
    }
}
