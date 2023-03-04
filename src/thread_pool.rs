use std::sync::mpsc::Receiver;
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

fn create_worker(receiver: Arc<Mutex<Receiver<Task>>>) -> thread::JoinHandle<()> {
    std::thread::spawn(move || {
        loop {
            let job = receiver.lock().unwrap().recv();
            // unlock receiver before job start
            if let Ok(job) = job {
                job();
            }
        }
    })
}

type Task = Box<dyn FnOnce() + Send + 'static>;

#[derive(Debug)]
pub struct ThreadPool {
    sender: mpsc::Sender<Task>,
}

impl ThreadPool {
    pub fn new(size: usize) -> ThreadPool {
        assert!(size > 0);

        let (sender, receiver) = mpsc::channel();

        let mut workers = Vec::with_capacity(size);

        let receiver = Arc::new(Mutex::new(receiver));
        for _ in 0..size {
            workers.push(create_worker(receiver.clone()));
        }

        ThreadPool { sender }
    }

    pub fn run<F>(&self, f: F)
    where
        F: FnOnce() + Send + 'static,
    {
        let job = Box::new(f);
        self.sender.send(job).unwrap();
    }
}
