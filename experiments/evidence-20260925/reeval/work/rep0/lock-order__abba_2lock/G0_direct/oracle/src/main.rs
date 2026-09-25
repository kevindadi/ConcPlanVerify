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
    // Wait until each lock is free, then take it (R4).
    {
        let mut a_guard = locks.a.lock().unwrap();
        while *a_guard {
            a_guard = locks.cv.wait(a_guard).unwrap();
        }
        *a_guard = true;
    }

    {
        let mut b_guard = locks.b.lock().unwrap();
        while *b_guard {
            b_guard = locks.cv.wait(b_guard).unwrap();
        }
        *b_guard = true;
    }

    // Critical work while holding both locks (R3).
    // Simulate the critical section by recording completion.
    {
        let mut d = done.lock().unwrap();
        d.push(name.to_string());
    }

    // Release both locks before finishing (R7).
    {
        let mut b_guard = locks.b.lock().unwrap();
        *b_guard = false;
        locks.cv.notify_all();
    }
    {
        let mut a_guard = locks.a.lock().unwrap();
        *a_guard = false;
        locks.cv.notify_all();
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
    let t1 = cir_trace::spawn("worker", move || worker("t1", locks1, done1));

    let locks2 = Arc::clone(&locks);
    let done2 = Arc::clone(&done);
    let t2 = cir_trace::spawn("worker", move || worker("t2", locks2, done2));

    t1.join().unwrap();
    t2.join().unwrap();

    let d = done.lock().unwrap();
    let t1_done = d.iter().filter(|s| s.as_str() == "t1").count();
    let t2_done = d.iter().filter(|s| s.as_str() == "t2").count();

    println!("DONE t1={} t2={}", t1_done, t2_done);
 cir_trace::finish();}
