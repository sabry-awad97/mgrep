use std::path::PathBuf;

pub struct SearchResult {
    pub path: PathBuf,
    pub line_number: usize,
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
