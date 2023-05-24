use rayon::ThreadPoolBuilder;
use std::error::Error;
use std::fs;
use std::io::{BufRead, BufReader};
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

    fn get_chunk(&self, chunk_size: usize) -> Vec<Job> {
        let mut jobs = self.jobs.lock().unwrap();
        let length = jobs.len();
        let chunk = jobs
            .splice(0..chunk_size.min(length), vec![])
            .collect::<Vec<_>>();
        chunk
    }

    fn finalize(&self) {
        self.add(Job::new(PathBuf::new()));
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
    chunk_size: usize,
}

impl Worker {
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

    fn find_in_file(&self, path: &Path) -> Result<Vec<SearchResult>, SearchError> {
        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = fs::File::open(path)?;
        let reader = BufReader::new(file);
        let mut matching_lines = Vec::new();

        for (line_number, line) in reader.lines().enumerate() {
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

    fn process_jobs(&self) {
        loop {
            let chunk = self.worklist.get_chunk(self.chunk_size);
            if chunk.is_empty() {
                break;
            }
            for job in chunk {
                match self.find_in_file(&job.path) {
                    Ok(results) => {
                        let mut result_vec = self.results.lock().unwrap();
                        result_vec.extend(results);
                    }
                    Err(error) => {
                        eprintln!("Error processing job: {}", error);
                    }
                }
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

    #[structopt(
        short,
        long,
        help = "The number of worker threads",
        default_value = "10"
    )]
    num_workers: usize,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args = Cli::from_args();

    let search_term = args.search_term.clone();
    let search_dir = args.search_dir;
    let chunk_size = 10;

    let worklist = Arc::new(Worklist::new());
    let results = Arc::new(Mutex::new(Vec::new()));

    let worklist_clone = Arc::clone(&worklist);
    thread::spawn(move || {
        if let Err(error) = discover_dirs(&worklist_clone, &search_dir) {
            eprintln!("Error discovering directories: {}", error);
        }
        worklist_clone.finalize();
    });

    let thread_pool = ThreadPoolBuilder::new()
        .num_threads(args.num_workers)
        .build()?;

    thread_pool.scope(|s| {
        for _ in 0..args.num_workers {
            let worklist_clone = Arc::clone(&worklist);
            let results_clone = Arc::clone(&results);
            let search_term_clone = search_term.clone();
            s.spawn(|_| {
                let worker =
                    Worker::new(search_term_clone, worklist_clone, results_clone, chunk_size);
                worker.process_jobs();
            });
        }
    });

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
