use std::sync::{Arc, Mutex, Condvar};
use std::thread;

struct Locks {
    a: Mutex<bool>,
    b: Mutex<bool>,
    cv: Condvar,
}

fn worker(name: &str, locks: Arc<Locks>, done: Arc<Mutex<Vec<String>>>) {
    // Acquire both locks in a consistent global order to avoid deadlock (R5).
    // Wait until a is free, then take it.
    {
        let mut a = locks.a.lock().unwrap();
        while *a {
            a = locks.cv.wait(a).unwrap();
        }
        *a = true;
    }

    // Now acquire b, waiting if busy.
    {
        let mut b = locks.b.lock().unwrap();
        while *b {
            b = locks.cv.wait(b).unwrap();
        }
        *b = true;
    }

    // Critical work: both locks held simultaneously (R3).
    // Simulate the critical section.
    let mut guard = locks.a.lock().unwrap();
    // We already hold a and b; just do work.
    drop(guard);

    // Release b (R7).
    {
        let mut b = locks.b.lock().unwrap();
        *b = false;
        locks.cv.notify_all();
    }

    // Release a (R7).
    {
        let mut a = locks.a.lock().unwrap();
        *a = false;
        locks.cv.notify_all();
    }

    // Record completion.
    let mut d = done.lock().unwrap();
    d.push(name.to_string());
}

fn main() {
    let locks = Arc::new(Locks {
        a: Mutex::new(false),
        b: Mutex::new(false),
        cv: Condvar::new(),
    });

    let done = Arc::new(Mutex::new(Vec::new()));

    let locks1 = Arc::clone(&locks);
    let done1 = Arc::clone(&done);
    let t1 = thread::spawn(move || worker("t1", locks1, done1));

    let locks2 = Arc::clone(&locks);
    let done2 = Arc::clone(&done);
    let t2 = thread::spawn(move || worker("t2", locks2, done2));

    t1.join().unwrap();
    t2.join().unwrap();

    let d = done.lock().unwrap();
    let t1_done = d.iter().filter(|s| *s == "t1").count();
    let t2_done = d.iter().filter(|s| *s == "t2").count();

    println!("DONE t1={} t2={}", t1_done, t2_done);
}
