mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
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

    // Wait until b is free, then take it.
    {
        let mut b = locks.b.lock().unwrap();
        while *b {
            b = locks.cv.wait(b).unwrap();
        }
        *b = true;
    }

    // Critical work: both locks held simultaneously (R3).
    // Simulate the critical section.
    {
        let _a = locks.a.lock().unwrap();
        let _b = locks.b.lock().unwrap();
        // Both locks are held here.
    }

    // Release b.
    {
        let mut b = locks.b.lock().unwrap();
        *b = false;
        locks.cv.notify_all();
    }

    // Release a.
    {
        let mut a = locks.a.lock().unwrap();
        *a = false;
        locks.cv.notify_all();
    }

    // Record completion.
    {
        let mut d = done.lock().unwrap();
        d.push(name.to_string());
    }
}

fn main() { cir_trace::init();
    let locks = Arc::new(Locks {
        a: Mutex::new_named("locks_mutex0", false),
        b: Mutex::new_named("locks_mutex1", false),
        cv: Condvar::new_named("locks_condvar0"),
    });

    let done = Arc::new(Mutex::new_named("done_mutex0", Vec::new()));

    let locks1 = Arc::clone(&locks);
    let done1 = Arc::clone(&done);
    let t1 = cir_trace::spawn("t1", move || worker("t1", locks1, done1));

    let locks2 = Arc::clone(&locks);
    let done2 = Arc::clone(&done);
    let t2 = cir_trace::spawn("t2", move || worker("t2", locks2, done2));

    t1.join().unwrap();
    t2.join().unwrap();

    let d = done.lock().unwrap();
    let t1_done = d.iter().filter(|s| *s == "t1").count();
    let t2_done = d.iter().filter(|s| *s == "t2").count();

    println!("DONE t1={} t2={}", t1_done, t2_done);
 cir_trace::finish();}
