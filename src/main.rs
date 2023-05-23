use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;
use structopt::StructOpt;

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
}

struct Result {
    path: PathBuf,
    line_number: usize,
    line: String,
}

struct Worker {
    search_term: String,
    worklist: Arc<Worklist>,
    results: Arc<Mutex<Vec<Result>>>,
}

impl Worker {
    fn new(search_term: String, worklist: Arc<Worklist>, results: Arc<Mutex<Vec<Result>>>) -> Self {
        Worker {
            search_term,
            worklist,
            results,
        }
    }

    fn find_in_file(&self, path: &Path) -> Vec<Result> {
        let file_contents = fs::read_to_string(path).unwrap();
        let mut matching_lines = Vec::new();

        for (line_number, line) in file_contents.lines().enumerate() {
            if line.contains(&self.search_term) {
                matching_lines.push(Result {
                    path: path.to_path_buf(),
                    line_number: line_number + 1,
                    line: line.to_string(),
                });
            }
        }

        matching_lines
    }

    fn process_jobs(&self) {
        loop {
            let job = self.worklist.next();
            if let Some(job) = job {
                let results = self.find_in_file(&job.path);
                let mut result_vec = self.results.lock().unwrap();
                result_vec.extend(results);
            } else {
                break;
            }
        }
    }
}

fn discover_dirs(wl: &Arc<Worklist>, dir_path: &Path) {
    if let Ok(entries) = fs::read_dir(dir_path) {
        for entry in entries {
            if let Ok(entry) = entry {
                let path = entry.path();
                if path.is_dir() {
                    discover_dirs(wl, &path);
                } else {
                    wl.add(Job::new(path));
                }
            }
        }
    } else {
        println!("Error reading directory");
    }
}

#[derive(StructOpt)]
struct Cli {
    #[structopt(help = "The search term")]
    search_term: String,
    #[structopt(help = "The directory to search in")]
    search_dir: String,
}

fn main() {
    let args = Cli::from_args();

    let search_term = args.search_term.clone();
    let search_dir = args.search_dir.clone();

    let worklist = Arc::new(Worklist::new());
    let results = Arc::new(Mutex::new(Vec::new()));

    let num_workers = 10;

    let worklist_clone = Arc::clone(&worklist);
    thread::spawn(move || {
        discover_dirs(&worklist_clone, Path::new(&search_dir));
        // Add sentinel jobs to signal the end
        for _ in 0..num_workers {
            worklist_clone.add(Job::new(PathBuf::new()));
        }
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
}
