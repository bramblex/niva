use std::sync::mpsc::{Receiver, Sender};
use std::sync::{mpsc, Arc, Mutex};
use std::thread;

pub struct EvalQueue {
    sender: Arc<Mutex<Sender<String>>>,
    receiver: Arc<Mutex<Receiver<String>>>,
}

impl EvalQueue {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel::<String>();

        EvalQueue {
            sender: Arc::new(Mutex::new(sender)),
            receiver: Arc::new(Mutex::new(receiver)),
        }
    }

    pub fn send_script(&self, message: String) {
        self.sender.lock().unwrap().send(message).unwrap();
    }

    pub fn send_callback(&self, response: String) {
        self.send_script(format!("TauriLite.__resolve__({response})"))
    }

    pub fn run(&self, eval_fn: Box<dyn Fn(String) + Send>) {
        let receiver = self.receiver.clone();
        thread::spawn(move || {
            let receiver = receiver.lock().unwrap();
            while let Ok(script) = receiver.recv() {
                let mut scripts = vec![script];
                scripts.append(receiver.try_iter().collect::<Vec<String>>().as_mut());
                eval_fn(scripts.join(";").to_string());
            }
        });
    }
}
