mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Semaphore {
    held: Mutex<bool>,
    available: Condvar,
}

impl Semaphore {
    fn new() -> Self {
        Self {
            held: Mutex::new(false),
            available: Condvar::new(),
        }
    }

    fn acquire(&self) -> Permit<'_> {
        let mut held = self.held.lock().unwrap();
        while *held {
            held = self.available.wait(held).unwrap();
        }
        *held = true;
        Permit { s: self }
    }
}

struct Permit<'a> {
    s: &'a Semaphore,
}

impl Drop for Permit<'_> {
    fn drop(&mut self) {
        let mut held = self.s.held.lock().unwrap();
        *held = false;
        self.s.available.notify_one();
    }
}

fn activation(s: Arc<Semaphore>) {
    let _permit = s.acquire();
    thread::yield_now();
}

fn role(s: Arc<Semaphore>) {
    let activations: Vec<_> = (0..2)
        .map(|_| {
            let s = Arc::clone(&s);
            cir_trace::spawn("activation#953", move || activation(s))
        })
        .collect();

    for activation in activations {
        activation.join().unwrap();
    }
}

fn w1(s: Arc<Semaphore>) {
    role(s);
}

fn w2(s: Arc<Semaphore>) {
    role(s);
}

fn w3(s: Arc<Semaphore>) {
    role(s);
}

fn main() { cir_trace::init();
    let s = Arc::new(Semaphore::new());

    let workers = [
        {
            let s = Arc::clone(&s);
            cir_trace::spawn("w1#1363", move || w1(s))
        },
        {
            let s = Arc::clone(&s);
            cir_trace::spawn("w2#1461", move || w2(s))
        },
        {
            let s = Arc::clone(&s);
            cir_trace::spawn("w3#1559", move || w3(s))
        },
    ];

    for worker in workers {
        worker.join().unwrap();
    }

    println!("DONE done=1");
 cir_trace::finish();}
