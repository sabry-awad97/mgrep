use cli::Cli;
use error::SearchError;
use job::Job;

use rayon::prelude::*;
use std::error::Error;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use structopt::StructOpt;
use worklist::Worklist;

mod cli;
mod error;
mod job;
mod worklist;

/// Represents a search result for a specific line in a file.
struct SearchResult {
    // The path of the file containing the search result
    path: PathBuf,
    // The line number of the search result
    line_number: usize,
    // The actual line content of the search result
    line: String,
}

/// Represents a worker responsible for searching a worklist of files for a given search term.
struct FileSearchWorker {
    /// The search term to look for in files.
    search_term: String,
    /// A reference to the shared worklist.
    worklist: Arc<Worklist>,
    /// A thread-safe list of search results.
    results: Arc<Mutex<Vec<SearchResult>>>,
}

impl FileSearchWorker {
    /// Creates a new worker with the specified search term, worklist, results.
    fn new(
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

            // Check if the line contains the search term.
            if line.contains(&self.search_term) {
                matching_lines.push(SearchResult {
                    path: path.clone(),
                    // Add 1 to convert zero-based index to one-based line number.
                    line_number: line_number + 1,
                    line: line.to_string(),
                });
            }
        }

        // Return the vector of matching search results.
        Ok(matching_lines)
    }

    /// Processes the jobs until all jobs are completed.
    fn process_jobs(&self) {
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

/// Recursively discovers directories and files within a given directory and adds jobs to the worklist.
fn discover_dirs(wl: &Arc<Worklist>, dir_path: &Path) -> Result<(), SearchError> {
    // Attempt to read the directory entries within the specified directory.
    fs::read_dir(dir_path)
        .map_err(|err| {
            SearchError::InvalidDir(format!(
                "Failed to read directory '{}': {}",
                dir_path.display(),
                err
            ))
        })
        .and_then(|entries| {
            // Iterate over each entry in the directory.
            entries.par_bridge().try_for_each(|entry| {
                let path = entry
                    .map_err(|err| {
                        SearchError::InvalidDir(format!("Failed to read directory entry: {}", err))
                    })?
                    .path();

                // Check if the entry is a directory.
                if path.is_dir() {
                    // Recursively explore subdirectories in parallel using Rayon.
                    let wl = Arc::clone(wl);
                    discover_dirs(&wl, &path)
                } else {
                    // Add the file as a job to the worklist.
                    wl.add(Job::new(path));
                    Ok(())
                }
            })
        })
}

fn main() -> Result<(), Box<dyn Error>> {
    // Parse command-line arguments using `Cli::from_args()`
    let args = Cli::from_args();
    let search_term_ref = &args.search_term;

    // Determine the number of worker threads
    let num_workers = num_cpus::get() - 1;

    // Create a shared worklist using `Arc`
    let worklist = Arc::new(Worklist::new());

    // Create a shared results list using `Arc` and `Mutex`
    let results = Arc::new(Mutex::new(Vec::new()));

    // Spawn a separate thread to discover directories and add jobs to the worklist
    let worklist_clone = Arc::clone(&worklist);
    thread::spawn(move || {
        if let Err(error) = discover_dirs(&worklist_clone, &args.search_dir) {
            eprintln!("Error discovering directories: {}", error);
            if let Some(source) = error.source() {
                eprintln!("Caused by: {}", source);
            }
        }
        worklist_clone.finalize(num_workers);
    });

    // Process jobs in parallel
    (0..num_workers).into_par_iter().for_each(|_| {
        // Clone the necessary variables for each worker thread
        let worklist_clone = Arc::clone(&worklist);
        let results_clone = Arc::clone(&results);
        let worker =
            FileSearchWorker::new(search_term_ref.to_string(), worklist_clone, results_clone);
        worker.process_jobs();
    });

    // Print the search results by locking the results list and iterating over the results
    let results = results.lock().unwrap();
    for result in &*results {
        println!(
            "{}[{}]: {}",
            result.path.display(),
            result.line_number,
            result.line
        );
    }

    // Return `Ok` to indicate successful execution of the `main` function
    Ok(())
}
