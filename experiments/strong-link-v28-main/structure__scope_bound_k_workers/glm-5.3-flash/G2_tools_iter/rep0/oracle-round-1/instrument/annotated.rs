mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;
use std::time::Duration;

// Shared resource: s (a counting semaphore initialized with a single permit).
struct Semaphore {
    permits: Mutex<usize>,
    available: Condvar,
}

impl Semaphore {
    fn new(permits: usize) -> Self {
        Semaphore {
            permits: Mutex::new(permits),
            available: Condvar::new(),
        }
    }

    // R4: an activation that cannot obtain the permit waits until it is available.
    fn acquire(&self) {
        let mut permits = self.permits.lock().unwrap();
        while *permits == 0 {
            permits = self.available.wait(permits).unwrap();
        }
        *permits -= 1;
    }

    // R2: the activation releases the permit after its work.
    fn release(&self) {
        let mut permits = self.permits.lock().unwrap();
        *permits += 1;
        self.available.notify_one();
    }
}

// One activation of a worker role: holds the single permit while working (R2, R3),
// waits for the permit if another activation holds it (R4).
fn activation(name: &'static str, id: usize, s: &Semaphore) {
    s.acquire();
    // Critical section: at most one activation holds the permit here (R3).
    thread::sleep(Duration::from_millis(20));
    s.release();
    let _ = (name, id); // role identity kept for traceability
}

// Worker roles: w1, w2, w3. Each role may have up to two activations at once (R1),
// so each role starts two activations and waits for both to finish.
fn w1(s: Arc<Semaphore>) {
    let mut handles = Vec::new();
    for id in 0..2 {
        let s = Arc::clone(&s);
        handles.push(thread::spawn(move || activation("w1", id, &s)));
    }
    for h in handles {
        h.join().unwrap();
    }
}

fn w2(s: Arc<Semaphore>) {
    let mut handles = Vec::new();
    for id in 0..2 {
        let s = Arc::clone(&s);
        handles.push(thread::spawn(move || activation("w2", id, &s)));
    }
    for h in handles {
        h.join().unwrap();
    }
}

fn w3(s: Arc<Semaphore>) {
    let mut handles = Vec::new();
    for id in 0..2 {
        let s = Arc::clone(&s);
        handles.push(thread::spawn(move || activation("w3", id, &s)));
    }
    for h in handles {
        h.join().unwrap();
    }
}

fn main() { cir_trace::init();
    // Single shared permit (R1).
    let s = Arc::new(Semaphore::new(1));

    // Main task starts the three worker roles (R1).
    let mut roles = Vec::new();
    for role in [w1, w2, w3] {
        let s = Arc::clone(&s);
        roles.push(thread::spawn(move || role(s)));
    }

    // Wait for all roles to complete (R5, R6): joining guarantees termination
    // under every interleaving, since every activation eventually acquires
    // and releases the permit.
    for r in roles {
        r.join().unwrap();
    }

    // All three worker roles completed; print the required line and exit (R7).
    println!("DONE done=1");
 cir_trace::finish();}
