use std::thread::spawn;

use super::ThreadPool;

/// Simple runner for concurrent jobs.
pub struct NaiveThreadPool {}

impl ThreadPool for NaiveThreadPool {
    /// Returns empty NaiveThreadPool.
    fn new(_threads: usize) -> crate::Result<Self>
    where
        Self: Sized,
    {
        Ok(NaiveThreadPool {})
    }

    /// Spawns single thread that quits right after executing job.
    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        // basically spawns thread for that job
        spawn(job);
    }
}
