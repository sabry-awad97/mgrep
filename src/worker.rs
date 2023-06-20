use crossbeam::channel::Sender;
use tokio::fs;
use tokio::io::{AsyncBufReadExt, BufReader};

use crate::error::SearchError;
use crate::result::SearchResult;
use crate::worklist::Worklist;
use std::path::PathBuf;
use std::sync::Arc;

pub struct Worker {
    search_term: String,
    worklist: Arc<Worklist>,
    result_sender: Sender<Vec<SearchResult>>,
}

impl Worker {
    pub fn new(
        search_term: String,
        worklist: Arc<Worklist>,
        result_sender: Sender<Vec<SearchResult>>,
    ) -> Self {
        Self {
            search_term,
            worklist,
            result_sender,
        }
    }

    async fn find_in_file(&self, path: PathBuf) -> Result<Vec<SearchResult>, SearchError> {
        if !path.exists() {
            return Ok(Vec::new());
        }

        let file = fs::File::open(&path).await?;
        let reader = BufReader::with_capacity(8192, file);
        let mut lines = reader.lines();
        let mut matching_lines = Vec::new();

        let mut line_number = 0;
        while let Some(line) = lines.next_line().await? {
            if line.contains(&self.search_term) {
                matching_lines.push(SearchResult::new(path.clone(), line_number, line));
            }

            line_number += 1;
        }

        Ok(matching_lines)
    }

    pub async fn process_jobs(&self) {
        loop {
            let job = self.worklist.next();
            if let Some(job) = job {
                match self.find_in_file(job.into_inner()).await {
                    Ok(results) => {
                        if let Err(send_error) = self.result_sender.send(results) {
                            eprintln!("Error sending results: {}", send_error);
                            break;
                        }
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
