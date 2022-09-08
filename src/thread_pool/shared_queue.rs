use std::{
    sync::{
        mpsc::{channel, Receiver, Sender},
        Arc, Mutex,
    },
    thread::{self, spawn, JoinHandle},
};

use super::ThreadPool;

/// Message that will be sent during job executions in SharedQueueThreadPool.
enum ThreadPoolMessage {
    /// Sends job to be executed.
    RunJob(Box<dyn FnOnce() + Send + 'static>),

    /// Sends shutdown signal.
    Shutdown,
}

/// Thread runner for concurrent jobs. Uses jobs queue for job distribution to spawned threads.
/// It uses constant number of threads that will wait for specific job to run.
/// It handles shutdown of those threads and panicking jobs.
pub struct SharedQueueThreadPool {
    // will send proper message on some action.
    sender: Sender<ThreadPoolMessage>,
    //TODO: implement graceful shutdown.
    // handy for waiting for all threads to perform shutdown.
    // handles: Vec<JoinHandle<()>>,

    // needed for sending proper amount of `ThreadPoolMessage:Shutdown`.
    // threads: usize,
}

impl ThreadPool for SharedQueueThreadPool {
    /// Spawns `threads` number of new threads that will wait for job to execute.
    fn new(threads: usize) -> crate::Result<Self>
    where
        Self: Sized,
    {
        let (sender, receiver) = channel();
        let receiver = Arc::new(Mutex::new(receiver));

        for _ in 0..threads {
            let receiver = receiver.clone();
            let _handle = spawn(move || handle(TaskReceiver(receiver)));
        }

        Ok(Self { sender })
    }

    fn spawn<F>(&self, job: F)
    where
        F: FnOnce() + Send + 'static,
    {
        if let Err(e) = self.sender.send(ThreadPoolMessage::RunJob(Box::new(job))) {
            error!("could not send a job: {}", e);
        }
    }
}

#[derive(Clone)]
pub struct TaskReceiver(Arc<Mutex<Receiver<ThreadPoolMessage>>>);

/// Custom implementation of Drop to catch panicking jobs.
impl Drop for TaskReceiver {
    fn drop(&mut self) {
        if thread::panicking() {
            let receiver = self.clone();
            // spawn another thread that just panicked.
            spawn(|| handle(receiver));
        }
    }
}

fn handle(receiver: TaskReceiver) {
    loop {
        match receiver.0.lock().unwrap().recv() {
            Ok(msg) => match msg {
                ThreadPoolMessage::RunJob(job) => job(),
                ThreadPoolMessage::Shutdown => {
                    info!("shutting down");
                    return;
                }
            },
            Err(e) => {
                error!("channel probably closed, quitting: {:?}", e);
                return;
            }
        }
    }
}
