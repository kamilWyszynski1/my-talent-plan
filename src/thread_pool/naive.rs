use std::thread::spawn;

use super::ThreadPool;

pub struct NaiveThreadPool {}

impl ThreadPool for NaiveThreadPool {
    fn new(_threads: usize) -> crate::Result<Self>
    where
        Self: Sized,
    {
        Ok(NaiveThreadPool {})
    }

    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        // basically spawns thread for that job
        spawn(job);
    }
}
