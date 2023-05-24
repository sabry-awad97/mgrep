use rayon::prelude::*; // Import the necessary traits for parallel processing
use rayon::ThreadPoolBuilder; // Import the ThreadPoolBuilder struct from the rayon crate
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
    fn from(error: std::io::Error) -> Self {
        SearchError::IoError(error)
    }
}

impl std::fmt::Display for SearchError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SearchError::IoError(error) => write!(f, "IO error: {}", error),
            SearchError::InvalidSearchDir => write!(f, "Invalid search directory"),
        }
    }
}

impl Error for SearchError {}

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
    // A thread-safe list of jobs
    jobs: Arc<Mutex<Vec<Job>>>,
}

impl Worklist {
    /// Creates a new empty worklist.
    fn new() -> Self {
        Worklist {
            jobs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Adds a new job to the worklist.
    fn add(&self, job: Job) {
        let mut jobs = self.jobs.lock().unwrap();
        jobs.push(job);
    }

    /// Retrieves a chunk of jobs from the worklist.
    fn get_chunk(&self, chunk_size: usize) -> Vec<Job> {
        let mut jobs = self.jobs.lock().unwrap();
        let length = jobs.len();
        let chunk = jobs
            .splice(0..chunk_size.min(length), vec![]) // Take a chunk of jobs from the list
            .collect::<Vec<_>>();
        chunk
    }

    /// Marks the end of jobs by adding a special empty job to the worklist.
    fn finalize(&self) {
        self.add(Job::new(PathBuf::new()));
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

/// Represents a worker responsible for searching for a term in files.
struct Worker {
    /// The search term to look for in files
    search_term: String,
    /// A reference to the shared worklist
    worklist: Arc<Worklist>,
    /// A thread-safe list of search results
    results: Arc<Mutex<Vec<SearchResult>>>,
    /// The number of jobs to process in a single chunk
    chunk_size: usize,
}

impl Worker {
    /// Creates a new worker with the specified search term, worklist, results, and chunk size.
    fn new(
        search_term: String,
        worklist: Arc<Worklist>,
        results: Arc<Mutex<Vec<SearchResult>>>,
        chunk_size: usize,
    ) -> Self {
        Self {
            search_term,
            worklist,
            results,
            chunk_size,
        }
    }

    /// Searches for the search term in the given file.
    /// Returns a vector of matching search results.
    fn find_in_file(&self, path: &Path) -> Result<Vec<SearchResult>, SearchError> {
        if !path.exists() {
            return Ok(Vec::new());
        }

        // Open the file for reading
        let file = fs::File::open(path)?;

        // Create a buffered reader for efficient reading
        let reader = BufReader::new(file);

        let mut matching_lines = Vec::new();

        for (line_number, line) in reader.lines().enumerate() {
            // Read the line from the reader
            let line = line?;
            if line.contains(&self.search_term) {
                matching_lines.push(SearchResult {
                    path: path.to_path_buf(),
                    line_number: line_number + 1,
                    line: line.to_string(),
                });
            }
        }

        Ok(matching_lines)
    }

    /// Processes the jobs in chunks until all jobs are completed.
    fn process_jobs(&self) {
        loop {
            // Get a chunk of jobs to process
            let jobs = self.worklist.get_chunk(self.chunk_size);
            if jobs.is_empty() {
                break;
            }

            let results = jobs
                .par_iter() // Process jobs in parallel using rayon
                .filter_map(|job| {
                    self.find_in_file(&job.path)
                        .map_err(|error| eprintln!("Error processing job: {}", error))
                        .ok()
                })
                .flatten()
                .collect::<Vec<_>>();

            let mut result_vec = self.results.lock().unwrap();
            result_vec.extend(results);
        }
    }
}

/// Recursively discovers directories and files within a given directory and adds jobs to the worklist.
fn discover_dirs(wl: &Arc<Worklist>, dir_path: &Path) -> Result<(), SearchError> {
    if let Ok(entries) = fs::read_dir(dir_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                // Recursively explore subdirectories
                discover_dirs(wl, &path)?;
            } else {
                // Add file as a job to the worklist
                wl.add(Job::new(path));
            }
        }
        Ok(())
    } else {
        Err(SearchError::InvalidSearchDir)
    }
}

/// Represents the command-line arguments for the program.
#[derive(StructOpt)]
struct Cli {
    /// The search term
    #[structopt(required = true)]
    search_term: String,

    /// The directory to search in
    #[structopt(required = true, parse(from_os_str))]
    search_dir: PathBuf,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::from_args();

    let search_term = args.search_term.clone();
    let search_dir = args.search_dir;
    let chunk_size = 10;
    let num_workers = num_cpus::get();

    // Create a shared worklist
    let worklist = Arc::new(Worklist::new());

    // Create a shared results list
    let results = Arc::new(Mutex::new(Vec::new()));

    // Spawn a separate thread to discover directories and add jobs to the worklist
    let worklist_clone = Arc::clone(&worklist);
    thread::spawn(move || {
        if let Err(error) = discover_dirs(&worklist_clone, &search_dir) {
            eprintln!("Error discovering directories: {}", error);
        }
        worklist_clone.finalize();
    });

    // Create a thread pool to process jobs in parallel
    let thread_pool = ThreadPoolBuilder::new().num_threads(num_workers).build()?;

    // Process jobs using worker threads
    thread_pool.scope(|s| {
        (0..num_workers).into_par_iter().for_each(|_| {
            let worklist_clone = Arc::clone(&worklist);
            let results_clone = Arc::clone(&results);
            let search_term_clone = search_term.clone();
            s.spawn(|_| {
                let worker =
                    Worker::new(search_term_clone, worklist_clone, results_clone, chunk_size);
                worker.process_jobs();
            });
        })
    });

    // Print the search results
    let results = results.lock().unwrap();
    for result in &*results {
        println!(
            "{}[{}]: {}",
            result.path.display(),
            result.line_number,
            result.line
        );
    }

    Ok(())
}
