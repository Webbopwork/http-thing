use std::{
    sync::{mpsc, Arc, Mutex},
    thread,
};

pub struct ThreadPool {
    workers: Vec<Worker>,
    sender: Option<mpsc::Sender<Job>>,
}

type Job = Box<dyn FnOnce() + Send + 'static>;

impl ThreadPool {
    pub fn new(size: usize) -> Self {
        assert!(size > 0, "Cannot initialize a thread pool with 0 threads");

        let (sender, receiver) = mpsc::channel();
        let arc_receiver = Arc::new(Mutex::new(receiver));
        let mut workers = Vec::with_capacity(size);

        for id in 0..size {
            workers.push(Worker::new(id, Arc::clone(&arc_receiver)));
        }

        Self {
            workers,
            sender: Some(sender),
        }
    }

    #[expect(clippy::unwrap_used, reason = "Should never panic here")]
    pub fn execute<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender.as_ref().unwrap().send(job).unwrap();
    }
}

impl Drop for ThreadPool {
    #[expect(
        clippy::unwrap_used,
        reason = "We don't really care if the process panics here"
    )]
    fn drop(&mut self) {
        drop(self.sender.take());

        for worker in &mut self.workers {
            log::trace!("Shutting down worker {}", worker.id);

            if let Some(thread) = worker.thread.take() {
                thread.join().unwrap();
            }
        }
    }
}

struct Worker {
    id: usize,
    thread: Option<thread::JoinHandle<()>>,
}

impl Worker {
    #[expect(
        clippy::unwrap_used,
        reason = "Will only throw an error if a thread panicked"
    )]
    fn new(id: usize, receiver: Arc<Mutex<mpsc::Receiver<Job>>>) -> Self {
        let thread = thread::spawn(move || loop {
            let message = receiver.lock().unwrap().recv();

            if let Ok(job) = message {
                log::trace!("Worker {id} got a job; executing.");
                job();
            } else {
                log::trace!("Worker {id} disconnected; shutting down.");
                break;
            }
        });

        Self {
            id,
            thread: Some(thread),
        }
    }
}
