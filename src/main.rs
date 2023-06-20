use std::error::Error;
use std::path::Path;
use std::sync::Arc;
use tokio::fs;

use structopt::StructOpt;

use async_recursion::async_recursion;
use cli::Cli;
use error::SearchError;
use job::Job;
use tokio::sync::Mutex;
use worker::Worker;
use worklist::Worklist;

mod cli;
mod error;
mod job;
mod result;
mod worker;
mod worklist;

#[async_recursion]
async fn discover_dirs(wl: &Arc<Worklist>, dir_path: &Path) -> Result<(), SearchError> {
    let mut entries = fs::read_dir(dir_path).await.map_err(|err| {
        SearchError::InvalidDir(format!(
            "Failed to read directory '{}': {}",
            dir_path.display(),
            err
        ))
    })?;

    let mut tasks = Vec::new();
    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_dir() {
            let task = async {
                let path = Arc::new(path);
                let wl_clone = Arc::clone(wl);
                let path_clone = Arc::new(path.clone());
                discover_dirs(&wl_clone, &path_clone).await?;
                Ok::<(), SearchError>(())
            };
            tasks.push(task);
        } else {
            wl.add(Job::new(path));
        }
    }
    for task in tasks {
        task.await?;
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Parse command-line arguments using `Cli::from_args()`
    let args = Cli::from_args();
    let search_term = args.search_term.clone();

    // Determine the number of worker threads
    let num_workers = num_cpus::get() - 1;

    // Create a shared worklist using `Arc`
    let worklist = Arc::new(Worklist::new());

    // Create a shared results list using `Arc` and `Mutex`
    let results = Arc::new(Mutex::new(Vec::new()));

    // Spawn a separate thread to discover directories and add jobs to the worklist
    let worklist_clone = Arc::clone(&worklist);
    tokio::spawn(async move {
        if let Err(error) = discover_dirs(&worklist_clone, &args.search_dir).await {
            eprintln!("Error discovering directories: {}", error);
            if let Some(source) = error.source() {
                eprintln!("Caused by: {}", source);
            }
        }
        worklist_clone.finalize(num_workers);
    });

    let mut worker_handles = Vec::new();
    for _ in 0..num_workers {
        let worklist_clone = Arc::clone(&worklist);
        let results_clone = Arc::clone(&results);
        let search_term_clone = search_term.clone();
        let handle = tokio::spawn(async move {
            let worker = Worker::new(search_term_clone, worklist_clone, results_clone);
            worker.process_jobs().await;
        });
        worker_handles.push(handle);
    }

    for handle in worker_handles {
        handle.await?;
    }

    let results = results.lock().await;
    for result in &*results {
        result.display()
    }
    Ok(())
}
