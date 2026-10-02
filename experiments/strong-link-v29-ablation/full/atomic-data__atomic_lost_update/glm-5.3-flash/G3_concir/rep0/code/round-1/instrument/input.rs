use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::Arc;
use std::thread;

fn w1(c: &AtomicI32) {
    let mut observed: i32 = c.load(Ordering::SeqCst);
    let mut exp: i32;
    loop {
        // s1: exp = observed
        exp = observed;
        // s2: observed = CAS(c, expected=exp, desired=exp+1)
        observed = match c.compare_exchange(exp, exp + 1, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_prev) => exp,
            Err(actual) => actual,
        };
        // s3: if observed == exp then return else retry
        if observed == exp {
            break;
        }
    }
    // s4: return
}

fn w2(c: &AtomicI32) {
    let mut observed: i32 = c.load(Ordering::SeqCst);
    let mut exp: i32;
    loop {
        // s1: exp = observed
        exp = observed;
        // s2: observed = CAS(c, expected=exp, desired=exp+1)
        observed = match c.compare_exchange(exp, exp + 1, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_prev) => exp,
            Err(actual) => actual,
        };
        // s3: if observed == exp then return else retry
        if observed == exp {
            break;
        }
    }
    // s4: return
}

fn main() {
    // Shared resource: c (atomic counter, init 0)
    let c = Arc::new(AtomicI32::new(0));

    let c_w1 = Arc::clone(&c);
    let handle_w1 = thread::spawn(move || {
        w1(&c_w1);
    });

    let c_w2 = Arc::clone(&c);
    let handle_w2 = thread::spawn(move || {
        w2(&c_w2);
    });

    // Wait for both workers to finish
    handle_w1.join().expect("w1 panicked");
    handle_w2.join().expect("w2 panicked");

    println!("DONE done=1");
}
