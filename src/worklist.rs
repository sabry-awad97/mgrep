use crossbeam::channel::{unbounded, Receiver, Sender};

use crate::job::Job;

pub struct Worklist {
    sender: Sender<Option<Job>>,
    receiver: Receiver<Option<Job>>,
}

impl Worklist {
    pub fn new() -> Self {
        let (sender, receiver) = unbounded();
        Self { sender, receiver }
    }

    pub fn add(&self, job: Job) {
        self.sender.send(Some(job)).unwrap();
    }

    pub fn next(&self) -> Option<Job> {
        self.receiver.recv().unwrap()
    }

    /// Marks the end of jobs by adding a special empty jobs to the worklist.
    pub fn finalize(&self, num_workers: usize) {
        for _ in 0..num_workers {
            self.sender.send(None).unwrap();
        }
    }
}
