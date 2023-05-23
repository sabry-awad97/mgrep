use std::error::Error;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use structopt::StructOpt;

#[derive(Debug)]
enum SearchError {
    IoError(std::io::Error),
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

struct Job {
    path: PathBuf,
}

impl Job {
    fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

struct Worklist {
    jobs: Arc<Mutex<Vec<Job>>>,
}

impl Worklist {
    fn new() -> Self {
        Worklist {
            jobs: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn add(&self, job: Job) {
        let mut jobs = self.jobs.lock().unwrap();
        jobs.push(job);
    }

    fn next(&self) -> Option<Job> {
        let mut jobs = self.jobs.lock().unwrap();
        jobs.pop()
    }

    fn finalize(&self, num_workers: i32) {
        for _ in 0..num_workers {
            self.add(Job::new(PathBuf::new()));
        }
    }
}

struct SearchResult {
    path: PathBuf,
    line_number: usize,
    line: String,
}

struct Worker {
    search_term: String,
    worklist: Arc<Worklist>,
    results: Arc<Mutex<Vec<SearchResult>>>,
}

impl Worker {
    fn new(
        search_term: String,
        worklist: Arc<Worklist>,
        results: Arc<Mutex<Vec<SearchResult>>>,
    ) -> Self {
        Worker {
            search_term,
            worklist,
            results,
        }
    }

    fn find_in_file(&self, path: &Path) -> Result<Vec<SearchResult>, SearchError> {
        if !path.exists() {
            return Ok(Vec::new());
        }

        let file_contents = fs::read_to_string(path)?;
        let mut matching_lines = Vec::new();

        for (line_number, line) in file_contents.lines().enumerate() {
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

fn discover_dirs(wl: &Arc<Worklist>, dir_path: &Path) -> Result<(), SearchError> {
    if let Ok(entries) = fs::read_dir(dir_path) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                discover_dirs(wl, &path)?;
            } else {
                wl.add(Job::new(path));
            }
        }
        Ok(())
    } else {
        Err(SearchError::InvalidSearchDir)
    }
}

#[derive(StructOpt)]
struct Cli {
    #[structopt(required = true, help = "The search term")]
    search_term: String,
    #[structopt(
        help = "The directory to search in",
        required = true,
        parse(from_os_str)
    )]
    search_dir: PathBuf,
}

fn main() -> Result<(), SearchError> {
    let args = Cli::from_args();

    let search_term = args.search_term.clone();
    let search_dir = args.search_dir;

    let worklist = Arc::new(Worklist::new());
    let results = Arc::new(Mutex::new(Vec::new()));

    let num_workers = 10;

    let worklist_clone = Arc::clone(&worklist);
    thread::spawn(move || {
        if let Err(error) = discover_dirs(&worklist_clone, &search_dir) {
            eprintln!("Error discovering directories: {}", error);
        }
        worklist_clone.finalize(num_workers);
    });

    let mut worker_threads = Vec::new();
    for _ in 0..num_workers {
        let worklist_clone = Arc::clone(&worklist);
        let results_clone = Arc::clone(&results);
        let search_term_clone = search_term.clone();
        let thread = thread::spawn(move || {
            let worker = Worker::new(search_term_clone, worklist_clone, results_clone);
            worker.process_jobs();
        });
        worker_threads.push(thread);
    }

    for thread in worker_threads {
        thread.join().unwrap();
    }

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
