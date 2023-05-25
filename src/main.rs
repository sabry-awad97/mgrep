use crossbeam::channel::{unbounded, Receiver, Sender}; // Import for creating communication channels between threads
use rayon::prelude::*; // Import the necessary traits for parallel processing
use std::error::Error; // Import the Error trait for error handling
use std::fs; // Import the fs module for file system operations
use std::io::{BufRead, BufReader}; // Import the necessary traits for buffered reading
use std::path::{Path, PathBuf}; // Import the necessary structs for working with file paths
use std::sync::{Arc, Mutex}; // Import the necessary synchronization primitives
use std::thread; // Import the thread module for multi-threading
use structopt::StructOpt; // Import the StructOpt trait for command-line argument parsing

#[derive(Debug)]
enum SearchError {
    /// Represents an IO error
    IoError(std::io::Error),
    /// Represents an invalid search directory
    InvalidSearchDir,
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
            SearchError::InvalidSearchDir => write!(f, "Invalid search directory"),
        }
    }
}

impl std::error::Error for SearchError {}

/// Represents a job to search for a specific term in a file.
struct Job {
    /// The path of the file to search
    path: PathBuf,
}

impl Job {
    /// Creates a new job with the specified file path.
    fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

/// Represents a collection of file search jobs.
struct Worklist {
    sender: Sender<Option<Job>>,
    receiver: Receiver<Option<Job>>,
}

impl Worklist {
    /// Creates a new empty worklist.
    fn new() -> Self {
        let (sender, receiver) = unbounded();
        Self { sender, receiver }
    }

    /// Adds a new job to the worklist.
    fn add(&self, job: Job) {
        self.sender.send(Some(job)).unwrap();
    }

    /// Retrieves the next job from the worklist.
    fn next(&self) -> Option<Job> {
        self.receiver.recv().unwrap()
    }

    /// Marks the end of jobs by adding a special empty jobs to the worklist.
    fn finalize(&self, num_workers: usize) {
        for _ in 0..num_workers {
            self.sender.send(None).unwrap();
        }
    }
}

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
    fn find_in_file(&self, path: &Path) -> Result<Vec<SearchResult>, SearchError> {
        if !path.exists() {
            // If the file doesn't exist, return an empty vector.
            return Ok(Vec::new());
        }

        // Attempt to open the file, and propagate any I/O errors.
        let file = fs::File::open(path)?;

        // Create a buffered reader for efficient reading.
        let reader = BufReader::new(file);

        // Stores the search results found in the file.
        let mut matching_lines = Vec::new();

        // Read each line from the file and keep track of its line number.
        for (line_number, line) in reader.lines().enumerate() {
            // Unwrap the line from the result, propagating any I/O errors.
            let line = line?;

            // Check if the line contains the search term.
            if line.contains(&self.search_term) {
                matching_lines.push(SearchResult {
                    path: path.to_path_buf(),
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
                match self.find_in_file(&job.path) {
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
    if let Ok(entries) = fs::read_dir(dir_path) {
        // Iterate over each entry in the directory.
        for entry in entries.flatten() {
            let path = entry.path();

            // Check if the entry is a directory.
            if path.is_dir() {
                // Recursively explore subdirectories by calling `discover_dirs` function again.
                discover_dirs(wl, &path)?;
            } else {
                // Add the file as a job to the worklist.
                wl.add(Job::new(path));
            }
        }

        // Return Ok to indicate successful discovery of directories and files.
        Ok(())
    } else {
        // Return an error if reading the directory failed.
        Err(SearchError::InvalidSearchDir)
    }
}

/// Represents the command-line arguments for the program.
#[derive(StructOpt)]
struct Cli {
    /// The search term
    search_term: String,

    /// The directory to search in
    #[structopt(parse(from_os_str))]
    search_dir: PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    // Parse command-line arguments using `Cli::from_args()`
    let args = Cli::from_args();

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
        }
        worklist_clone.finalize(num_workers);
    });

    // Process jobs in parallel
    (0..num_workers).into_par_iter().for_each(|_| {
        // Clone the necessary variables for each worker thread
        let worklist_clone = Arc::clone(&worklist);
        let results_clone = Arc::clone(&results);
        let search_term_clone = args.search_term.clone();
        let worker = FileSearchWorker::new(search_term_clone, worklist_clone, results_clone);
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
