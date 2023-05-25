use crate::error::SearchError;
use crate::result::SearchResult;
use crate::worklist::Worklist;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Represents a worker responsible for searching a worklist of files for a given search term.
pub struct Worker {
    /// The search term to look for in files.
    search_term: String,
    /// A reference to the shared worklist.
    worklist: Arc<Worklist>,
    /// A thread-safe list of search results.
    results: Arc<Mutex<Vec<SearchResult>>>,
}

impl Worker {
    /// Creates a new worker with the specified search term, worklist, results.
    pub fn new(
        search_term: String,
        worklist: Arc<Worklist>,
        results: Arc<Mutex<Vec<SearchResult>>>,
    ) -> Self {
        Self {
            search_term,
            worklist,
            results,
        }
    }

    /// Searches for the search term in the given file.
    /// Returns a vector of matching search results.
    fn find_in_file(&self, path: PathBuf) -> Result<Vec<SearchResult>, SearchError> {
        if !path.exists() {
            // If the file doesn't exist, return an empty vector.
            return Ok(Vec::new());
        }

        // Attempt to open the file, and propagate any I/O errors.
        let file = fs::File::open(&path)?;

        // Create a buffered reader for efficient reading.
        let reader = BufReader::with_capacity(8192, file);

        // Stores the search results found in the file.
        let mut matching_lines = Vec::new();

        // Read each line from the file and keep track of its line number.
        for (line_number, line) in reader.lines().enumerate() {
            // Unwrap the line from the result, propagating any I/O errors.
            let line = line?;

            // Add 1 to convert zero-based index to one-based line number.
            let line_number = line_number + 1;

            // Check if the line contains the search term.
            if line.contains(&self.search_term) {
                matching_lines.push(SearchResult::new(path.clone(), line_number, line));
            }
        }

        // Return the vector of matching search results.
        Ok(matching_lines)
    }

    /// Processes the jobs until all jobs are completed.
    pub fn process_jobs(&self) {
        loop {
            let job = self.worklist.next();
            if let Some(job) = job {
                match self.find_in_file(job.into_inner()) {
                    Ok(results) => {
                        let mut result_vec = self.results.lock().unwrap();
                        result_vec.extend(results);
                    }
                    Err(error) => {
                        eprintln!("Error processing job: {}", error);
                    }
                }
            } else {
                break;
            }
        }
    }
}
