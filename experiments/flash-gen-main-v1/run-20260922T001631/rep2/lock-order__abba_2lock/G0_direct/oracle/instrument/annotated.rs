mod cir_trace;
use cir_trace::sync::{Mutex, Condvar};
use std::sync::{Arc};
use std::thread;

struct Shared {
    lock1: Mutex<()>,
    lock2: Mutex<()>,
    done1: Mutex<bool>,
    done2: Mutex<bool>,
    cv: Condvar,
}

fn worker(
    shared: Arc<Shared>,
    id: usize,
    done_flag: &'static str,
) {
    // Acquire both locks in a consistent global order to prevent deadlock (R5).
    // Both workers always take lock1 before lock2, so no circular wait is possible.
    let _g1 = shared.lock1.lock().unwrap();
    let _g2 = shared.lock2.lock().unwrap();

    // Critical work while holding both locks (R3).
    // Simulate the critical section deterministically.
    let mut acc: u64 = 0;
    for i in 0..1000u64 {
        acc = acc.wrapping_add(i ^ (id as u64));
    }
    std::hint::black_box(acc);

    // Release both locks before finishing (R7) by dropping guards.
    drop(_g2);
    drop(_g1);

    // Mark this worker as done and notify main.
    let mut d = shared.done1.lock().unwrap();
    if done_flag == "t1" {
        *d = true;
    } else {
        // use done2
        drop(d);
        let mut d2 = shared.done2.lock().unwrap();
        *d2 = true;
        shared.cv.notify_all();
        return;
    }
    shared.cv.notify_all();
}

fn main() { cir_trace::init();
    let shared = Arc::new(Shared {
        lock1: Mutex::new_named("shared_mutex0", ()),
        lock2: Mutex::new_named("shared_mutex1", ()),
        done1: Mutex::new_named("shared_mutex2", false),
        done2: Mutex::new_named("shared_mutex3", false),
        cv: Condvar::new_named("shared_condvar0"),
    });

    let s1 = Arc::clone(&shared);
    let s2 = Arc::clone(&shared);

    let t1 = cir_trace::spawn("t1", move || worker(s1, 1, "t1"));
    let t2 = cir_trace::spawn("t2", move || worker(s2, 2, "t2"));

    // Main waits until both workers have finished (R6).
    {
        let mut d1 = shared.done1.lock().unwrap();
        while !*d1 {
            d1 = shared.cv.wait(d1).unwrap();
        }
    }
    {
        let mut d2 = shared.done2.lock().unwrap();
        while !*d2 {
            d2 = shared.cv.wait(d2).unwrap();
        }
    }

    t1.join().unwrap();
    t2.join().unwrap();

    println!("DONE t1=1 t2=1");
 cir_trace::finish();}
