use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread;

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

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        println!("Usage: {} <search_term> <search_dir>", args[0]);
        return;
    }
}
