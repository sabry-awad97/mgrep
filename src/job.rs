use std::path::PathBuf;

pub struct Job {
    path: PathBuf,
}

impl Job {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn as_path(&self) -> &PathBuf {
        &self.path
    }
}
