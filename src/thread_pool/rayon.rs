use super::ThreadPool;

/// Abstraction on rayon crate.
pub struct RayonThreadPool {
    thread_pool: rayon::ThreadPool,
}

impl ThreadPool for RayonThreadPool {
    fn new(threads: usize) -> crate::Result<Self>
    where
        Self: Sized,
    {
        let thread_pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build()?;

        Ok(Self { thread_pool })
    }

    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        self.thread_pool.spawn(job)
    }
}
