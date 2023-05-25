use std::path::PathBuf;

/// Represents a job to search for a specific term in a file.
pub struct Job {
    /// The path of the file to search
    path: PathBuf,
}

impl Job {
    /// Creates a new job with the specified file path.
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn into_inner(self) -> PathBuf {
        self.path
    }
}
