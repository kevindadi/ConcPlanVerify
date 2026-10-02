use std::sync::{Arc, Condvar, Mutex};
use std::thread;

// Counting semaphore with a RAII guard so the permit is released
// on every path (R4), and waiting never blocks the holder (R5).
struct Semaphore {
    permits: Mutex<usize>,
    available: Condvar,
}

impl Semaphore {
    fn new(initial: usize) -> Self {
        Semaphore {
            permits: Mutex::new(initial),
            available: Condvar::new(),
        }
    }

    // Blocks while no permit is free. While blocked, the mutex is
    // released, so the current holder can still release (R5).
    fn acquire(&self) -> Guard<'_> {
        let mut count = self.permits.lock().unwrap();
        while *count == 0 {
            count = self.available.wait(count).unwrap();
        }
        *count -= 1;
        Guard { sem: self }
    }
}

struct Guard<'a> {
    sem: &'a Semaphore,
}

// Dropping the guard releases exactly one permit, guaranteeing
// release happens exactly once per acquire on every path (R4).
impl Drop for Guard<'_> {
    fn drop(&mut self) {
        let mut count = self.sem.permits.lock().unwrap();
        *count += 1;
        drop(count);
        self.sem.available.notify_one();
    }
}

// Worker role: may acquire the permit more than once (R4), and
// holds it for the whole duration of each unit of work (R3).
fn worker(_name: &'static str, s: Arc<Semaphore>) {
    for _ in 0..2 {
        let _permit = s.acquire(); // held until end of this scope
        // Perform work while holding the permit.
    }
    // All guards have dropped here: releases == acquires.
}

// Supervising task: launches both workers and waits for both (R1).
fn supervisor(s: Arc<Semaphore>) {
    let s1 = Arc::clone(&s);
    let s2 = Arc::clone(&s);

    let h1 = thread::spawn(move || worker("w1", s1));
    let h2 = thread::spawn(move || worker("w2", s2));

    h1.join().unwrap();
    h2.join().unwrap();
}

fn main() {
    // One shared permit pool starting with exactly one permit (R2).
    let s = Arc::new(Semaphore::new(1));

    supervisor(s);

    // Every schedule terminates (R6): the semaphore never deadlocks,
    // guards always release, and the supervisor joins both workers.
    println!("DONE done=1");
}
