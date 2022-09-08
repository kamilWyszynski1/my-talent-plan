mod naive;

use crate::Result;

pub use naive::NaiveThreadPool;

pub trait ThreadPool: Send {
    fn new(threads: usize) -> Result<Self>
    where
        Self: Sized;

    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static;
}
