#[derive(Debug)]
pub enum SearchError {
    /// Represents an IO error
    IoError(std::io::Error),
    /// Represents an invalid search directory
    InvalidDir(String),
}

impl From<std::io::Error> for SearchError {
    // Convert std::io::Error to SearchError
    fn from(error: std::io::Error) -> Self {
        SearchError::IoError(error)
    }
}

impl std::fmt::Display for SearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            // Format the IO error with a custom message
            SearchError::IoError(error) => write!(f, "IO error: {}", error),
            // Provide a custom message for the invalid search directory error
            SearchError::InvalidDir(path) => write!(f, "Failed to read directory: '{}'", path),
        }
    }
}

impl std::error::Error for SearchError {}
