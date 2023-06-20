use async_recursion::async_recursion;
use cli::Cli;
use crossbeam::channel::{unbounded, TryRecvError};
use error::SearchError;
use job::Job;
use std::error::Error;
use std::path::Path;
use std::sync::Arc;
use structopt::StructOpt;
use tokio::fs;
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
    let mut entries = fs::read_dir(dir_path)
        .await
        .map_err(|_| SearchError::InvalidDir(dir_path.display().to_string()))?;

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
    let args = Cli::from_args();
    let search_term = args.search_term.clone();

    let num_workers = num_cpus::get() - 1;

    let worklist = Arc::new(Worklist::new());

    let (result_sender, result_receiver) = unbounded();

    let worklist_clone = Arc::clone(&worklist);
    tokio::spawn(async move {
        if let Err(error) = discover_dirs(&worklist_clone, &args.search_dir).await {
            eprintln!("{}", error);
            if let Some(source) = error.source() {
                eprintln!("Caused by: {}", source);
            }
        }
        worklist_clone.finalize(num_workers);
    });

    let mut worker_handles = Vec::new();
    for _ in 0..num_workers {
        let worklist_clone = Arc::clone(&worklist);
        let result_sender_clone = result_sender.clone();
        let search_term_clone = search_term.clone();
        let handle = tokio::spawn(async move {
            let worker = Worker::new(search_term_clone, worklist_clone, result_sender_clone);
            worker.process_jobs().await;
        });
        worker_handles.push(handle);
    }

    for handle in worker_handles {
        handle.await?;
    }

    let mut results = Vec::new();

    loop {
        match result_receiver.try_recv() {
            Ok(result_batch) => {
                results.extend(result_batch);
            }
            Err(TryRecvError::Empty) => {
                // println!("No more results available.");
                break;
            }
            Err(TryRecvError::Disconnected) => {
                // println!("The result channel has been closed.");
                break;
            }
        }
    }

    for result in results {
        result.display();
    }

    Ok(())
}
