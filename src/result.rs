use std::path::PathBuf;

/// Represents a search result for a specific line in a file.
pub struct SearchResult {
    // The path of the file containing the search result
    pub path: PathBuf,
    // The line number of the search result
    pub line_number: usize,
    // The actual line content of the search result
    pub line: String,
}

impl SearchResult {
    pub fn new(path: PathBuf, line_number: usize, line: String) -> Self {
        Self {
            path,
            line_number,
            line,
        }
    }

    pub fn display(&self) {
        println!(
            "{}[{}]: {}",
            self.path.display(),
            self.line_number,
            self.line
        );
    }
}
