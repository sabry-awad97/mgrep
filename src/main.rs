use std::error::Error;
use std::fs;
use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;

use rayon::prelude::*;
use structopt::StructOpt;

use cli::Cli;
use error::SearchError;
use job::Job;
use worker::Worker;
use worklist::Worklist;

mod cli;
mod error;
mod job;
mod result;
mod worker;
mod worklist;

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
        let worker = Worker::new(search_term_ref.to_string(), worklist_clone, results_clone);
        worker.process_jobs();
    });

    // Print the search results by locking the results list and iterating over the results
    let results = results.lock().unwrap();
    for result in &*results {
        result.display()
    }

    // Return `Ok` to indicate successful execution of the `main` function
    Ok(())
}
