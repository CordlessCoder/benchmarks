use std::{
    cell::Cell,
    ops::{Deref, DerefMut},
    path::PathBuf,
};

thread_local! {
    static CACHED_PATH: Cell<PathBuf> = const { Cell::new(PathBuf::new()) };
}

#[derive(Debug)]
pub struct CachedPath {
    pub path: PathBuf,
}

impl CachedPath {
    #[must_use]
    pub fn take() -> Self {
        CachedPath {
            path: CACHED_PATH.take(),
        }
    }
    pub fn with_path<R>(cb: impl FnOnce(&mut PathBuf) -> R) -> R {
        let mut path = Self::take();
        cb(&mut path)
    }
}

impl Deref for CachedPath {
    type Target = PathBuf;

    fn deref(&self) -> &Self::Target {
        &self.path
    }
}

impl DerefMut for CachedPath {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.path
    }
}

impl Drop for CachedPath {
    fn drop(&mut self) {
        CACHED_PATH.replace(core::mem::take(&mut self.path));
    }
}
