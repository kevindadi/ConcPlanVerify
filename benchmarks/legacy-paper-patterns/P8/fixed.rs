//! Reference Rust for tests/e2e/cas_race/buggy.json.
//! Gold verdict: SAFE (negative control). Two threads race on an atomic CAS;
//! exactly one wins, both terminate on either branch. There is no deadlock
//! and no data race — baselines must NOT report a defect here.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

fn main() {
    let flag = Arc::new(AtomicBool::new(false));

    let f1 = Arc::clone(&flag);
    let setter1 = thread::spawn(move || {
        // s1: cas(false, true), branch on the observed value
        match f1.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => { /* s2: won the race */ }
            Err(_) => { /* s3: lost the race */ }
        }
    });

    let f2 = Arc::clone(&flag);
    let setter2 = thread::spawn(move || {
        // s1: cas(false, true), branch on the observed value
        match f2.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst) {
            Ok(_) => { /* s2: won the race */ }
            Err(_) => { /* s3: lost the race */ }
        }
    });

    setter1.join().unwrap();
    setter2.join().unwrap();
}
