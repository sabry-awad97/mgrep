use std::path::PathBuf;

pub struct Job {
    path: PathBuf,
}

impl Job {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn into_inner(self) -> PathBuf {
        self.path
    }
}
