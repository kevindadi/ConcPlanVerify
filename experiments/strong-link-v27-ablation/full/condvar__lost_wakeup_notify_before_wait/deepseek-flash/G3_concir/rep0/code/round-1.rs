use std::sync::{Arc, Condvar, Mutex};
use std::thread;

struct Ready {
    ready: bool,
}

struct Shared {
    m: Mutex<Ready>,
    cv: Condvar,
}

fn waiter(shared: Arc<Shared>) {
    let mut guard = shared.m.lock().unwrap();
    while !guard.ready {
        guard = shared.cv.wait(guard).unwrap();
    }
}

fn notifier(shared: Arc<Shared>) {
    let mut guard = shared.m.lock().unwrap();
    guard.ready = true;
    shared.cv.notify_one();
}

fn main() {
    let shared = Arc::new(Shared {
        m: Mutex::new(Ready { ready: false }),
        cv: Condvar::new(),
    });

    let w = {
        let shared = Arc::clone(&shared);
        thread::spawn(move || waiter(shared))
    };

    let n = {
        let shared = Arc::clone(&shared);
        thread::spawn(move || notifier(shared))
    };

    w.join().unwrap();
    n.join().unwrap();

    println!("DONE ready=true");
}
