mod naive;
mod shared_queue;

use crate::Result;

pub use naive::NaiveThreadPool;
pub use shared_queue::SharedQueueThreadPool;

pub trait ThreadPool {
    fn new(threads: usize) -> Result<Self>
    where
        Self: Sized;

    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static;
}
